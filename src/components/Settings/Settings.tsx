import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  loadSettings,
  saveSettings,
  getFullModelStatus,
  downloadModel,
  checkClaudeStatus,
  getAudioDevices,
  type AppSettings,
  type FullModelStatus,
  type DownloadProgress,
  type ClaudeStatus,
  type AudioDevice,
} from "../../lib/tauri-commands";

const DEFAULT_SUMMARY_PROMPT = `당신은 회의록 작성 전문가입니다. 제공된 회의 전사 내용을 분석하여 구조화된 한국어 회의록을 작성하세요.

다음 형식으로 마크다운 출력을 생성하세요:

## 요약

### 회의 개요
- (1~3줄 요약)

### 논의 내용
- (안건별 정리)

### 결정 사항
- (확정된 사항)

### 액션아이템
- [ ] 담당자: 작업 내용 (기한)

규칙:
- 전문용어는 원어 그대로 유지 (HAProxy, CrowdSec 등)
- 핵심 내용만 간결하게 정리
- 액션아이템에는 담당자와 기한을 명시
- 논의 내용은 주제별로 그룹핑`;

interface Props {
  open: boolean;
  onClose: () => void;
}

export default function Settings({ open, onClose }: Props) {
  const [settings, setSettings] = useState<AppSettings>({
    dooray_base_url: null,
    dooray_token: null,
    stt_model: null,
    audio_device: null,
    vad_mode: null,
    summary_prompt: null,
  });
  const [modelStatus, setModelStatus] = useState<FullModelStatus | null>(null);
  const [claudeStatus, setClaudeStatus] = useState<ClaudeStatus | null>(null);
  const [audioDevices, setAudioDevices] = useState<AudioDevice[]>([]);
  const [downloading, setDownloading] = useState<string | null>(null);
  const [progress, setProgress] = useState<DownloadProgress | null>(null);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    if (!open) return;
    loadSettings().then(setSettings).catch(console.error);
    getFullModelStatus().then(setModelStatus).catch(console.error);
    checkClaudeStatus().then(setClaudeStatus).catch(console.error);
    getAudioDevices().then(setAudioDevices).catch(console.error);
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const unlisten = listen<DownloadProgress>("download:progress", (e) => {
      setProgress(e.payload);
      if (e.payload.done) {
        setDownloading(null);
        setProgress(null);
        getFullModelStatus().then(setModelStatus);
      }
    });
    return () => { unlisten.then((fn) => fn()); };
  }, [open]);

  if (!open) return null;

  const handleSave = async () => {
    try {
      await saveSettings(settings);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      alert(`저장 실패: ${e}`);
    }
  };

  const handleDownload = async (model: string) => {
    setDownloading(model);
    setProgress(null);
    try {
      await downloadModel(model);
    } catch (e) {
      alert(`다운로드 실패: ${e}`);
      setDownloading(null);
      setProgress(null);
    }
  };

  const formatBytes = (bytes: number) => {
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
    return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
  };

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
      <div className="bg-gray-800 rounded-xl p-6 w-[520px] max-h-[80vh] overflow-y-auto shadow-2xl">
        <div className="flex items-center justify-between mb-6">
          <h2 className="text-lg font-semibold text-white">설정</h2>
          <button onClick={onClose} className="text-gray-400 hover:text-white text-xl">&times;</button>
        </div>

        {/* Models Section */}
        <section className="mb-6">
          <h3 className="text-sm font-semibold text-gray-300 mb-3">모델 관리</h3>

          {modelStatus && (
            <div className="space-y-2">
              {/* VAD Model */}
              <div className="flex items-center justify-between bg-gray-700/50 rounded-lg p-3">
                <div>
                  <p className="text-sm text-gray-200">Silero VAD</p>
                  <p className="text-xs text-gray-500">음성 활동 감지 (~2MB)</p>
                </div>
                {modelStatus.vad_downloaded ? (
                  <span className="text-xs text-green-400 bg-green-500/10 px-2 py-1 rounded">설치됨</span>
                ) : downloading === "vad" ? (
                  <ProgressBar progress={progress} formatBytes={formatBytes} />
                ) : (
                  <button
                    onClick={() => handleDownload("vad")}
                    className="text-xs bg-blue-600 hover:bg-blue-500 text-white px-3 py-1 rounded transition-colors"
                  >
                    다운로드
                  </button>
                )}
              </div>

              {/* STT Models */}
              {modelStatus.stt.models.map((m) => (
                <div key={m.name} className="flex items-center justify-between bg-gray-700/50 rounded-lg p-3">
                  <div>
                    <p className="text-sm text-gray-200">
                      {m.name}
                      {m.size === modelStatus.stt.recommended && (
                        <span className="ml-2 text-xs text-yellow-400">(추천)</span>
                      )}
                    </p>
                    <p className="text-xs text-gray-500">
                      시스템 RAM: {modelStatus.stt.system_ram_gb}GB
                    </p>
                  </div>
                  {m.downloaded ? (
                    <span className="text-xs text-green-400 bg-green-500/10 px-2 py-1 rounded">설치됨</span>
                  ) : downloading === sizeToKey(m.size) ? (
                    <ProgressBar progress={progress} formatBytes={formatBytes} />
                  ) : (
                    <button
                      onClick={() => handleDownload(sizeToKey(m.size))}
                      disabled={downloading !== null}
                      className="text-xs bg-blue-600 hover:bg-blue-500 disabled:bg-gray-600 text-white px-3 py-1 rounded transition-colors"
                    >
                      다운로드
                    </button>
                  )}
                </div>
              ))}
            </div>
          )}

          {/* STT Model Selection */}
          {modelStatus && (
            <div className="mt-3">
              <label className="block text-xs text-gray-400 mb-1">사용할 STT 모델</label>
              <select
                value={settings.stt_model ?? "auto"}
                onChange={(e) =>
                  setSettings({
                    ...settings,
                    stt_model: e.target.value === "auto" ? null : e.target.value,
                  })
                }
                className="w-full bg-gray-700 text-white rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                <option value="auto">
                  자동 (RAM 기반 추천: {modelStatus.stt.recommended})
                </option>
                {modelStatus.stt.models
                  .filter((m) => m.downloaded)
                  .map((m) => (
                    <option key={m.size} value={sizeToKey(m.size)}>
                      {m.name}
                    </option>
                  ))}
              </select>
              {settings.stt_model && !modelStatus.stt.models.find(
                (m) => sizeToKey(m.size) === settings.stt_model && m.downloaded
              ) && (
                <p className="text-xs text-yellow-500 mt-1">
                  선택한 모델이 아직 다운로드되지 않았습니다.
                </p>
              )}
            </div>
          )}
        </section>

        {/* Claude CLI Section */}
        <section className="mb-6">
          <h3 className="text-sm font-semibold text-gray-300 mb-3">Claude Code CLI</h3>
          <div className="bg-gray-700/50 rounded-lg p-3">
            {claudeStatus?.available ? (
              <div className="flex items-center gap-2">
                <span className="text-xs text-green-400 bg-green-500/10 px-2 py-1 rounded">사용 가능</span>
                <span className="text-xs text-gray-500">{claudeStatus.path}</span>
              </div>
            ) : (
              <div>
                <span className="text-xs text-red-400 bg-red-500/10 px-2 py-1 rounded">미설치</span>
                <p className="text-xs text-gray-500 mt-1">{claudeStatus?.error}</p>
              </div>
            )}
          </div>
        </section>

        {/* Summary Prompt Section */}
        <section className="mb-6">
          <div className="flex items-center justify-between mb-3">
            <h3 className="text-sm font-semibold text-gray-300">요약 프롬프트</h3>
            {settings.summary_prompt !== null && (
              <button
                onClick={() => setSettings({ ...settings, summary_prompt: null })}
                className="text-xs text-yellow-400 hover:text-yellow-300 transition-colors"
              >
                기본값으로 초기화
              </button>
            )}
          </div>
          <textarea
            value={settings.summary_prompt ?? DEFAULT_SUMMARY_PROMPT}
            onChange={(e) =>
              setSettings({
                ...settings,
                summary_prompt: e.target.value === DEFAULT_SUMMARY_PROMPT ? null : e.target.value,
              })
            }
            rows={10}
            className="w-full bg-gray-700 text-sm text-gray-200 rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500 resize-y font-mono"
          />
          <p className="text-xs text-gray-500 mt-1">
            {settings.summary_prompt === null ? "기본 프롬프트 사용 중" : "커스텀 프롬프트 사용 중"}
          </p>
        </section>

        {/* Audio Input Section */}
        <section className="mb-6">
          <h3 className="text-sm font-semibold text-gray-300 mb-3">오디오 입력</h3>
          <div className="space-y-3">
            <div>
              <label className="block text-xs text-gray-400 mb-1">마이크</label>
              <select
                value={settings.audio_device ?? ""}
                onChange={(e) =>
                  setSettings({
                    ...settings,
                    audio_device: e.target.value || null,
                  })
                }
                className="w-full bg-gray-700 text-white rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                <option value="">시스템 기본 마이크</option>
                {audioDevices.map((d) => (
                  <option key={d.name} value={d.name}>
                    {d.name} {d.is_default ? "(기본)" : ""}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-xs text-gray-400 mb-1">음성 감지 (VAD)</label>
              <select
                value={settings.vad_mode ?? "energy"}
                onChange={(e) =>
                  setSettings({
                    ...settings,
                    vad_mode: e.target.value === "energy" ? null : e.target.value,
                  })
                }
                className="w-full bg-gray-700 text-white rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                <option value="energy">에너지 기반 (기본, 모델 불필요)</option>
                <option value="silero">Silero VAD (ONNX 모델 필요)</option>
              </select>
              {settings.vad_mode === "silero" && modelStatus && !modelStatus.vad_downloaded && (
                <p className="text-xs text-yellow-500 mt-1">
                  Silero VAD 모델이 설치되지 않았습니다. 위에서 다운로드해주세요.
                </p>
              )}
            </div>
          </div>
        </section>

        {/* Update Section */}
        <section className="mb-6">
          <h3 className="text-sm font-semibold text-gray-300 mb-3">앱 업데이트</h3>
          <UpdateChecker />
        </section>

        {/* Dooray API Section */}
        <section className="mb-6">
          <h3 className="text-sm font-semibold text-gray-300 mb-3">Dooray Wiki API</h3>
          <div className="space-y-3">
            <div>
              <label className="block text-xs text-gray-400 mb-1">Base URL</label>
              <input
                type="text"
                value={settings.dooray_base_url ?? ""}
                onChange={(e) => setSettings({ ...settings, dooray_base_url: e.target.value || null })}
                placeholder="https://your-org.dooray.com"
                className="w-full bg-gray-700 text-white rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
            <div>
              <label className="block text-xs text-gray-400 mb-1">API Token</label>
              <input
                type="password"
                value={settings.dooray_token ?? ""}
                onChange={(e) => setSettings({ ...settings, dooray_token: e.target.value || null })}
                placeholder="dooray-api 토큰을 입력하세요"
                className="w-full bg-gray-700 text-white rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
          </div>
        </section>

        {/* Actions */}
        <div className="flex justify-end gap-2">
          {saved && <span className="text-xs text-green-400 self-center">저장됨!</span>}
          <button
            onClick={onClose}
            className="px-4 py-2 text-sm text-gray-400 hover:text-gray-200 transition-colors"
          >
            닫기
          </button>
          <button
            onClick={handleSave}
            className="px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white text-sm rounded-lg transition-colors"
          >
            저장
          </button>
        </div>
      </div>
    </div>
  );
}

function sizeToKey(size: string): string {
  switch (size) {
    case "Small": return "small";
    case "Medium": return "medium";
    case "Large": return "large";
    default: return size.toLowerCase();
  }
}

function UpdateChecker() {
  const [status, setStatus] = useState<string>("");
  const [updating, setUpdating] = useState(false);
  const [updateProgress, setUpdateProgress] = useState<number>(0);

  const handleCheck = async () => {
    setStatus("업데이트 확인 중...");
    setUpdating(true);
    try {
      const { check } = await import("@tauri-apps/plugin-updater");
      const update = await check();

      if (!update) {
        setStatus("최신 버전입니다.");
        setUpdating(false);
        return;
      }

      setStatus(`v${update.version} 다운로드 중...`);

      let totalLen = 0;
      let downloaded = 0;

      await update.downloadAndInstall((event) => {
        if (event.event === "Started" && event.data.contentLength) {
          totalLen = event.data.contentLength;
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          if (totalLen > 0) {
            setUpdateProgress(Math.round((downloaded / totalLen) * 100));
          }
        } else if (event.event === "Finished") {
          setStatus("설치 완료. 재시작합니다...");
        }
      });

      const { relaunch } = await import("@tauri-apps/plugin-process");
      await relaunch();
    } catch (e) {
      setStatus(`업데이트 실패: ${e}`);
      setUpdating(false);
    }
  };

  return (
    <div className="bg-gray-700/50 rounded-lg p-3">
      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm text-gray-200">현재 버전: v0.1.0</p>
          {status && <p className="text-xs text-gray-400 mt-1">{status}</p>}
        </div>
        <button
          onClick={handleCheck}
          disabled={updating}
          className="text-xs bg-blue-600 hover:bg-blue-500 disabled:bg-gray-600 text-white px-3 py-1 rounded transition-colors"
        >
          {updating ? "확인 중..." : "업데이트 확인"}
        </button>
      </div>
      {updateProgress > 0 && updateProgress < 100 && (
        <div className="mt-2 w-full h-1.5 bg-gray-600 rounded-full overflow-hidden">
          <div
            className="h-full bg-blue-500 transition-all"
            style={{ width: `${updateProgress}%` }}
          />
        </div>
      )}
    </div>
  );
}

function ProgressBar({ progress, formatBytes }: { progress: DownloadProgress | null; formatBytes: (b: number) => string }) {
  if (!progress) {
    return <span className="text-xs text-gray-400 animate-pulse">준비 중...</span>;
  }
  const pct = progress.total_bytes
    ? Math.round((progress.downloaded_bytes / progress.total_bytes) * 100)
    : null;
  return (
    <div className="flex items-center gap-2">
      <div className="w-20 h-1.5 bg-gray-600 rounded-full overflow-hidden">
        <div
          className="h-full bg-blue-500 transition-all"
          style={{ width: `${pct ?? 50}%` }}
        />
      </div>
      <span className="text-xs text-gray-400">
        {formatBytes(progress.downloaded_bytes)}
        {pct !== null && ` (${pct}%)`}
      </span>
    </div>
  );
}
