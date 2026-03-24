use crate::models::Session;
use crate::session::storage::Database;

pub struct SessionManager {
    db: Database,
}

impl SessionManager {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub fn create_session(
        &self,
        title: String,
        participants: Option<Vec<String>>,
        context_hint: Option<String>,
    ) -> Result<Session, String> {
        let now = chrono::Utc::now().to_rfc3339();
        let id = uuid::Uuid::new_v4().to_string();
        let participants_json = participants.map(|p| serde_json::to_string(&p).unwrap_or_default());

        let session = Session {
            id,
            title,
            started_at: now.clone(),
            ended_at: None,
            duration_secs: None,
            participants: participants_json,
            context_hint,
            notes: None,
            status: "created".to_string(),
            audio_path: None,
            model_used: None,
            created_at: now.clone(),
            updated_at: now,
        };

        self.db
            .insert_session(&session)
            .map_err(|e| format!("Failed to create session: {}", e))?;

        Ok(session)
    }

    /// Transition session to "recording". Stores the recording start time in
    /// a transient field (audio_path is repurposed as _recording_started_at)
    /// so we can calculate incremental duration on stop.
    pub fn start_session_recording(&self, id: &str) -> Result<Session, String> {
        let now = chrono::Utc::now().to_rfc3339();
        self.db
            .update_session_status(id, "recording", None, None)
            .map_err(|e| format!("Failed to update session: {}", e))?;

        // Store recording start timestamp in audio_path temporarily
        let conn = self.db.conn();
        let conn = conn.lock().unwrap();
        conn.execute(
            "UPDATE sessions SET audio_path = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![now, now, id],
        )
        .map_err(|e| format!("Failed to update recording start: {}", e))?;
        drop(conn);

        self.db
            .get_session(id)
            .map_err(|e| format!("Failed to fetch session: {}", e))
    }

    /// Stop recording. Adds this recording segment's duration to existing duration_secs.
    pub fn stop_session(&self, id: &str) -> Result<Session, String> {
        let session = self
            .db
            .get_session(id)
            .map_err(|e| format!("Session not found: {}", e))?;

        let now = chrono::Utc::now();

        // Calculate this recording segment's duration
        let segment_duration = if let Some(ref recording_start) = session.audio_path {
            chrono::DateTime::parse_from_rfc3339(recording_start)
                .map(|started| (now - started.with_timezone(&chrono::Utc)).num_seconds())
                .unwrap_or(0)
        } else {
            0
        };

        // Accumulate total duration
        let total_duration = session.duration_secs.unwrap_or(0) + segment_duration;

        // Clear the recording start marker, set status to completed
        let conn = self.db.conn();
        let conn = conn.lock().unwrap();
        conn.execute(
            "UPDATE sessions SET status = ?1, ended_at = ?2, duration_secs = ?3, audio_path = NULL, updated_at = ?2 WHERE id = ?4",
            rusqlite::params!["completed", now.to_rfc3339(), total_duration, id],
        )
        .map_err(|e| format!("Failed to update session: {}", e))?;
        drop(conn);

        self.db
            .get_session(id)
            .map_err(|e| format!("Failed to fetch updated session: {}", e))
    }

    pub fn list_sessions(&self) -> Result<Vec<Session>, String> {
        self.db
            .list_sessions()
            .map_err(|e| format!("Failed to list sessions: {}", e))
    }

    pub fn get_session(&self, id: &str) -> Result<Session, String> {
        self.db
            .get_session(id)
            .map_err(|e| format!("Session not found: {}", e))
    }

    pub fn update_notes(&self, id: &str, notes: &str) -> Result<(), String> {
        self.db
            .update_session_notes(id, notes)
            .map_err(|e| format!("Failed to update notes: {}", e))
    }

    pub fn db(&self) -> &Database {
        &self.db
    }
}
