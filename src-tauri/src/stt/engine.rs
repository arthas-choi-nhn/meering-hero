use serde::Serialize;
use std::path::Path;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use super::postprocess::PostProcessor;

#[derive(Debug, Clone, Serialize)]
pub struct TranscribeResult {
    pub text: String,
    pub segments: Vec<TextSegment>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TextSegment {
    pub text: String,
    pub start_ms: i64,
    pub end_ms: i64,
}

/// Minimum audio length to attempt transcription (0.3 seconds at 16kHz)
const MIN_AUDIO_SAMPLES: usize = 4800;
/// Skip segments where Whisper thinks there's no speech
const NO_SPEECH_PROB_THRESHOLD: f32 = 0.4;

pub struct SttEngine {
    ctx: WhisperContext,
    language: String,
    initial_prompt: Option<String>,
    post_processor: PostProcessor,
}

impl SttEngine {
    pub fn new(
        model_path: &Path,
        language: &str,
        initial_prompt: Option<String>,
    ) -> Result<Self, String> {
        let mut params = WhisperContextParameters::default();
        params.use_gpu(true);

        let ctx = WhisperContext::new_with_params(
            model_path.to_str().ok_or("Invalid model path")?,
            params,
        )
        .map_err(|e| format!("Failed to load whisper model: {}", e))?;

        Ok(Self {
            ctx,
            language: language.to_string(),
            initial_prompt,
            post_processor: PostProcessor::new(),
        })
    }

    /// Transcribe a chunk of 16kHz mono f32 audio.
    pub fn transcribe(&self, audio: &[f32]) -> Result<TranscribeResult, String> {
        // Skip very short audio — Whisper hallucinates on tiny clips
        if audio.len() < MIN_AUDIO_SAMPLES {
            return Ok(TranscribeResult {
                text: String::new(),
                segments: Vec::new(),
            });
        }

        let mut state = self
            .ctx
            .create_state()
            .map_err(|e| format!("Failed to create whisper state: {}", e))?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some(&self.language));
        params.set_translate(false);
        params.set_no_timestamps(false);
        params.set_single_segment(false);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_suppress_blank(true);
        params.set_suppress_nst(true);
        params.set_no_context(true); // Prevent cross-segment hallucination carry-over

        let n_threads = std::thread::available_parallelism()
            .map(|n| (n.get() / 2).max(1) as i32)
            .unwrap_or(4);
        params.set_n_threads(n_threads);

        if let Some(ref prompt) = self.initial_prompt {
            params.set_initial_prompt(prompt);
        }

        // Run transcription
        state
            .full(params, audio)
            .map_err(|e| format!("Transcription failed: {}", e))?;

        // Extract results
        let num_segments = state.full_n_segments();
        let mut segments = Vec::new();
        let mut full_text = String::new();

        for i in 0..num_segments {
            if let Some(segment) = state.get_segment(i) {
                // Filter hallucinations: skip segments with high no_speech probability
                let no_speech_prob = segment.no_speech_probability();
                if no_speech_prob > NO_SPEECH_PROB_THRESHOLD {
                    println!(
                        "[STT] Skipping hallucinated segment (no_speech_prob={:.2}): '{}'",
                        no_speech_prob,
                        segment.to_str_lossy().unwrap_or_default()
                    );
                    continue;
                }

                let text = segment
                    .to_str_lossy()
                    .map_err(|e| format!("Failed to get segment text: {}", e))?;

                let start = segment.start_timestamp();
                let end = segment.end_timestamp();

                let processed_text = self.post_processor.process(&text);

                if !processed_text.trim().is_empty() {
                    segments.push(TextSegment {
                        text: processed_text.clone(),
                        start_ms: start * 10,
                        end_ms: end * 10,
                    });
                    full_text.push_str(&processed_text);
                }
            }
        }

        Ok(TranscribeResult {
            text: full_text.trim().to_string(),
            segments,
        })
    }
}
