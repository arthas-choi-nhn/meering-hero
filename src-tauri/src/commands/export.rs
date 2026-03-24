use crate::commands::session::AppState;
use crate::commands::settings::load_settings;
use crate::export::dooray::{DoorayClient, Wiki, WikiPage};
use crate::export::renderer;
use crate::models::Export;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoorayConfig {
    pub base_url: String,
    pub token: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportResult {
    pub page_id: String,
}

fn get_dooray_client() -> Result<DoorayClient, String> {
    let settings = load_settings();
    let base_url = settings
        .dooray_base_url
        .ok_or("Dooray Base URL이 설정되지 않았습니다. 설정에서 입력해주세요.")?;
    let token = settings
        .dooray_token
        .ok_or("Dooray API Token이 설정되지 않았습니다. 설정에서 입력해주세요.")?;
    Ok(DoorayClient::new(&base_url, &token))
}

#[tauri::command]
pub async fn list_dooray_wikis() -> Result<Vec<Wiki>, String> {
    let client = get_dooray_client()?;
    client.list_wikis().await
}

#[tauri::command]
pub async fn list_dooray_wiki_pages(
    wiki_id: String,
    parent_page_id: Option<String>,
) -> Result<Vec<WikiPage>, String> {
    let client = get_dooray_client()?;
    let mut pages = client
        .list_wiki_pages(&wiki_id, parent_page_id.as_deref())
        .await?;

    // If top-level query returns only a root page (Home),
    // automatically drill into it to show actual pages
    if parent_page_id.is_none() && pages.len() == 1 && pages[0].root {
        let root_id = pages[0].id.clone();
        pages = client
            .list_wiki_pages(&wiki_id, Some(&root_id))
            .await?;
    }

    Ok(pages)
}

#[tauri::command]
pub async fn create_dooray_wiki_page(
    state: State<'_, Mutex<AppState>>,
    session_id: String,
    summary_id: Option<i64>,
    wiki_id: String,
    parent_page_id: Option<String>,
    title: String,
) -> Result<ExportResult, String> {
    let client = get_dooray_client()?;

    let (session, segments, summary_content) = gather_export_data(&state, &session_id, summary_id)?;

    let body = renderer::render_wiki_page(
        &session,
        &segments,
        summary_content.as_deref(),
        session.notes.as_deref(),
    );

    let result = client
        .create_wiki_page(&wiki_id, parent_page_id.as_deref(), &title, &body)
        .await?;

    save_export_record(&state, &session_id, summary_id, None)?;

    Ok(ExportResult {
        page_id: result.id,
    })
}

#[tauri::command]
pub async fn update_dooray_wiki_page(
    state: State<'_, Mutex<AppState>>,
    session_id: String,
    summary_id: Option<i64>,
    wiki_id: String,
    page_id: String,
) -> Result<(), String> {
    let client = get_dooray_client()?;

    let (session, segments, summary_content) = gather_export_data(&state, &session_id, summary_id)?;

    let body = renderer::render_wiki_page(
        &session,
        &segments,
        summary_content.as_deref(),
        session.notes.as_deref(),
    );

    client.update_wiki_page(&wiki_id, &page_id, &body).await?;

    save_export_record(&state, &session_id, summary_id, None)?;

    Ok(())
}

fn gather_export_data(
    state: &State<'_, Mutex<AppState>>,
    session_id: &str,
    summary_id: Option<i64>,
) -> Result<
    (
        crate::models::Session,
        Vec<crate::models::Segment>,
        Option<String>,
    ),
    String,
> {
    let app = state.lock().map_err(|e| e.to_string())?;
    let session = app.session_manager.get_session(session_id)?;
    let segments = app
        .session_manager
        .db()
        .get_segments_by_session(session_id)
        .map_err(|e| format!("Failed to get segments: {}", e))?;
    let summary = summary_id.and_then(|id| {
        app.session_manager
            .db()
            .get_summaries_by_session(session_id)
            .ok()?
            .into_iter()
            .find(|s| s.id == id)
    }).map(|s| s.content);
    Ok((session, segments, summary))
}

fn save_export_record(
    state: &State<'_, Mutex<AppState>>,
    session_id: &str,
    summary_id: Option<i64>,
    target_url: Option<String>,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    let export = Export {
        id: 0,
        session_id: session_id.to_string(),
        summary_id,
        target: "dooray_wiki".to_string(),
        target_url,
        exported_at: now,
    };
    let app = state.lock().map_err(|e| e.to_string())?;
    app.session_manager
        .db()
        .insert_export(&export)
        .map_err(|e| format!("Failed to save export record: {}", e))?;
    Ok(())
}
