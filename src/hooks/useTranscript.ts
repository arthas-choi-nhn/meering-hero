import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import type { SttEvent } from "../lib/tauri-commands";

export interface TranscriptLine {
  text: string;
  startMs: number;
  endMs: number;
  isFinal: boolean;
}

export function useTranscript(sessionId: string | null) {
  const [lines, setLines] = useState<TranscriptLine[]>([]);

  useEffect(() => {
    if (!sessionId) return;

    const unlistenFinal = listen<SttEvent>("stt:final", (event) => {
      if (event.payload.session_id !== sessionId) return;
      setLines((prev) => [
        ...prev,
        {
          text: event.payload.text,
          startMs: event.payload.start_ms,
          endMs: event.payload.end_ms,
          isFinal: true,
        },
      ]);
    });

    return () => {
      unlistenFinal.then((fn) => fn());
    };
  }, [sessionId]);

  const clear = useCallback(() => setLines([]), []);

  return { lines, clear };
}
