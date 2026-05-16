import { useCallback } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { API } from "../constants";
import { readApiError } from "../utils/api";
import type { LogEntry, LogResponse } from "../types";

export function useLogExport(logEntries: LogEntry[], currentRound: number) {
  const saveLogFile = useCallback(async (text: string, defaultName?: string) => {
    let defaultPath = defaultName ?? `mhw-radar-${new Date().toISOString().slice(0, 10)}.txt`;

    try {
      const res = await fetch(`${API}/api/desktop-path`);
      if (res.ok) {
        const data = await res.json();
        if (data.path) {
          defaultPath = `${data.path}\\${defaultPath}`;
        }
      }
    } catch { /* ignore */ }

    const filePath = await save({
      defaultPath,
      filters: [{ name: "文本文件", extensions: ["txt"] }],
    });

    if (!filePath) return;

    const res = await fetch(`${API}/api/logs/export`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ path: filePath, content: text }),
    });

    if (!res.ok) {
      throw new Error(await readApiError(res));
    }
  }, []);

  const exportCurrentPage = useCallback(async () => {
    if (logEntries.length === 0) return;
    try {
      const text = logEntries
        .map((e) => `[${e.timestamp}] [${e.level}] ${e.message}`)
        .join("\r\n") + "\r\n";
      const roundLabel = String(currentRound + 1).padStart(3, "0");
      await saveLogFile(text, `mhw-radar-round-${roundLabel}-${new Date().toISOString().slice(0, 10)}.txt`);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      window.alert(`导出本页失败：${message}`);
    }
  }, [currentRound, logEntries, saveLogFile]);

  const exportAllRounds = useCallback(async () => {
    try {
      const res = await fetch(`${API}/api/logs`);
      if (!res.ok) throw new Error(await readApiError(res));
      const data: LogResponse = await res.json();
      if (data.entries.length === 0) return;
      const text = data.entries
        .map((e) => `[${e.timestamp}] [${e.level}] ${e.message}`)
        .join("\r\n") + "\r\n";
      await saveLogFile(text, `mhw-radar-all-${new Date().toISOString().slice(0, 10)}.txt`);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      window.alert(`导出全部失败：${message}`);
    }
  }, [saveLogFile]);

  return { exportCurrentPage, exportAllRounds };
}
