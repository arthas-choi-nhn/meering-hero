use rusqlite::{params, Connection, Result as SqlResult};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::models::{Export, Segment, Session, Summary};

pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(db_path: &Path) -> SqlResult<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(db_path)?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS sessions (
                id              TEXT PRIMARY KEY,
                title           TEXT NOT NULL,
                started_at      TEXT NOT NULL,
                ended_at        TEXT,
                duration_secs   INTEGER,
                participants    TEXT,
                context_hint    TEXT,
                notes           TEXT,
                status          TEXT NOT NULL,
                audio_path      TEXT,
                model_used      TEXT,
                created_at      TEXT NOT NULL,
                updated_at      TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS segments (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id      TEXT NOT NULL REFERENCES sessions(id),
                text            TEXT NOT NULL,
                start_ms        INTEGER NOT NULL,
                end_ms          INTEGER NOT NULL,
                is_final        BOOLEAN NOT NULL DEFAULT 1,
                speaker         TEXT,
                created_at      TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS summaries (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id      TEXT NOT NULL REFERENCES sessions(id),
                template        TEXT NOT NULL,
                content         TEXT NOT NULL,
                provider        TEXT NOT NULL,
                cost_usd        REAL,
                duration_ms     INTEGER,
                created_at      TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS exports (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id      TEXT NOT NULL REFERENCES sessions(id),
                summary_id      INTEGER REFERENCES summaries(id),
                target          TEXT NOT NULL,
                target_url      TEXT,
                exported_at     TEXT NOT NULL
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS segments_fts USING fts5(
                text,
                content=segments,
                content_rowid=id
            );

            CREATE TRIGGER IF NOT EXISTS segments_ai AFTER INSERT ON segments BEGIN
                INSERT INTO segments_fts(rowid, text) VALUES (new.id, new.text);
            END;

            CREATE TRIGGER IF NOT EXISTS segments_ad AFTER DELETE ON segments BEGIN
                INSERT INTO segments_fts(segments_fts, rowid, text) VALUES('delete', old.id, old.text);
            END;

            CREATE TRIGGER IF NOT EXISTS segments_au AFTER UPDATE ON segments BEGIN
                INSERT INTO segments_fts(segments_fts, rowid, text) VALUES('delete', old.id, old.text);
                INSERT INTO segments_fts(rowid, text) VALUES (new.id, new.text);
            END;
            ",
        )?;
        Ok(())
    }

    // --- Session CRUD ---

    pub fn insert_session(&self, session: &Session) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO sessions (id, title, started_at, ended_at, duration_secs, participants, context_hint, notes, status, audio_path, model_used, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                session.id,
                session.title,
                session.started_at,
                session.ended_at,
                session.duration_secs,
                session.participants,
                session.context_hint,
                session.notes,
                session.status,
                session.audio_path,
                session.model_used,
                session.created_at,
                session.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn get_session(&self, id: &str) -> SqlResult<Session> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, title, started_at, ended_at, duration_secs, participants, context_hint, notes, status, audio_path, model_used, created_at, updated_at FROM sessions WHERE id = ?1",
            params![id],
            |row| {
                Ok(Session {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    started_at: row.get(2)?,
                    ended_at: row.get(3)?,
                    duration_secs: row.get(4)?,
                    participants: row.get(5)?,
                    context_hint: row.get(6)?,
                    notes: row.get(7)?,
                    status: row.get(8)?,
                    audio_path: row.get(9)?,
                    model_used: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                })
            },
        )
    }

    pub fn list_sessions(&self) -> SqlResult<Vec<Session>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, started_at, ended_at, duration_secs, participants, context_hint, notes, status, audio_path, model_used, created_at, updated_at FROM sessions ORDER BY created_at DESC",
        )?;
        let sessions = stmt
            .query_map([], |row| {
                Ok(Session {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    started_at: row.get(2)?,
                    ended_at: row.get(3)?,
                    duration_secs: row.get(4)?,
                    participants: row.get(5)?,
                    context_hint: row.get(6)?,
                    notes: row.get(7)?,
                    status: row.get(8)?,
                    audio_path: row.get(9)?,
                    model_used: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;
        Ok(sessions)
    }

    pub fn update_session_status(&self, id: &str, status: &str, ended_at: Option<&str>, duration_secs: Option<i64>) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE sessions SET status = ?1, ended_at = ?2, duration_secs = ?3, updated_at = ?4 WHERE id = ?5",
            params![status, ended_at, duration_secs, now, id],
        )?;
        Ok(())
    }

    pub fn update_session_notes(&self, id: &str, notes: &str) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE sessions SET notes = ?1, updated_at = ?2 WHERE id = ?3",
            params![notes, now, id],
        )?;
        Ok(())
    }

    // --- Segment CRUD ---

    pub fn insert_segment(&self, segment: &Segment) -> SqlResult<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO segments (session_id, text, start_ms, end_ms, is_final, speaker, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                segment.session_id,
                segment.text,
                segment.start_ms,
                segment.end_ms,
                segment.is_final,
                segment.speaker,
                segment.created_at,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_segments_by_session(&self, session_id: &str) -> SqlResult<Vec<Segment>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, text, start_ms, end_ms, is_final, speaker, created_at FROM segments WHERE session_id = ?1 ORDER BY start_ms",
        )?;
        let segments = stmt
            .query_map(params![session_id], |row| {
                Ok(Segment {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    text: row.get(2)?,
                    start_ms: row.get(3)?,
                    end_ms: row.get(4)?,
                    is_final: row.get(5)?,
                    speaker: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;
        Ok(segments)
    }

    // --- Summary CRUD ---

    pub fn insert_summary(&self, summary: &Summary) -> SqlResult<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO summaries (session_id, template, content, provider, cost_usd, duration_ms, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                summary.session_id,
                summary.template,
                summary.content,
                summary.provider,
                summary.cost_usd,
                summary.duration_ms,
                summary.created_at,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_summaries_by_session(&self, session_id: &str) -> SqlResult<Vec<Summary>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, template, content, provider, cost_usd, duration_ms, created_at FROM summaries WHERE session_id = ?1 ORDER BY created_at DESC",
        )?;
        let summaries = stmt
            .query_map(params![session_id], |row| {
                Ok(Summary {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    template: row.get(2)?,
                    content: row.get(3)?,
                    provider: row.get(4)?,
                    cost_usd: row.get(5)?,
                    duration_ms: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;
        Ok(summaries)
    }

    pub fn update_summary_content(&self, id: i64, content: &str) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE summaries SET content = ?1 WHERE id = ?2",
            params![content, id],
        )?;
        Ok(())
    }

    // --- Export CRUD ---

    pub fn insert_export(&self, export: &Export) -> SqlResult<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO exports (session_id, summary_id, target, target_url, exported_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                export.session_id,
                export.summary_id,
                export.target,
                export.target_url,
                export.exported_at,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    // --- Full-text Search ---

    pub fn search_segments(&self, query: &str) -> SqlResult<Vec<Segment>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT s.id, s.session_id, s.text, s.start_ms, s.end_ms, s.is_final, s.speaker, s.created_at
             FROM segments s
             JOIN segments_fts fts ON s.id = fts.rowid
             WHERE segments_fts MATCH ?1
             ORDER BY rank",
        )?;
        let segments = stmt
            .query_map(params![query], |row| {
                Ok(Segment {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    text: row.get(2)?,
                    start_ms: row.get(3)?,
                    end_ms: row.get(4)?,
                    is_final: row.get(5)?,
                    speaker: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;
        Ok(segments)
    }
}

impl Database {
    pub fn conn(&self) -> &Arc<Mutex<Connection>> {
        &self.conn
    }
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            conn: Arc::clone(&self.conn),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_db() -> Database {
        Database::new(PathBuf::from(":memory:").as_path()).unwrap()
    }

    #[test]
    fn test_session_crud() {
        let db = create_test_db();
        let now = chrono::Utc::now().to_rfc3339();

        let session = Session {
            id: "test-session-1".to_string(),
            title: "테스트 회의".to_string(),
            started_at: now.clone(),
            ended_at: None,
            duration_secs: None,
            participants: Some(r#"["arthas", "member1"]"#.to_string()),
            context_hint: Some("DLS 인프라".to_string()),
            notes: None,
            status: "recording".to_string(),
            audio_path: None,
            model_used: Some("whisper-medium".to_string()),
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        db.insert_session(&session).unwrap();

        let fetched = db.get_session("test-session-1").unwrap();
        assert_eq!(fetched.title, "테스트 회의");
        assert_eq!(fetched.status, "recording");

        db.update_session_status("test-session-1", "completed", Some(&now), Some(2700))
            .unwrap();
        let updated = db.get_session("test-session-1").unwrap();
        assert_eq!(updated.status, "completed");
        assert_eq!(updated.duration_secs, Some(2700));

        let sessions = db.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
    }

    #[test]
    fn test_segment_crud_and_fts() {
        let db = create_test_db();
        let now = chrono::Utc::now().to_rfc3339();

        let session = Session {
            id: "sess-1".to_string(),
            title: "FTS 테스트".to_string(),
            started_at: now.clone(),
            ended_at: None,
            duration_secs: None,
            participants: None,
            context_hint: None,
            notes: None,
            status: "completed".to_string(),
            audio_path: None,
            model_used: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        db.insert_session(&session).unwrap();

        let seg1 = Segment {
            id: 0,
            session_id: "sess-1".to_string(),
            text: "HAProxy 보안 룰 업데이트 건으로 논의합니다".to_string(),
            start_ms: 0,
            end_ms: 5000,
            is_final: true,
            speaker: None,
            created_at: now.clone(),
        };
        let seg2 = Segment {
            id: 0,
            session_id: "sess-1".to_string(),
            text: "CrowdSec 오프라인 모드 설정 확인 필요".to_string(),
            start_ms: 5000,
            end_ms: 10000,
            is_final: true,
            speaker: None,
            created_at: now.clone(),
        };

        db.insert_segment(&seg1).unwrap();
        db.insert_segment(&seg2).unwrap();

        let segments = db.get_segments_by_session("sess-1").unwrap();
        assert_eq!(segments.len(), 2);

        let results = db.search_segments("HAProxy").unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].text.contains("HAProxy"));

        let results2 = db.search_segments("CrowdSec").unwrap();
        assert_eq!(results2.len(), 1);
    }

    #[test]
    fn test_summary_crud() {
        let db = create_test_db();
        let now = chrono::Utc::now().to_rfc3339();

        let session = Session {
            id: "sess-2".to_string(),
            title: "요약 테스트".to_string(),
            started_at: now.clone(),
            ended_at: None,
            duration_secs: None,
            participants: None,
            context_hint: None,
            notes: None,
            status: "completed".to_string(),
            audio_path: None,
            model_used: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        db.insert_session(&session).unwrap();

        let summary = Summary {
            id: 0,
            session_id: "sess-2".to_string(),
            template: "MeetingMinutes".to_string(),
            content: "## 요약\n- 보안 업데이트 논의".to_string(),
            provider: "ClaudeCodeCli".to_string(),
            cost_usd: None,
            duration_ms: Some(5000),
            created_at: now.clone(),
        };
        let id = db.insert_summary(&summary).unwrap();
        assert!(id > 0);

        let summaries = db.get_summaries_by_session("sess-2").unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].template, "MeetingMinutes");

        db.update_summary_content(id, "## 수정된 요약\n- 내용 변경")
            .unwrap();
        let updated = db.get_summaries_by_session("sess-2").unwrap();
        assert!(updated[0].content.contains("수정된 요약"));
    }
}
