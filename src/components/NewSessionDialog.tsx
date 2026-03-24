import { useState } from "react";

interface Props {
  open: boolean;
  onClose: () => void;
  onStart: (title: string, participants?: string[], contextHint?: string) => void;
}

export default function NewSessionDialog({ open, onClose, onStart }: Props) {
  const [title, setTitle] = useState("");
  const [participants, setParticipants] = useState("");
  const [contextHint, setContextHint] = useState("");

  if (!open) return null;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const parts = participants.trim()
      ? participants.split(",").map((p) => p.trim())
      : undefined;
    const hint = contextHint.trim() || undefined;
    onStart(title || "새 회의", parts, hint);
    setTitle("");
    setParticipants("");
    setContextHint("");
    onClose();
  };

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
      <form
        onSubmit={handleSubmit}
        className="bg-gray-800 rounded-xl p-6 w-96 space-y-4 shadow-2xl"
      >
        <h2 className="text-lg font-semibold text-white">새 회의 추가</h2>

        <div>
          <label className="block text-sm text-gray-400 mb-1">회의 제목</label>
          <input
            type="text"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder="주간 DLS 인프라 회의"
            className="w-full bg-gray-700 text-white rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
            autoFocus
          />
        </div>

        <div>
          <label className="block text-sm text-gray-400 mb-1">참석자 (쉼표 구분)</label>
          <input
            type="text"
            value={participants}
            onChange={(e) => setParticipants(e.target.value)}
            placeholder="arthas, member1, member2"
            className="w-full bg-gray-700 text-white rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
        </div>

        <div>
          <label className="block text-sm text-gray-400 mb-1">맥락 힌트 (전문용어)</label>
          <input
            type="text"
            value={contextHint}
            onChange={(e) => setContextHint(e.target.value)}
            placeholder="HAProxy, CrowdSec, ClickHouse"
            className="w-full bg-gray-700 text-white rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
        </div>

        <div className="flex gap-2 justify-end pt-2">
          <button
            type="button"
            onClick={onClose}
            className="px-4 py-2 text-sm text-gray-400 hover:text-gray-200 transition-colors"
          >
            취소
          </button>
          <button
            type="submit"
            className="px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white rounded-lg text-sm font-medium transition-colors"
          >
            추가
          </button>
        </div>
      </form>
    </div>
  );
}
