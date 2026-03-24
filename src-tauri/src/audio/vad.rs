use ndarray::Array3;
use ort::session::Session;
use ort::value::Tensor;
use std::path::Path;

/// Audio chunk that contains detected speech.
#[derive(Debug, Clone)]
pub struct SpeechSegment {
    pub samples: Vec<f32>,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum VadState {
    Silence,
    Speech,
}

const WINDOW_SIZE: usize = 512;
const SAMPLE_RATE: u64 = 16000;
const MS_PER_WINDOW: u32 = (WINDOW_SIZE as u64 * 1000 / SAMPLE_RATE) as u32;

// ── Public interface ────────────────────────────────────────────────

pub enum VadProcessor {
    Energy(EnergyVad),
    Silero(SileroVad),
}

impl VadProcessor {
    /// Create energy-based VAD (no model needed).
    pub fn new_energy() -> Self {
        Self::Energy(EnergyVad::new())
    }

    /// Create Silero ONNX VAD.
    pub fn new_silero(model_path: &Path) -> Result<Self, String> {
        Ok(Self::Silero(SileroVad::new(model_path)?))
    }

    pub fn process_chunk(&mut self, samples: &[f32]) -> Vec<SpeechSegment> {
        match self {
            Self::Energy(v) => v.process_chunk(samples),
            Self::Silero(v) => v.process_chunk(samples),
        }
    }

    pub fn flush(&mut self) -> Option<SpeechSegment> {
        match self {
            Self::Energy(v) => v.flush(),
            Self::Silero(v) => v.flush(),
        }
    }
}

// ── Shared state machine ────────────────────────────────────────────

struct VadStateMachine {
    vad_state: VadState,
    speech_buffer: Vec<f32>,
    silence_counter_ms: u32,
    speech_counter_ms: u32,
    current_start_ms: u64,
    total_samples_processed: u64,
    min_speech_ms: u32,
    min_silence_ms: u32,
    inference_count: u64,
}

impl VadStateMachine {
    fn new(min_speech_ms: u32, min_silence_ms: u32) -> Self {
        Self {
            vad_state: VadState::Silence,
            speech_buffer: Vec::new(),
            silence_counter_ms: 0,
            speech_counter_ms: 0,
            current_start_ms: 0,
            total_samples_processed: 0,
            min_speech_ms,
            min_silence_ms,
            inference_count: 0,
        }
    }

    fn current_time_ms(&self) -> u64 {
        self.total_samples_processed * 1000 / SAMPLE_RATE
    }

    fn process_window(&mut self, window: &[f32], is_speech: bool) -> Option<SpeechSegment> {
        self.inference_count += 1;

        match self.vad_state {
            VadState::Silence => {
                if is_speech {
                    self.speech_counter_ms += MS_PER_WINDOW;
                    self.speech_buffer.extend_from_slice(window);

                    if self.speech_counter_ms >= self.min_speech_ms {
                        self.vad_state = VadState::Speech;
                        self.current_start_ms = self
                            .current_time_ms()
                            .saturating_sub(self.speech_counter_ms as u64);
                        self.silence_counter_ms = 0;
                        println!("[VAD] -> SPEECH at {}ms", self.current_start_ms);
                    }
                } else {
                    self.speech_counter_ms = 0;
                    self.speech_buffer.clear();
                }
                None
            }
            VadState::Speech => {
                self.speech_buffer.extend_from_slice(window);

                if !is_speech {
                    self.silence_counter_ms += MS_PER_WINDOW;

                    if self.silence_counter_ms >= self.min_silence_ms {
                        let segment = SpeechSegment {
                            samples: std::mem::take(&mut self.speech_buffer),
                            start_ms: self.current_start_ms,
                            end_ms: self.current_time_ms(),
                        };
                        self.vad_state = VadState::Silence;
                        self.speech_counter_ms = 0;
                        self.silence_counter_ms = 0;
                        println!(
                            "[VAD] -> SILENCE, segment {}ms-{}ms ({} samples)",
                            segment.start_ms, segment.end_ms, segment.samples.len()
                        );
                        return Some(segment);
                    }
                } else {
                    self.silence_counter_ms = 0;
                }
                None
            }
        }
    }

    fn flush(&mut self) -> Option<SpeechSegment> {
        if self.vad_state == VadState::Speech && !self.speech_buffer.is_empty() {
            let segment = SpeechSegment {
                samples: std::mem::take(&mut self.speech_buffer),
                start_ms: self.current_start_ms,
                end_ms: self.current_time_ms(),
            };
            self.vad_state = VadState::Silence;
            self.speech_counter_ms = 0;
            self.silence_counter_ms = 0;
            Some(segment)
        } else {
            None
        }
    }

