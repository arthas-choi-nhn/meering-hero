use crate::models::{Segment, Session, Summary};
use crate::session::manager::SessionManager;
use std::sync::Mutex;
use tauri::State;

pub struct AppState {
    pub session_manager: SessionManager,
}

#[tauri::command]
pub fn create_session(
    state: State<'_, Mutex<AppState>>,
    title: String,
    participants: Option<Vec<String>>,
    context_hint: Option<String>,
) -> Result<Session, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    state.session_manager.create_session(title, participants, context_hint)
}

#[tauri::command]
pub fn list_sessions(state: State<'_, Mutex<AppState>>) -> Result<Vec<Session>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    state.session_manager.list_sessions()
}

#[tauri::command]
pub fn get_session(state: State<'_, Mutex<AppState>>, id: String) -> Result<Session, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    state.session_manager.get_session(&id)
}

#[tauri::command]
pub fn stop_session(state: State<'_, Mutex<AppState>>, id: String) -> Result<Session, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    state.session_manager.stop_session(&id)
}

#[tauri::command]
pub fn update_session_notes(
    state: State<'_, Mutex<AppState>>,
    id: String,
    notes: String,
) -> Result<(), String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    state.session_manager.update_notes(&id, &notes)
}

#[tauri::command]
pub fn get_segments(
    state: State<'_, Mutex<AppState>>,
    session_id: String,
) -> Result<Vec<Segment>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    state
        .session_manager
        .db()
        .get_segments_by_session(&session_id)
        .map_err(|e| format!("Failed to get segments: {}", e))
}

#[tauri::command]
pub fn get_summaries(
    state: State<'_, Mutex<AppState>>,
    session_id: String,
) -> Result<Vec<Summary>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    state
        .session_manager
        .db()
        .get_summaries_by_session(&session_id)
        .map_err(|e| format!("Failed to get summaries: {}", e))
}

#[tauri::command]
pub fn search_segments(
    state: State<'_, Mutex<AppState>>,
    query: String,
) -> Result<Vec<Segment>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    state
        .session_manager
        .db()
        .search_segments(&query)
        .map_err(|e| format!("Search failed: {}", e))
}
