use crate::audio::vad::VadProcessor;
use crate::commands::settings;
use crate::model::manager::{ModelManager, ModelSize};
use crate::stt::engine::SttEngine;
use crate::stt::pipeline::PipelineHandle;
use std::sync::{Arc, Mutex};
use tauri::State;

pub struct PipelineState {
    pub pipeline: Option<PipelineHandle>,
}

#[tauri::command]
pub fn start_recording(
    state: State<'_, Mutex<PipelineState>>,
    session_state: State<'_, Mutex<crate::commands::session::AppState>>,
    app_handle: tauri::AppHandle,
    session_id: String,
    device_name: Option<String>,
) -> Result<(), String> {
    // If no device_name provided, use the one from settings
    let device_name = device_name.or_else(|| {
        settings::load_settings().audio_device
    });

    // Mark session as recording and get context_hint
    let context_hint = {
        let app = session_state.lock().map_err(|e| e.to_string())?;
        let session = app.session_manager.start_session_recording(&session_id)?;
        session.context_hint
    };

    // Determine which STT model to use: settings override > auto-detect by RAM
    let app_settings = settings::load_settings();

    // Initialize VAD based on settings (default: energy)
    let vad = match app_settings.vad_mode.as_deref() {
        Some("silero") => {
            let user_vad = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("meeting-app")
                .join("models")
                .join("silero_vad.onnx");
            if !user_vad.exists() {
                return Err("Silero VAD 모델이 설치되지 않았습니다. 설정에서 다운로드해주세요.".to_string());
            }
            VadProcessor::new_silero(&user_vad)
                .map_err(|e| format!("Failed to initialize Silero VAD: {}", e))?
        }
        _ => VadProcessor::new_energy(),
    };
    let model_size = match app_settings.stt_model.as_deref() {
        Some("small") => ModelSize::Small,
        Some("medium") => ModelSize::Medium,
        Some("large") => ModelSize::Large,
        _ => ModelManager::recommended_model(),
    };

    let model_manager = ModelManager::new();
    let model_path = model_manager.model_path(model_size);

    if !model_path.exists() {
        return Err(format!(
            "STT 모델이 설치되지 않았습니다. 설정에서 {} 를 다운로드해주세요.",
            model_size.display_name()
        ));
    }

    let engine = SttEngine::new(&model_path, "ko", context_hint)
        .map_err(|e| format!("Failed to initialize STT engine: {}", e))?;

    // Get DB handle for segment persistence
    let db = {
        let app = session_state.lock().map_err(|e| e.to_string())?;
        app.session_manager.db().clone()
    };

    // Start pipeline
    let pipeline = PipelineHandle::start(
        session_id,
        device_name,
        vad,
        Arc::new(engine),
        db,
        app_handle,
    )?;

    let mut state = state.lock().map_err(|e| e.to_string())?;
    state.pipeline = Some(pipeline);

    Ok(())
}

#[tauri::command]
pub fn stop_recording(
    state: State<'_, Mutex<PipelineState>>,
    session_state: State<'_, Mutex<crate::commands::session::AppState>>,
    session_id: String,
) -> Result<crate::models::Session, String> {
    {
        let mut state = state.lock().map_err(|e| e.to_string())?;
        if let Some(ref mut pipeline) = state.pipeline {
            pipeline.stop();
        }
        state.pipeline = None;
    }

    let app = session_state.lock().map_err(|e| e.to_string())?;
    app.session_manager.stop_session(&session_id)
}

#[tauri::command]
pub fn pause_recording(state: State<'_, Mutex<PipelineState>>) -> Result<(), String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    if let Some(ref pipeline) = state.pipeline {
        pipeline.pause();
    }
    Ok(())
}

#[tauri::command]
pub fn resume_recording(state: State<'_, Mutex<PipelineState>>) -> Result<(), String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    if let Some(ref pipeline) = state.pipeline {
        pipeline.resume();
    }
    Ok(())
}
