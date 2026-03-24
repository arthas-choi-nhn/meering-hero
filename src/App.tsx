import { useState, useCallback } from "react";
import { useSession } from "./hooks/useSession";
import { useTranscript } from "./hooks/useTranscript";
import {
  createSession,
  startRecording,
  stopRecording,
  pauseRecording,
  resumeRecording,
  getSegments,
  getSummaries,
  type Summary,
} from "./lib/tauri-commands";
import SessionList from "./components/SessionList/SessionList";
import LiveTranscript from "./components/LiveTranscript/LiveTranscript";
import RecordingControls from "./components/RecordingControls";
import NewSessionDialog from "./components/NewSessionDialog";
import SummaryPanel from "./components/SummaryPanel/SummaryPanel";
import NoteEditor from "./components/NoteEditor/NoteEditor";
import Settings from "./components/Settings/Settings";
import DoorayExportDialog from "./components/DoorayExportDialog";
import ResizeHandle from "./components/ResizeHandle";

function App() {
  const { sessions, currentSession, setCurrentSession, selectSession, refresh } =
    useSession();
  const [isPaused, setIsPaused] = useState(false);
  const [showNewSession, setShowNewSession] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  // 녹음 중인 세션 ID를 추적 (null이면 녹음 안 하는 중)
  const [recordingSessionId, setRecordingSessionId] = useState<string | null>(null);
  const { lines, clear } = useTranscript(recordingSessionId);
  const [viewLines, setViewLines] = useState<
    { text: string; startMs: number; endMs: number; isFinal: boolean }[]
  >([]);

  const [rightPanelWidth, setRightPanelWidth] = useState(320);
  const [showExport, setShowExport] = useState(false);
  const [latestSummary, setLatestSummary] = useState<Summary | null>(null);

  const isRecording = recordingSessionId !== null;
  const isCurrentRecording = isRecording && currentSession?.id === recordingSessionId;

  const handlePanelResize = useCallback((delta: number) => {
    setRightPanelWidth((w) => Math.max(200, Math.min(600, w + delta)));
  }, []);

  // 회의 추가 (녹음 시작 없이)
  const handleCreateSession = useCallback(
    async (title: string, participants?: string[], contextHint?: string) => {
      try {
        const session = await createSession(title, participants, contextHint);
        setCurrentSession(session);
        setViewLines([]);
        await refresh();
      } catch (e) {
        alert(`세션 생성 실패: ${e}`);
      }
    },
    [setCurrentSession, refresh],
  );

  // 현재 세션에 대해 녹음 시작
  const handleStartRecording = useCallback(
    async (deviceName?: string) => {
      if (!currentSession) return;
      if (isRecording) {
        alert("이미 녹음 중인 세션이 있습니다. 먼저 녹음을 중지해주세요.");
        return;
      }
      try {
        const existingSegs = await getSegments(currentSession.id).catch(() => []);
        setViewLines(
          existingSegs.map((s) => ({
            text: s.text,
            startMs: s.start_ms,
            endMs: s.end_ms,
            isFinal: s.is_final,
          })),
        );
        clear();
        await startRecording(currentSession.id, deviceName);
        setRecordingSessionId(currentSession.id);
        setIsPaused(false);
        await refresh();
      } catch (e) {
        // 녹음 시작 실패 시 세션 상태 복구
        try {
          await stopRecording(currentSession.id);
        } catch { /* ignore */ }
        setRecordingSessionId(null);
        await refresh();
        alert(`녹음 시작 실패: ${e}`);
      }
    },
    [currentSession, isRecording, clear, refresh],
  );

  const handleStopRecording = useCallback(async () => {
    if (!recordingSessionId) return;
    try {
      const session = await stopRecording(recordingSessionId);
      // 현재 보고 있는 세션이 녹음 중이던 세션이면 갱신
      if (currentSession?.id === recordingSessionId) {
        setCurrentSession(session);
        const segs = await getSegments(recordingSessionId).catch(() => []);
        setViewLines(
          segs.map((s) => ({
            text: s.text,
            startMs: s.start_ms,
            endMs: s.end_ms,
            isFinal: s.is_final,
          })),
        );
      }
      setRecordingSessionId(null);
      setIsPaused(false);
      await refresh();
    } catch (e) {
      alert(`녹음 종료 실패: ${e}`);
    }
  }, [recordingSessionId, currentSession, setCurrentSession, refresh]);

  const handlePause = useCallback(async () => {
    await pauseRecording();
    setIsPaused(true);
  }, []);

  const handleResume = useCallback(async () => {
    await resumeRecording();
    setIsPaused(false);
  }, []);

  const handleSelectSession = useCallback(
    async (id: string) => {
      // 녹음 중인 세션을 떠나도 녹음은 계속됨 (중지 안 함)
      await selectSession(id);
      try {
        const segs = await getSegments(id);
        setViewLines(
          segs.map((s) => ({
            text: s.text,
            startMs: s.start_ms,
            endMs: s.end_ms,
            isFinal: s.is_final,
          })),
        );
      } catch {
        setViewLines([]);
      }
      getSummaries(id)
        .then((sums) => setLatestSummary(sums[0] ?? null))
        .catch(() => setLatestSummary(null));
    },
    [selectSession],
  );

  // 현재 세션이 녹음 중: 기존 + 실시간, 아니면: 저장된 세그먼트만
  const displayLines = isCurrentRecording ? [...viewLines, ...lines] : viewLines;

  return (
    <div className="flex h-screen bg-gray-900 text-white">
      <SessionList
        sessions={sessions}
        currentId={currentSession?.id ?? null}
        recordingId={recordingSessionId}
        onSelect={handleSelectSession}
      />

      <div className="flex-1 flex flex-col min-w-0">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-gray-700">
          <div>
            <h1 className="text-lg font-semibold">
              {currentSession?.title ?? "Meering Hero"}
            </h1>
            {currentSession?.participants && (
              <p className="text-xs text-gray-500">
                {JSON.parse(currentSession.participants).join(", ")}
              </p>
            )}
          </div>
          <div className="flex items-center gap-2">
            {!isRecording && (
              <button
                onClick={() => setShowNewSession(true)}
                className="px-4 py-1.5 bg-blue-600 hover:bg-blue-500 text-sm text-white rounded-lg transition-colors"
              >
                + 새 회의
              </button>
            )}
            <button
              onClick={() => setShowSettings(true)}
              className="p-1.5 text-gray-400 hover:text-white transition-colors"
              title="설정"
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>
            </button>
          </div>
        </div>

        {/* Main content: transcript + right panel */}
        <div className="flex-1 flex overflow-hidden">
          <div className="flex-1 flex flex-col min-w-0">
            <LiveTranscript lines={displayLines} isRecording={isCurrentRecording} />
          </div>

          {currentSession && !isCurrentRecording && currentSession.status !== "created" && (
            <>
              <ResizeHandle onResize={handlePanelResize} />
              <div
                className="flex flex-col overflow-y-auto shrink-0"
                style={{ width: rightPanelWidth }}
              >
                <SummaryPanel sessionId={currentSession.id} />
                <NoteEditor
                  sessionId={currentSession.id}
                  initialNotes={currentSession.notes ?? ""}
                />
                <div className="border-t border-gray-700 p-4">
                  <button
                    onClick={() => setShowExport(true)}
                    className="w-full px-4 py-2 bg-teal-600 hover:bg-teal-500 text-white text-sm rounded-lg transition-colors flex items-center justify-center gap-2"
                  >
                    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/></svg>
                    Dooray Wiki로 내보내기
                  </button>
                </div>
              </div>
            </>
          )}
        </div>

        {/* Controls */}
        <RecordingControls
          isRecording={isCurrentRecording}
          isPaused={isPaused}
          onStart={() => handleStartRecording()}
          onStop={handleStopRecording}
          onPause={handlePause}
          onResume={handleResume}
          startedAt={currentSession?.started_at ?? null}
          canStart={!!currentSession && !isRecording}
        />

        {/* 다른 세션을 보고 있지만 녹음 중인 세션이 있을 때 안내 */}
        {isRecording && !isCurrentRecording && (
          <div className="px-4 py-2 bg-red-900/30 border-t border-red-800/50 flex items-center justify-between">
            <span className="text-xs text-red-300">
              다른 세션에서 녹음 중입니다
            </span>
            <button
              onClick={() => handleSelectSession(recordingSessionId!)}
              className="text-xs text-red-300 hover:text-white underline"
            >
              녹음 중인 세션으로 이동
            </button>
          </div>
        )}
      </div>

      <NewSessionDialog
        open={showNewSession}
        onClose={() => setShowNewSession(false)}
        onStart={handleCreateSession}
      />

      <Settings
        open={showSettings}
        onClose={() => setShowSettings(false)}
      />

      {currentSession && (
        <DoorayExportDialog
          open={showExport}
          onClose={() => setShowExport(false)}
          sessionId={currentSession.id}
          sessionTitle={currentSession.title}
          latestSummary={latestSummary}
        />
      )}
    </div>
  );
}

export default App;
