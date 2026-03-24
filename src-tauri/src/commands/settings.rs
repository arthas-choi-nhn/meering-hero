use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Emitter;

use crate::model::manager::{ModelManager, ModelSize, ModelStatus};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppSettings {
    pub dooray_base_url: Option<String>,
    pub dooray_token: Option<String>,
    /// "small", "medium", "large", or null (auto-detect by RAM)
    pub stt_model: Option<String>,
    /// Selected audio input device name, or null (use default)
    pub audio_device: Option<String>,
    /// "energy" (default) or "silero"
    pub vad_mode: Option<String>,
    /// Custom system prompt for Claude summarization (None = use default)
    pub summary_prompt: Option<String>,
}

fn settings_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("meeting-app")
        .join("settings.json")
}

#[tauri::command]
pub fn load_settings() -> AppSettings {
    let path = settings_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

#[tauri::command]
pub fn save_settings(settings: AppSettings) -> Result<(), String> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let json = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("Failed to write settings: {}", e))
}

#[derive(Debug, Clone, Serialize)]
pub struct FullModelStatus {
    pub stt: ModelStatus,
    pub vad_downloaded: bool,
    pub vad_path: String,
}

fn models_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("meeting-app")
        .join("models")
}

#[tauri::command]
pub fn get_full_model_status() -> FullModelStatus {
    let manager = ModelManager::new();
    let stt = manager.get_status();
    let vad_path = models_dir().join("silero_vad.onnx");

    // Clean up leftover .downloading temp files
    if let Ok(entries) = std::fs::read_dir(models_dir()) {
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|e| e.to_str()) == Some("downloading") {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }

    FullModelStatus {
        stt,
        vad_downloaded: vad_path.exists(),
        vad_path: vad_path.to_string_lossy().to_string(),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub model: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub done: bool,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn download_model(
    app_handle: tauri::AppHandle,
    model: String,
) -> Result<String, String> {
    let (url, dest_path) = match model.as_str() {
        "vad" => {
            let dest = models_dir().join("silero_vad.onnx");
            (
                "https://github.com/snakers4/silero-vad/raw/master/src/silero_vad/data/silero_vad.onnx".to_string(),
                dest,
            )
        }
        "small" | "medium" | "large" => {
            let manager = ModelManager::new();
            let size = match model.as_str() {
                "small" => ModelSize::Small,
                "medium" => ModelSize::Medium,
                _ => ModelSize::Large,
            };
            let dest = manager.model_path(size);
            let filename = size.model_filename();
            let url = format!(
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}",
                filename
            );
            (url, dest)
        }
        _ => return Err(format!("Unknown model: {}", model)),
    };

    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    // Download to a temp file first, rename on completion.
    // If app crashes mid-download, only the .downloading file remains.
    let tmp_path = dest_path.with_extension("downloading");

    // Remove any previous partial download
    let _ = std::fs::remove_file(&tmp_path);
    // Remove previous complete file if re-downloading
    let _ = std::fs::remove_file(&dest_path);

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Download request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Download failed: HTTP {}", resp.status()));
    }

    let total = resp.content_length();
    let mut downloaded: u64 = 0;
    let mut file = tokio::fs::File::create(&tmp_path)
        .await
        .map_err(|e| format!("Failed to create temp file: {}", e))?;

    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;
    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Write error: {}", e))?;
        downloaded += chunk.len() as u64;

        let _ = app_handle.emit(
            "download:progress",
            DownloadProgress {
                model: model.clone(),
                downloaded_bytes: downloaded,
                total_bytes: total,
                done: false,
                error: None,
            },
        );
    }

    // Flush and close before rename
    file.flush().await.map_err(|e| format!("Flush error: {}", e))?;
    drop(file);

    // Atomic rename: tmp → final (only if download completed fully)
    std::fs::rename(&tmp_path, &dest_path)
        .map_err(|e| format!("Failed to finalize download: {}", e))?;

    let _ = app_handle.emit(
        "download:progress",
        DownloadProgress {
            model: model.clone(),
            downloaded_bytes: downloaded,
            total_bytes: total,
            done: true,
            error: None,
        },
    );

    Ok(dest_path.to_string_lossy().to_string())
}
