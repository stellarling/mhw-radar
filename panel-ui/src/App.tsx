import { useState, useEffect, useCallback, useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { API, GITHUB_REPO } from "./constants";
import { useApi } from "./hooks/useApi";
import { PanelView } from "./layouts/PanelView";
import { LogoImage } from "./ui/LogoImage";
import type { LogFilter } from "./features/LogSection";
import { UpdateBanner } from "./features/UpdateBanner";
import { LogAnalysis } from "./features/LogAnalysis";
import { useQuestTimer } from "./hooks/useQuestTimer";
import { useUpdateChecker } from "./hooks/useUpdateChecker";
import { useLogExport } from "./hooks/useLogExport";
import { readApiError } from "./utils/api";
import type { Settings, LogEntry, LogResponse, ConnectionLogEntry, ConnectionLogResponse, BoolKey, PanelStatus, QuestStat, MonsterActionStats } from "./types";

export default function App() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const status = useApi<PanelStatus>("/api/status", 1000);

  const [logEntries, setLogEntries] = useState<LogEntry[]>([]);
  const [currentRound, setCurrentRound] = useState(0);
  const [totalRounds, setTotalRounds] = useState(0);
  const totalRoundsRef = useRef(0);
  const logsRequestIdRef = useRef(0);

  const [connectionLogs, setConnectionLogs] = useState<ConnectionLogEntry[]>([]);
  const [autoScroll, setAutoScroll] = useState(true);
  const [logFilter, setLogFilter] = useState<LogFilter>("all");

  const { displayTime } = useQuestTimer(status);
  const {
    updateInfo,
    updateStatus,
    updateError,
    appVersion,
    latestVersion,
    downloadProgress,
    checkForUpdates,
    openExternal,
    handleUpdate,
    setUpdateStatus,
  } = useUpdateChecker();

  const { exportCurrentPage, exportAllRounds } = useLogExport(logEntries, currentRound);

  const githubUrl = `https://github.com/${GITHUB_REPO}`;

  // ── Startup timing log ──
  useEffect(() => {
    console.log(`[startup] App mounted: ${performance.now().toFixed(0)}ms`);
  }, []);

  // ── Settings polling ──
  useEffect(() => {
    const fetchSettings = async () => {
      try {
        const res = await fetch(`${API}/api/settings`);
        setSettings(await res.json());
      } catch {
        /* mhw-radar.exe not running */
      }
    };

    fetchSettings();
    const id = setInterval(fetchSettings, 2000);
    return () => clearInterval(id);
  }, []);

  // ── Log polling ──
  const fetchLogs = useCallback(async () => {
    const requestId = ++logsRequestIdRef.current;

    try {
      const requestedRound = currentRound;
      const res = await fetch(`${API}/api/logs?round=${requestedRound}`);
      const data: LogResponse = await res.json();

      if (requestId !== logsRequestIdRef.current) {
        return;
      }

      const nextTotalRounds = data.total_rounds;
      const previousTotalRounds = totalRoundsRef.current;
      const latestRound = Math.max(0, nextTotalRounds - 1);

      totalRoundsRef.current = nextTotalRounds;
      setTotalRounds(nextTotalRounds);

      if (nextTotalRounds <= 0) {
        setCurrentRound(0);
        setLogEntries([]);
        return;
      }

      if (nextTotalRounds > previousTotalRounds && requestedRound !== latestRound) {
        setCurrentRound(latestRound);
        return;
      }

      if (requestedRound > latestRound) {
        setCurrentRound(latestRound);
        return;
      }

      setLogEntries(data.entries);
    } catch {
      /* ignore */
    }
  }, [currentRound]);

  useEffect(() => {
    fetchLogs();
    const id = setInterval(fetchLogs, 500);
    return () => clearInterval(id);
  }, [fetchLogs]);

  // ── Connection log polling（延迟 1 秒启动） ──
  useEffect(() => {
    const fetchConnectionLogs = async () => {
      try {
        const res = await fetch(`${API}/api/connection-logs`);
        const data: ConnectionLogResponse = await res.json();
        setConnectionLogs(data.entries);
      } catch {
        /* backend not running yet */
      }
    };

    const startId = setTimeout(() => {
      fetchConnectionLogs();
      const id = setInterval(fetchConnectionLogs, 2000);
      intervalId = id;
    }, 1000);

    let intervalId: ReturnType<typeof setInterval>;

    return () => {
      clearTimeout(startId);
      if (intervalId !== undefined) clearInterval(intervalId);
    };
  }, []);

  // ── Settings update ──
  const updateSetting = async (patch: Partial<Settings>) => {
    if (!settings) return;

    const updated = { ...settings, ...patch };
    setSettings(updated);

    try {
      await fetch(`${API}/api/settings`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(updated),
      });
    } catch {
      /* ignore */
    }
  };

  const handleToggle = (key: BoolKey, value: boolean) => {
    updateSetting({ [key]: value } as unknown as Partial<Settings>);
  };

  // ── Log clear ──
  const clearLogs = async () => {
    try {
      const res = await fetch(`${API}/api/logs/clear`, { method: "POST" });
      if (!res.ok) {
        throw new Error(await readApiError(res));
      }

      logsRequestIdRef.current += 1;
      totalRoundsRef.current = 0;
      setLogEntries([]);
      setCurrentRound(0);
      setTotalRounds(0);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      window.alert(`清除日志失败：${message}`);
    }
  };

  // ── Close window ──
  const closeWindow = async (e: React.MouseEvent) => {
    e.stopPropagation();

    try {
      await invoke("kill_engine");
    } catch {
      /* command may not exist in some contexts */
    }

    try {
      await getCurrentWindow().destroy();
    } catch {
      try {
        await getCurrentWindow().close();
      } catch {
        /* give up */
      }
    }
  };

  return (
    <PanelView
      version={appVersion}
      logo={<LogoImage fill />}
      onOpenExternal={openExternal}
      onClose={closeWindow}
      status={{ status, displayTime }}
      banner={
        <UpdateBanner
          status={updateStatus}
          updateInfo={updateInfo}
          updateError={updateError}
          downloadProgress={downloadProgress}
          onUpdate={handleUpdate}
          onDismiss={() => setUpdateStatus("latest")}
          onRetry={checkForUpdates}
          onOpenGithub={openExternal}
        />
      }
      opacity={{ settings, onChange: updateSetting }}
      toggles={{ settings, onToggle: handleToggle }}
      diagnostics={{ logs: connectionLogs }}
      logs={{
        entries: logEntries, totalRounds, currentRound, autoScroll, logFilter,
        onAutoScrollChange: setAutoScroll, onFilterChange: setLogFilter, onPageChange: setCurrentRound,
        onClear: clearLogs, onExportCurrent: exportCurrentPage, onExportAll: exportAllRounds,
      }}
      analysis={<DesktopLogAnalysis />}
      updates={{
        updateStatus, updateInfo, updateError, latestVersion, downloadProgress, githubUrl,
        onCheck: checkForUpdates, onUpdate: handleUpdate, onOpenGithub: openExternal,
      }}
    />
  );
}

function DesktopLogAnalysis() {
  const stats = useApi<QuestStat[]>("/api/quest-stats", 2000);
  const actionStats = useApi<MonsterActionStats[]>("/api/action-stats", 3000);

  return <LogAnalysis stats={stats ?? []} actionStats={actionStats ?? []} />;
}
