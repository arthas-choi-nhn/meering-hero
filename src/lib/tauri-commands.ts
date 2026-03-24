import { invoke } from "@tauri-apps/api/core";

// Types matching Rust structs
export interface Session {
  id: string;
  title: string;
  started_at: string;
  ended_at: string | null;
  duration_secs: number | null;
  participants: string | null;
  context_hint: string | null;
  notes: string | null;
  status: string;
  audio_path: string | null;
  model_used: string | null;
  created_at: string;
  updated_at: string;
}

export interface Segment {
  id: number;
  session_id: string;
  text: string;
  start_ms: number;
  end_ms: number;
  is_final: boolean;
  speaker: string | null;
  created_at: string;
}

export interface Summary {
  id: number;
  session_id: string;
  template: string;
  content: string;
  provider: string;
  cost_usd: number | null;
  duration_ms: number | null;
  created_at: string;
}

export interface AudioDevice {
  name: string;
  is_default: boolean;
}

export interface ModelInfo {
  size: string;
  name: string;
  downloaded: boolean;
  path: string | null;
}

export interface ModelStatus {
  recommended: string;
  system_ram_gb: number;
  models: ModelInfo[];
}

export interface SttEvent {
  session_id: string;
  text: string;
  start_ms: number;
  end_ms: number;
  is_partial: boolean;
}

// Session commands
export const createSession = (
  title: string,
  participants?: string[],
  contextHint?: string,
) => invoke<Session>("create_session", { title, participants, contextHint });

export const listSessions = () => invoke<Session[]>("list_sessions");

export const getSession = (id: string) => invoke<Session>("get_session", { id });

export const updateSessionNotes = (id: string, notes: string) =>
  invoke<void>("update_session_notes", { id, notes });

// Audio commands
export const getAudioDevices = () => invoke<AudioDevice[]>("get_audio_devices");

// Model commands
export const getModelStatus = () => invoke<ModelStatus>("get_model_status");

// Recording commands
export const startRecording = (sessionId: string, deviceName?: string) =>
  invoke<void>("start_recording", { sessionId, deviceName });

export const stopRecording = (sessionId: string) =>
  invoke<Session>("stop_recording", { sessionId });

export const pauseRecording = () => invoke<void>("pause_recording");

export const resumeRecording = () => invoke<void>("resume_recording");

// Segment commands
export const getSegments = (sessionId: string) =>
  invoke<Segment[]>("get_segments", { sessionId });

export const getSummaries = (sessionId: string) =>
  invoke<Summary[]>("get_summaries", { sessionId });

export const searchSegments = (query: string) =>
  invoke<Segment[]>("search_segments", { query });

// Summary commands
export interface ClaudeStatus {
  available: boolean;
  path: string | null;
  error: string | null;
}

export const checkClaudeStatus = () =>
  invoke<ClaudeStatus>("check_claude_status");

export const summarizeSession = (sessionId: string, template: string) =>
  invoke<Summary>("summarize_session", { sessionId, template });

export const updateSummaryContent = (summaryId: number, content: string) =>
  invoke<void>("update_summary_content", { summaryId, content });

// Export commands
export interface DoorayConfig {
  base_url: string;
  token: string;
}

export interface ExportResult {
  page_id: string;
  page_url: string;
}

// Settings commands
export interface AppSettings {
  dooray_base_url: string | null;
  dooray_token: string | null;
  /** "small" | "medium" | "large" | null (auto) */
  stt_model: string | null;
  /** Audio input device name, or null (use system default) */
  audio_device: string | null;
  /** "energy" (default) or "silero" */
  vad_mode: string | null;
  /** Custom system prompt for Claude summarization, or null (use default) */
  summary_prompt: string | null;
}

export interface FullModelStatus {
  stt: ModelStatus;
  vad_downloaded: boolean;
  vad_path: string;
}

export interface DownloadProgress {
  model: string;
  downloaded_bytes: number;
  total_bytes: number | null;
  done: boolean;
  error: string | null;
}

export const loadSettings = () => invoke<AppSettings>("load_settings");

export const saveSettings = (settings: AppSettings) =>
  invoke<void>("save_settings", { settings });

export const getFullModelStatus = () =>
  invoke<FullModelStatus>("get_full_model_status");

export const downloadModel = (model: string) =>
  invoke<string>("download_model", { model });

// Dooray API types
export interface DoorayWiki {
  id: string;
  name: string;
}

export interface WikiPage {
  id: string;
  subject: string;
  has_children: boolean;
}

// Dooray browsing commands
export const listDoorayWikis = () =>
  invoke<DoorayWiki[]>("list_dooray_wikis");

export const listDoorayWikiPages = (wikiId: string, parentPageId?: string | null) =>
  invoke<WikiPage[]>("list_dooray_wiki_pages", { wikiId, parentPageId });

// Dooray export commands
export const createDoorayWikiPage = (
  sessionId: string,
  summaryId: number | null,
  wikiId: string,
  parentPageId: string | null,
  title: string,
) =>
  invoke<ExportResult>("create_dooray_wiki_page", {
    sessionId,
    summaryId,
    wikiId,
    parentPageId,
    title,
  });

export const updateDoorayWikiPage = (
  sessionId: string,
  summaryId: number | null,
  wikiId: string,
  pageId: string,
) =>
  invoke<void>("update_dooray_wiki_page", { sessionId, summaryId, wikiId, pageId });
