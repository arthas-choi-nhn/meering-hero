use serde::Serialize;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use super::templates;

#[derive(Debug, Clone, Serialize)]
pub struct ClaudeStatus {
    pub available: bool,
    pub path: Option<String>,
    pub error: Option<String>,
}

/// Find the claude CLI binary path.
pub fn find_claude_binary() -> Option<PathBuf> {
    let candidates = [
        "/Users/prismsoft/.local/bin/claude",
        "/usr/local/bin/claude",
        "/opt/homebrew/bin/claude",
    ];

    for candidate in &candidates {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return Some(path);
        }
    }

    // Try ~/.npm-global/bin/claude
    if let Some(home) = dirs::home_dir() {
        let npm_global = home.join(".npm-global/bin/claude");
        if npm_global.exists() {
            return Some(npm_global);
        }
        // Try ~/.claude/local/bin/claude
        let claude_local = home.join(".claude/local/bin/claude");
        if claude_local.exists() {
            return Some(claude_local);
        }
    }

    // Fallback: which claude
    std::process::Command::new("which")
        .arg("claude")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(PathBuf::from(path));
                }
            }
            None
        })
}

/// Check if Claude CLI is available.
pub fn check_status() -> ClaudeStatus {
    match find_claude_binary() {
        Some(path) => ClaudeStatus {
            available: true,
            path: Some(path.to_string_lossy().to_string()),
            error: None,
        },
        None => ClaudeStatus {
            available: false,
            path: None,
            error: Some("Claude CLI를 찾을 수 없습니다. Claude Code가 설치되어 있는지 확인하세요.".to_string()),
        },
    }
}

/// Summarize transcript using Claude Code CLI.
pub async fn summarize(transcript: &str, template: &str, custom_system_prompt: Option<&str>) -> Result<String, String> {
    let claude_path = find_claude_binary()
        .ok_or("Claude CLI를 찾을 수 없습니다")?;

    let default_prompt = match template {
        "MeetingMinutes" => templates::meeting_minutes_system_prompt(),
        _ => templates::meeting_minutes_system_prompt(),
    };
    let system_prompt = custom_system_prompt.unwrap_or(default_prompt);
    let user_prompt = templates::meeting_minutes_prompt(transcript);

    // For large transcripts (>100KB), pipe via stdin
    let use_stdin = user_prompt.len() > 100_000;

    let work_dir = std::env::temp_dir().join("meering-hero-claude");
    std::fs::create_dir_all(&work_dir).ok();

    // Create an empty MCP config to disable all MCP servers
    let empty_mcp_config = work_dir.join("empty-mcp.json");
    std::fs::write(&empty_mcp_config, r#"{"mcpServers": {}}"#).ok();

    let mut cmd = Command::new(&claude_path);
    cmd.current_dir(&work_dir);
    cmd.arg("--strict-mcp-config");
    cmd.arg("--mcp-config").arg(&empty_mcp_config);
    cmd.arg("-p");

    if use_stdin {
        cmd.arg("-"); // read from stdin
    } else {
        cmd.arg(&user_prompt);
    }

    cmd.arg("--output-format").arg("text");
    cmd.arg("--max-turns").arg("1");

    if !system_prompt.is_empty() {
        cmd.arg("--system-prompt").arg(system_prompt);
    }

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    if use_stdin {
        cmd.stdin(Stdio::piped());
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn claude process: {}", e))?;

    if use_stdin {
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(user_prompt.as_bytes())
                .await
                .map_err(|e| format!("Failed to write to stdin: {}", e))?;
        }
    }

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        child.wait_with_output(),
    )
    .await
    .map_err(|_| "Claude CLI 응답 시간 초과 (120초)")?
    .map_err(|e| format!("Failed to get output: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("exit code: {}", output.status)
        };
        return Err(format!("Claude CLI 오류: {}", detail));
    }

    let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if result.is_empty() {
        return Err("Claude CLI가 빈 응답을 반환했습니다".to_string());
    }

    Ok(result)
}