    fn process_chunks(
        &mut self,
        samples: &[f32],
        detect_fn: &mut dyn FnMut(&[f32]) -> bool,
    ) -> Vec<SpeechSegment> {
        let mut segments = Vec::new();
        for chunk in samples.chunks(WINDOW_SIZE) {
            let window = if chunk.len() < WINDOW_SIZE {
                let mut padded = vec![0.0f32; WINDOW_SIZE];
                padded[..chunk.len()].copy_from_slice(chunk);
                padded
            } else {
                chunk.to_vec()
            };
            let is_speech = detect_fn(&window);
            if let Some(seg) = self.process_window(&window, is_speech) {
                segments.push(seg);
            }
            self.total_samples_processed += chunk.len() as u64;
        }
        segments
    }
}

// ── Energy-based VAD ────────────────────────────────────────────────

pub struct EnergyVad {
    sm: VadStateMachine,
    threshold: f32,
}

impl EnergyVad {
    fn new() -> Self {
        Self {
            sm: VadStateMachine::new(300, 500),
            threshold: 0.03, // Raised to avoid noise triggering Whisper hallucinations
        }
    }

    fn process_chunk(&mut self, samples: &[f32]) -> Vec<SpeechSegment> {
        let threshold = self.threshold;
        self.sm.process_chunks(samples, &mut |window| {
            let rms = (window.iter().map(|s| s * s).sum::<f32>() / window.len() as f32).sqrt();
            rms > threshold
        })
    }

    fn flush(&mut self) -> Option<SpeechSegment> {
        self.sm.flush()
    }
}

// ── Silero ONNX VAD (v5) ────────────────────────────────────────────
//
// Silero VAD v5 requires a context window prepended to the input:
//   actual_input = [context(64 samples) | audio(512 samples)] → shape [1, 576]
// After inference, context is updated to the last 64 samples of the concatenated input.

const STATE_DIM: usize = 128;
const CONTEXT_SIZE: usize = 64; // 4ms at 16kHz

pub struct SileroVad {
    sm: VadStateMachine,
    session: Session,
    state: Array3<f32>,
    context: Vec<f32>,
    threshold: f32,
}

impl SileroVad {
    fn new(model_path: &Path) -> Result<Self, String> {
        let session = Session::builder()
            .map_err(|e| format!("Failed to create session builder: {}", e))?
            .commit_from_file(model_path)
            .map_err(|e| format!("Failed to load VAD model: {}", e))?;

        Ok(Self {
            sm: VadStateMachine::new(250, 300),
            session,
            state: Array3::<f32>::zeros((2, 1, STATE_DIM)),
            context: vec![0.0f32; CONTEXT_SIZE],
            threshold: 0.5,
        })
    }

    fn run_inference(&mut self, window: &[f32]) -> f32 {
        // 1. Prepend context to audio: [context(64) | audio(512)] = 576 samples
        let mut input_data = Vec::with_capacity(CONTEXT_SIZE + window.len());
        input_data.extend_from_slice(&self.context);
        input_data.extend_from_slice(window);
        let total_len = input_data.len() as i64;

        // 2. Update context for next call (last 64 samples)
        self.context = input_data[input_data.len() - CONTEXT_SIZE..].to_vec();

        // 3. Build tensors
        let input_tensor = Tensor::from_array(([1i64, total_len], input_data));
        let state_tensor = Tensor::from_array(self.state.clone());
        // sr must be scalar (shape [])
        let sr_tensor = Tensor::from_array(ndarray::arr0(SAMPLE_RATE as i64));

        let (input_tensor, state_tensor, sr_tensor) =
            match (input_tensor, state_tensor, sr_tensor) {
                (Ok(i), Ok(s), Ok(sr)) => (i, s, sr),
                _ => return 0.0,
            };

        // 4. Run inference
        let outputs = match self.session.run(ort::inputs![
            "input" => input_tensor,
            "state" => state_tensor,
            "sr" => sr_tensor,
        ]) {
            Ok(o) => o,
            Err(e) => {
                eprintln!("[SileroVAD] inference error: {}", e);
                return 0.0;
            }
        };

        // 5. Extract probability
        let prob = outputs[0usize]
            .try_extract_tensor::<f32>()
            .ok()
            .and_then(|(_, data)| data.first().copied())
            .unwrap_or(0.0);

        // 6. Update state
        if let Ok(arr) = outputs[1usize].try_extract_array::<f32>() {
            if let Ok(arr3) = arr.to_owned().into_dimensionality::<ndarray::Ix3>() {
                self.state = arr3;
            }
        }

        prob
    }

    fn process_chunk(&mut self, samples: &[f32]) -> Vec<SpeechSegment> {
        let threshold = self.threshold;
        let mut probs = Vec::new();
        for chunk in samples.chunks(WINDOW_SIZE) {
            let window = if chunk.len() < WINDOW_SIZE {
                let mut padded = vec![0.0f32; WINDOW_SIZE];
                padded[..chunk.len()].copy_from_slice(chunk);
                padded
            } else {
                chunk.to_vec()
            };
            let prob = self.run_inference(&window);
            probs.push((window, prob));
        }

        let mut segments = Vec::new();
        for (window, prob) in probs {
            let is_speech = prob >= threshold;
            if let Some(seg) = self.sm.process_window(&window, is_speech) {
                segments.push(seg);
            }
            self.sm.total_samples_processed += window.len() as u64;
        }
        segments
    }

    fn flush(&mut self) -> Option<SpeechSegment> {
        self.sm.flush()
    }
}
