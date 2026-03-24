use crate::models::{Segment, Session};

/// Render a Wiki page markdown from session data.
pub fn render_wiki_page(
    session: &Session,
    segments: &[Segment],
    summary: Option<&str>,
    notes: Option<&str>,
) -> String {
    let mut md = String::new();

    // Title
    md.push_str(&format!("# {}\n\n", session.title));

    // Metadata
    md.push_str(&format!("**일시**: {}\n", format_datetime(&session.started_at)));
    if let Some(ref participants) = session.participants {
        if let Ok(names) = serde_json::from_str::<Vec<String>>(participants) {
            md.push_str(&format!("**참석자**: {}\n", names.join(", ")));
        }
    }
    if let Some(secs) = session.duration_secs {
        md.push_str(&format!("**소요시간**: {}분\n", secs / 60));
    }

    md.push_str("\n---\n\n");

    // Summary
    if let Some(summary_content) = summary {
        md.push_str("## 요약\n\n");
        md.push_str(summary_content);
        md.push_str("\n\n---\n\n");
    }

    // Notes
    if let Some(notes_content) = notes {
        if !notes_content.trim().is_empty() {
            md.push_str("## 노트\n\n");
            md.push_str(notes_content);
            md.push_str("\n\n---\n\n");
        }
    }

    // Full transcript in collapsible section
    if !segments.is_empty() {
        md.push_str("<details>\n<summary>전체 전사 내용 (펼치기)</summary>\n\n");
        for segment in segments {
            let time = format_ms(segment.start_ms);
            md.push_str(&format!("[{}] {}\n", time, segment.text.trim()));
        }
        md.push_str("\n</details>\n");
    }

    md
}

fn format_datetime(iso: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(iso)
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|_| iso.to_string())
}

fn format_ms(ms: i64) -> String {
    let total_secs = ms / 1000;
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    format!("{:02}:{:02}:{:02}", hours, mins, secs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Segment, Session};

    #[test]
    fn test_render_wiki_page() {
        let session = Session {
            id: "test".to_string(),
            title: "주간 DLS 인프라 회의".to_string(),
            started_at: "2026-03-23T14:00:00+09:00".to_string(),
            ended_at: None,
            duration_secs: Some(2700),
            participants: Some(r#"["arthas","member1"]"#.to_string()),
            context_hint: None,
            notes: Some("중요 사항 메모".to_string()),
            status: "completed".to_string(),
            audio_path: None,
            model_used: None,
            created_at: "2026-03-23T14:00:00+09:00".to_string(),
            updated_at: "2026-03-23T14:45:00+09:00".to_string(),
        };

        let segments = vec![Segment {
            id: 1,
            session_id: "test".to_string(),
            text: "HAProxy 보안 룰 업데이트 건으로 논의합니다".to_string(),
            start_ms: 12000,
            end_ms: 18000,
            is_final: true,
            speaker: None,
            created_at: "2026-03-23T14:00:12+09:00".to_string(),
        }];

        let md = render_wiki_page(
            &session,
            &segments,
            Some("## 요약\n- 보안 업데이트 논의"),
            session.notes.as_deref(),
        );

        assert!(md.contains("# 주간 DLS 인프라 회의"));
        assert!(md.contains("arthas, member1"));
        assert!(md.contains("45분"));
        assert!(md.contains("보안 업데이트 논의"));
        assert!(md.contains("중요 사항 메모"));
        assert!(md.contains("<details>"));
        assert!(md.contains("[00:00:12]"));
    }
}
