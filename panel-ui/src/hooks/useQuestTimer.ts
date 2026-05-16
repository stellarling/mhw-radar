import { useState, useEffect, useRef } from "react";
import type { PanelStatus } from "../types";

export function useQuestTimer(status: PanelStatus | null) {
  const [displayTime, setDisplayTime] = useState<number | null>(null);
  const timerBaseRef = useRef<{ value: number; time: number } | null>(null);

  useEffect(() => {
    if (status?.in_quest && status.quest_elapsed_ms != null) {
      timerBaseRef.current = { value: status.quest_elapsed_ms, time: Date.now() };
      setDisplayTime(status.quest_elapsed_ms);
    } else {
      timerBaseRef.current = null;
      setDisplayTime(status?.quest_elapsed_ms ?? null);
    }
  }, [status?.quest_elapsed_ms, status?.in_quest]);

  useEffect(() => {
    const id = setInterval(() => {
      if (timerBaseRef.current) {
        const elapsed = timerBaseRef.current.value + (Date.now() - timerBaseRef.current.time);
        setDisplayTime(elapsed);
      }
    }, 50);
    return () => clearInterval(id);
  }, []);

  return { displayTime };
}
