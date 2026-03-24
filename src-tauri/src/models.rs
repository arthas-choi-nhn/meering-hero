use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration_secs: Option<i64>,
    pub participants: Option<String>,
    pub context_hint: Option<String>,
    pub notes: Option<String>,
    pub status: String,
    pub audio_path: Option<String>,
    pub model_used: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub id: i64,
    pub session_id: String,
    pub text: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub is_final: bool,
    pub speaker: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub id: i64,
    pub session_id: String,
    pub template: String,
    pub content: String,
    pub provider: String,
    pub cost_usd: Option<f64>,
    pub duration_ms: Option<i64>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Export {
    pub id: i64,
    pub session_id: String,
    pub summary_id: Option<i64>,
    pub target: String,
    pub target_url: Option<String>,
    pub exported_at: String,
}
