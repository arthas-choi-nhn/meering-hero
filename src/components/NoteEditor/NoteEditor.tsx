import { useState, useEffect, useRef, useCallback } from "react";
import { updateSessionNotes } from "../../lib/tauri-commands";

interface Props {
  sessionId: string;
  initialNotes: string;
}

export default function NoteEditor({ sessionId, initialNotes }: Props) {
  const [notes, setNotes] = useState(initialNotes);
  const [saving, setSaving] = useState(false);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    setNotes(initialNotes);
  }, [initialNotes]);

  const save = useCallback(
    async (value: string) => {
      setSaving(true);
      try {
        await updateSessionNotes(sessionId, value);
      } catch (e) {
        console.error("Failed to save notes:", e);
      } finally {
        setSaving(false);
      }
    },
    [sessionId],
  );

  const handleChange = (value: string) => {
    setNotes(value);
    if (timeoutRef.current) clearTimeout(timeoutRef.current);
    timeoutRef.current = setTimeout(() => save(value), 1000);
  };

  return (
    <div className="border-t border-gray-700 p-4">
      <div className="flex items-center justify-between mb-2">
        <h3 className="text-sm font-semibold text-gray-300">노트</h3>
        {saving && <span className="text-xs text-gray-500">저장 중...</span>}
      </div>
      <textarea
        value={notes}
        onChange={(e) => handleChange(e.target.value)}
        placeholder="회의 관련 메모를 입력하세요..."
        className="w-full h-32 bg-gray-800 text-sm text-gray-200 rounded-lg p-3 focus:outline-none focus:ring-2 focus:ring-blue-500 resize-y"
      />
    </div>
  );
}
