use crate::commands::session::AppState;
use crate::commands::settings::load_settings;
use crate::llm::claude_cli;
use crate::models::Summary;
use std::sync::Mutex;
use tauri::State;

#[tauri::command]
pub fn check_claude_status() -> claude_cli::ClaudeStatus {
    claude_cli::check_status()
}

#[tauri::command]
pub async fn summarize_session(
    state: State<'_, Mutex<AppState>>,
    session_id: String,
    template: String,
) -> Result<Summary, String> {
    // Get all segments for the session
    let transcript = {
        let app = state.lock().map_err(|e| e.to_string())?;
        let segments = app
            .session_manager
            .db()
            .get_segments_by_session(&session_id)
            .map_err(|e| format!("Failed to get segments: {}", e))?;

        if segments.is_empty() {
            return Err("전사 내용이 없습니다".to_string());
        }

        segments
            .iter()
            .map(|s| {
                let time_secs = s.start_ms / 1000;
                let mins = time_secs / 60;
                let secs = time_secs % 60;
                format!("[{:02}:{:02}] {}", mins, secs, s.text.trim())
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let settings = load_settings();
    let custom_prompt = settings.summary_prompt.clone();

    let start = std::time::Instant::now();
    let content = claude_cli::summarize(&transcript, &template, custom_prompt.as_deref()).await?;
    let duration_ms = start.elapsed().as_millis() as i64;

    let now = chrono::Utc::now().to_rfc3339();
    let summary = Summary {
        id: 0,
        session_id: session_id.clone(),
        template,
        content: content.clone(),
        provider: "ClaudeCodeCli".to_string(),
        cost_usd: None,
        duration_ms: Some(duration_ms),
        created_at: now,
    };

    // Save to DB
    let id = {
        let app = state.lock().map_err(|e| e.to_string())?;
        app.session_manager
            .db()
            .insert_summary(&summary)
            .map_err(|e| format!("Failed to save summary: {}", e))?
    };

    Ok(Summary { id, ..summary })
}

#[tauri::command]
pub fn update_summary_content(
    state: State<'_, Mutex<AppState>>,
    summary_id: i64,
    content: String,
) -> Result<(), String> {
    let app = state.lock().map_err(|e| e.to_string())?;
    app.session_manager
        .db()
        .update_summary_content(summary_id, &content)
        .map_err(|e| format!("Failed to update summary: {}", e))
}
