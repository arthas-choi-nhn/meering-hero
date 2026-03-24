use crate::audio::capture::{AudioCaptureManager, AudioDevice};

#[tauri::command]
pub fn get_audio_devices() -> Result<Vec<AudioDevice>, String> {
    AudioCaptureManager::list_devices()
}
