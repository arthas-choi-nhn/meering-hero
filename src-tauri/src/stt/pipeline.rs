use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::mpsc;

use crate::audio::capture::AudioCaptureManager;
use crate::audio::vad::{SpeechSegment, VadProcessor};
use crate::models::Segment;
use crate::session::storage::Database;
use crate::stt::engine::SttEngine;

/// Event payload emitted to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct SttEvent {
    pub session_id: String,
    pub text: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub is_partial: bool,
}

/// Thread-safe pipeline handle. The actual audio stream runs on a separate thread.
pub struct PipelineHandle {
    shutdown: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    pipeline_thread: Option<std::thread::JoinHandle<()>>,
}

// Safety: PipelineHandle only contains Arc<AtomicBool> and JoinHandle,
// both of which are Send + Sync.
unsafe impl Send for PipelineHandle {}

impl PipelineHandle {
    pub fn start(
        session_id: String,
        device_name: Option<String>,
        vad: VadProcessor,
        engine: Arc<SttEngine>,
        db: Database,
        app_handle: tauri::AppHandle,
    ) -> Result<Self, String> {
        let shutdown = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(AtomicBool::new(false));

        let shutdown_clone = Arc::clone(&shutdown);
        let paused_clone = Arc::clone(&paused);

        let pipeline_thread = std::thread::spawn(move || {
            Self::run_pipeline(
                session_id,
                device_name,
                vad,
                engine,
                db,
                app_handle,
                shutdown_clone,
                paused_clone,
            );
        });

        Ok(Self {
            shutdown,
            paused,
            pipeline_thread: Some(pipeline_thread),
        })
    }

    pub fn stop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self.pipeline_thread.take() {
            let _ = handle.join();
        }
    }

    pub fn pause(&self) {
        self.paused.store(true, Ordering::Relaxed);
    }

    pub fn resume(&self) {
        self.paused.store(false, Ordering::Relaxed);
    }

    fn run_pipeline(
        session_id: String,
        device_name: Option<String>,
        mut vad: VadProcessor,
        engine: Arc<SttEngine>,
        db: Database,
        app_handle: tauri::AppHandle,
        shutdown: Arc<AtomicBool>,
        paused: Arc<AtomicBool>,
    ) {
        println!("[Pipeline] Starting audio capture (device: {:?})...", device_name);
        let (tx, mut rx) = mpsc::unbounded_channel::<Vec<f32>>();

        let mut capture = AudioCaptureManager::new();
        if let Err(e) = capture.start(device_name.as_deref(), tx) {
            eprintln!("[Pipeline] Failed to start audio capture: {}", e);
            return;
        }
        println!("[Pipeline] Audio capture started, entering main loop");

        let chunk_size = 512;
        let mut audio_buffer: Vec<f32> = Vec::new();
        let mut recv_count: u64 = 0;

        while !shutdown.load(Ordering::Relaxed) {
            if paused.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(50));
                continue;
            }

            match rx.try_recv() {
                Ok(samples) => {
                    recv_count += 1;
                    if recv_count <= 3 || recv_count % 500 == 0 {
                        let max_val = samples.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
                        println!(
                            "[Pipeline] Audio recv #{}: {} samples, peak={:.4}",
                            recv_count, samples.len(), max_val
                        );
                    }
                    audio_buffer.extend_from_slice(&samples);

                    while audio_buffer.len() >= chunk_size {
                        let chunk: Vec<f32> = audio_buffer.drain(..chunk_size).collect();
                        let segments = vad.process_chunk(&chunk);

                        for segment in segments {
                            println!(
                                "[Pipeline] Speech segment detected: {}ms - {}ms ({} samples)",
                                segment.start_ms, segment.end_ms, segment.samples.len()
                            );
                            Self::handle_speech_segment(
                                &session_id,
                                &segment,
                                &engine,
                                &db,
                                &app_handle,
                            );
                        }
                    }
                }
                Err(mpsc::error::TryRecvError::Empty) => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    break;
                }
            }
        }

        // Flush remaining VAD buffer
        if let Some(segment) = vad.flush() {
            Self::handle_speech_segment(&session_id, &segment, &engine, &db, &app_handle);
        }

        capture.stop();
    }

    fn handle_speech_segment(
        session_id: &str,
        segment: &SpeechSegment,
        engine: &SttEngine,
        db: &Database,
        app_handle: &tauri::AppHandle,
    ) {
        let min_samples = 4800; // 0.3s — skip tiny segments that cause hallucinations
        let max_samples = 16000 * 3; // 3 seconds at 16kHz
        let samples = &segment.samples;

        if samples.len() < min_samples {
            println!("[Pipeline] Skipping short segment ({} samples < {})", samples.len(), min_samples);
            return;
        }

        if samples.len() <= max_samples {
            Self::transcribe_and_emit(
                session_id,
                samples,
                segment.start_ms,
                segment.end_ms,
                engine,
                db,
                app_handle,
            );
        } else {
            let step = max_samples;
            let mut offset = 0;
            while offset < samples.len() {
                let end = (offset + max_samples).min(samples.len());
                let window = &samples[offset..end];
                let window_start_ms = segment.start_ms + (offset as u64 * 1000 / 16000);
                let window_end_ms = segment.start_ms + (end as u64 * 1000 / 16000);

                Self::transcribe_and_emit(
                    session_id,
                    window,
                    window_start_ms,
                    window_end_ms,
                    engine,
                    db,
                    app_handle,
                );

                offset += step;
            }
        }
    }

    fn transcribe_and_emit(
        session_id: &str,
        audio: &[f32],
        start_ms: u64,
        end_ms: u64,
        engine: &SttEngine,
        db: &Database,
        app_handle: &tauri::AppHandle,
    ) {
        println!("[Pipeline] Transcribing {} samples...", audio.len());
        match engine.transcribe(audio) {
            Ok(result) => {
                println!("[Pipeline] Transcribed: '{}'", result.text);
                if !result.text.is_empty() {
                    // Save to DB
                    let now = chrono::Utc::now().to_rfc3339();
                    let segment = Segment {
                        id: 0,
                        session_id: session_id.to_string(),
                        text: result.text.clone(),
                        start_ms: start_ms as i64,
                        end_ms: end_ms as i64,
                        is_final: true,
                        speaker: None,
                        created_at: now,
                    };
                    if let Err(e) = db.insert_segment(&segment) {
                        eprintln!("[Pipeline] Failed to save segment: {}", e);
                    }

                    // Emit to frontend
                    let event = SttEvent {
                        session_id: session_id.to_string(),
                        text: result.text,
                        start_ms: start_ms as i64,
                        end_ms: end_ms as i64,
                        is_partial: false,
                    };
                    let _ = app_handle.emit("stt:final", &event);
                }
            }
            Err(e) => {
                eprintln!("[Pipeline] STT error: {}", e);
            }
        }
    }
}
