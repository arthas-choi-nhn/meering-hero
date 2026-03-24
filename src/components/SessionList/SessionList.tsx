import type { Session } from "../../lib/tauri-commands";

interface Props {
  sessions: Session[];
  currentId: string | null;
  recordingId: string | null;
  onSelect: (id: string) => void;
}

function formatDate(iso: string): string {
  const d = new Date(iso);
  const month = d.getMonth() + 1;
  const day = d.getDate();
  const hours = String(d.getHours()).padStart(2, "0");
  const mins = String(d.getMinutes()).padStart(2, "0");
  return `${month}/${day} ${hours}:${mins}`;
}

export default function SessionList({ sessions, currentId, recordingId, onSelect }: Props) {
  return (
    <div className="w-64 border-r border-gray-700 flex flex-col bg-gray-900/50">
      <div className="p-3 border-b border-gray-700">
        <h2 className="text-sm font-semibold text-gray-300">회의 목록</h2>
      </div>
      <div className="flex-1 overflow-y-auto">
        {sessions.length === 0 ? (
          <p className="text-xs text-gray-500 p-3">아직 회의가 없습니다</p>
        ) : (
          sessions.map((session) => (
            <button
              key={session.id}
              onClick={() => onSelect(session.id)}
              className={`w-full text-left p-3 border-b border-gray-800 hover:bg-gray-800/50 transition-colors ${
                currentId === session.id ? "bg-gray-800" : ""
              }`}
            >
              <div className="flex items-center justify-between gap-2">
                <span className="text-sm text-gray-200 truncate">{session.title}</span>
                {recordingId === session.id && (
                  <span className="text-xs bg-red-500/20 text-red-400 px-1.5 py-0.5 rounded flex items-center gap-1">
                    <span className="w-1.5 h-1.5 bg-red-400 rounded-full animate-pulse" />
                    녹음중
                  </span>
                )}
              </div>
              <div className="text-xs text-gray-500 mt-1">
                {formatDate(session.created_at)}
                {session.duration_secs != null && session.duration_secs > 0 && ` · ${Math.round(session.duration_secs / 60)}분`}
              </div>
            </button>
          ))
        )}
      </div>
    </div>
  );
}
