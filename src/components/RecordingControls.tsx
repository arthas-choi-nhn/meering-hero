import { useState, useEffect } from "react";

interface Props {
  isRecording: boolean;
  isPaused: boolean;
  onStart: () => void;
  onStop: () => void;
  onPause: () => void;
  onResume: () => void;
  startedAt: string | null;
  canStart: boolean;
}

function formatDuration(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  if (h > 0) {
    return `${String(h).padStart(2, "0")}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  }
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}

export default function RecordingControls({
  isRecording,
  isPaused,
  onStart,
  onStop,
  onPause,
  onResume,
  startedAt,
  canStart,
}: Props) {
  const [elapsed, setElapsed] = useState(0);

  useEffect(() => {
    if (!isRecording || isPaused || !startedAt) {
      return;
    }
    const interval = setInterval(() => {
      const start = new Date(startedAt).getTime();
      const now = Date.now();
      setElapsed(Math.floor((now - start) / 1000));
    }, 1000);
    return () => clearInterval(interval);
  }, [isRecording, isPaused, startedAt]);

  useEffect(() => {
    if (!isRecording) setElapsed(0);
  }, [isRecording]);

  return (
    <div className="flex items-center gap-4 p-4 border-t border-gray-700">
      {isRecording && (
        <div className="flex items-center gap-2">
          <div className={`w-3 h-3 rounded-full ${isPaused ? "bg-yellow-500" : "bg-red-500 animate-pulse"}`} />
          <span className="text-sm font-mono text-gray-300">
            {formatDuration(elapsed)}
          </span>
        </div>
      )}

      <div className="flex-1" />

      {!isRecording ? (
        <button
          onClick={onStart}
          disabled={!canStart}
          className="px-6 py-2 bg-blue-600 hover:bg-blue-500 disabled:bg-gray-600 disabled:cursor-not-allowed text-white rounded-lg font-medium transition-colors"
        >
          녹음 시작
        </button>
      ) : (
        <div className="flex gap-2">
          {isPaused ? (
            <button
              onClick={onResume}
              className="px-4 py-2 bg-green-600 hover:bg-green-500 text-white rounded-lg text-sm transition-colors"
            >
              재개
            </button>
          ) : (
            <button
              onClick={onPause}
              className="px-4 py-2 bg-yellow-600 hover:bg-yellow-500 text-white rounded-lg text-sm transition-colors"
            >
              일시정지
            </button>
          )}
          <button
            onClick={onStop}
            className="px-4 py-2 bg-red-600 hover:bg-red-500 text-white rounded-lg text-sm transition-colors"
          >
            녹음 종료
          </button>
        </div>
      )}
    </div>
  );
}
