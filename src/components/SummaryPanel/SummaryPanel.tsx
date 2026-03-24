import { useState, useEffect } from "react";
import {
  summarizeSession,
  getSummaries,
  updateSummaryContent,
  checkClaudeStatus,
  type Summary,
} from "../../lib/tauri-commands";

interface Props {
  sessionId: string;
}

export default function SummaryPanel({ sessionId }: Props) {
  const [summaries, setSummaries] = useState<Summary[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [editing, setEditing] = useState(false);
  const [editContent, setEditContent] = useState("");
  const [claudeAvailable, setClaudeAvailable] = useState<boolean | null>(null);

  useEffect(() => {
    checkClaudeStatus().then((s) => setClaudeAvailable(s.available));
    getSummaries(sessionId).then(setSummaries).catch(console.error);
  }, [sessionId]);

  const handleSummarize = async () => {
    setLoading(true);
    setError(null);
    try {
      const summary = await summarizeSession(sessionId, "MeetingMinutes");
      setSummaries((prev) => [summary, ...prev]);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleSave = async (summary: Summary) => {
    try {
      await updateSummaryContent(summary.id, editContent);
      setSummaries((prev) =>
        prev.map((s) => (s.id === summary.id ? { ...s, content: editContent } : s)),
      );
      setEditing(false);
    } catch (e) {
      setError(String(e));
    }
  };

  const latest = summaries[0];

  return (
    <div className="border-t border-gray-700 p-4">
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-sm font-semibold text-gray-300">요약</h3>
        <button
          onClick={handleSummarize}
          disabled={loading || claudeAvailable === false}
          className="px-3 py-1 text-xs bg-purple-600 hover:bg-purple-500 disabled:bg-gray-600 disabled:cursor-not-allowed text-white rounded transition-colors"
        >
          {loading ? "요약 생성 중..." : "Claude로 요약"}
        </button>
      </div>

      {claudeAvailable === false && (
        <p className="text-xs text-yellow-500 mb-2">
          Claude CLI를 찾을 수 없습니다. Claude Code를 설치하세요.
        </p>
      )}

      {error && (
        <p className="text-xs text-red-400 mb-2">{error}</p>
      )}

      {latest && (
        <div className="bg-gray-800 rounded-lg p-3">
          {editing ? (
            <div>
              <textarea
                value={editContent}
                onChange={(e) => setEditContent(e.target.value)}
                className="w-full h-48 bg-gray-700 text-sm text-gray-200 rounded p-2 focus:outline-none focus:ring-2 focus:ring-purple-500 resize-y"
              />
              <div className="flex gap-2 mt-2">
                <button
                  onClick={() => handleSave(latest)}
                  className="px-3 py-1 text-xs bg-green-600 hover:bg-green-500 text-white rounded"
                >
                  저장
                </button>
                <button
                  onClick={() => setEditing(false)}
                  className="px-3 py-1 text-xs text-gray-400 hover:text-gray-200"
                >
                  취소
                </button>
              </div>
            </div>
          ) : (
            <div>
              <div className="text-sm text-gray-200 whitespace-pre-wrap max-h-64 overflow-y-auto">
                {latest.content}
              </div>
              <button
                onClick={() => {
                  setEditContent(latest.content);
                  setEditing(true);
                }}
                className="mt-2 text-xs text-gray-400 hover:text-gray-200"
              >
                편집
              </button>
            </div>
          )}
          <div className="text-xs text-gray-500 mt-2">
            {latest.provider} · {latest.duration_ms ? `${(latest.duration_ms / 1000).toFixed(1)}초` : ""}
          </div>
        </div>
      )}
    </div>
  );
}
