use crate::model::manager::{ModelManager, ModelStatus};

#[tauri::command]
pub fn get_model_status() -> ModelStatus {
    let manager = ModelManager::new();
    manager.get_status()
}
