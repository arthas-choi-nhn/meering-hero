mod audio;
mod commands;
mod export;
mod llm;
mod model;
mod models;
mod session;
mod stt;

use commands::recording::PipelineState;
use commands::session::AppState;
use session::manager::SessionManager;
use session::storage::Database;
use std::sync::Mutex;

fn get_db_path() -> std::path::PathBuf {
    let base = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("meeting-app")
        .join("db");
    base.join("sessions.db")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db_path = get_db_path();
    let db = Database::new(&db_path).expect("Failed to initialize database");
    let session_manager = SessionManager::new(db);
    let app_state = Mutex::new(AppState { session_manager });
    let pipeline_state = Mutex::new(PipelineState { pipeline: None });

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            #[cfg(desktop)]
            app.handle().plugin(tauri_plugin_updater::Builder::new().build())?;
            Ok(())
        })
        .manage(app_state)
        .manage(pipeline_state)
        .invoke_handler(tauri::generate_handler![
            // Session commands
            commands::session::create_session,
            commands::session::list_sessions,
            commands::session::get_session,
            commands::session::stop_session,
            commands::session::update_session_notes,
            commands::session::get_segments,
            commands::session::get_summaries,
            commands::session::search_segments,
            // Audio commands
            commands::audio::get_audio_devices,
            // Model commands
            commands::model::get_model_status,
            // Recording pipeline commands
            commands::recording::start_recording,
            commands::recording::stop_recording,
            commands::recording::pause_recording,
            commands::recording::resume_recording,
            // Summary commands
            commands::summary::check_claude_status,
            commands::summary::summarize_session,
            commands::summary::update_summary_content,
            // Export commands
            commands::export::list_dooray_wikis,
            commands::export::list_dooray_wiki_pages,
            commands::export::create_dooray_wiki_page,
            commands::export::update_dooray_wiki_page,
            // Settings commands
            commands::settings::load_settings,
            commands::settings::save_settings,
            commands::settings::get_full_model_status,
            commands::settings::download_model,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
