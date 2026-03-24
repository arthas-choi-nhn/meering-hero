import { useState, useEffect, useCallback } from "react";
import {
  listSessions,
  getSession,
  type Session,
} from "../lib/tauri-commands";

export function useSession() {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [currentSession, setCurrentSession] = useState<Session | null>(null);
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const list = await listSessions();
      setSessions(list);
    } catch (e) {
      console.error("Failed to list sessions:", e);
    } finally {
      setLoading(false);
    }
  }, []);

  const selectSession = useCallback(async (id: string) => {
    try {
      const session = await getSession(id);
      setCurrentSession(session);
    } catch (e) {
      console.error("Failed to get session:", e);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { sessions, currentSession, setCurrentSession, selectSession, refresh, loading };
}
