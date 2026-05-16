import { useState, useEffect, useCallback, useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { API, GITHUB_REPO } from "./constants";
import { useApi } from "./hooks/useApi";
import { MainLayout } from "./layouts/MainLayout";
import { Sidebar } from "./layouts/Sidebar";
import { HeaderBar } from "./layouts/HeaderBar";
import { StatusBar } from "./layouts/StatusBar";
import { OpacitySection } from "./features/OpacitySection";
import { ToggleSection } from "./features/ToggleSection";
import { LogSection } from "./features/LogSection";
import { UpdateBanner } from "./features/UpdateBanner";
import { LogAnalysisSection } from "./features/LogAnalysisSection";
import { SoftwareUpdatesSection } from "./features/SoftwareUpdatesSection";
import { UsageGuideSection } from "./features/UsageGuideSection";
import { useQuestTimer } from "./hooks/useQuestTimer";
import { useScrollSpy } from "./hooks/useScrollSpy";
import { useUpdateChecker } from "./hooks/useUpdateChecker";
import { useLogExport } from "./hooks/useLogExport";
import { readApiError } from "./utils/api";
import type { Settings, LogEntry, LogResponse, BoolKey, PanelStatus } from "./types";

export default function App() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const status = useApi<PanelStatus>("/api/status", 1000);
  const [logEntries, setLogEntries] = useState<LogEntry[]>([]);
  const [currentRound, setCurrentRound] = useState(0);
  const [totalRounds, setTotalRounds] = useState(0);
  const logEndRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = useState(true);

  const { displayTime } = useQuestTimer(status);
  const { activeSection, scrollToSection, mainPanelRef, sectionRefs } = useScrollSpy();
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
    try {
      const res = await fetch(`${API}/api/logs?round=${currentRound}`);
      const data: LogResponse = await res.json();
      setLogEntries(data.entries);
      setTotalRounds(data.total_rounds);
    } catch {
      /* ignore */
    }
  }, [currentRound]);

  useEffect(() => {
    fetchLogs();
    const id = setInterval(fetchLogs, 500);
    return () => clearInterval(id);
  }, [fetchLogs]);

  // ── Auto-scroll ──
  useEffect(() => {
    if (autoScroll && totalRounds > 0) setCurrentRound(totalRounds - 1);
  }, [autoScroll, totalRounds]);

  useEffect(() => {
    if (!autoScroll || logEntries.length === 0 || !logEndRef.current) return;
    const logContainer = logEndRef.current.parentElement as HTMLDivElement | null;
    if (logContainer) {
      logContainer.scrollTo({ top: logContainer.scrollHeight, behavior: "smooth" });
    }
  }, [logEntries, autoScroll]);

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

      setLogEntries([]);
      setCurrentRound(0);
      setTotalRounds(1);
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

  const scrollToBottom = () => {
    mainPanelRef.current?.scrollTo({
      top: mainPanelRef.current.scrollHeight,
      behavior: "smooth",
    });
  };

  // ── Render ──
  return (
    <>
      <style>{`
        .drag-region { -webkit-app-region: drag; app-region: drag; }
        .drag-region .no-drag,
        .drag-region [data-no-drag] { -webkit-app-region: no-drag; app-region: no-drag; }
      `}</style>
      <MainLayout>
        <Sidebar
          activeSection={activeSection}
          onNavigate={(id) => scrollToSection(id as Parameters<typeof scrollToSection>[0])}
        />

        <div style={{ flex: 1, display: "flex", flexDirection: "column", minWidth: 0 }}>
          <HeaderBar
            version={appVersion}
            onScrollBottom={scrollToBottom}
            onClose={closeWindow}
          />

          <div
            ref={mainPanelRef}
            className="main-panel"
            style={{
              flex: 1,
              overflowY: "auto",
              display: "flex",
              flexDirection: "column",
            }}
          >
            <StatusBar status={status} displayTime={displayTime} />
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
            <OpacitySection settings={settings} onChange={updateSetting} />

            <div ref={sectionRefs.basicTools} id="basic-tools">
              <ToggleSection settings={settings} onToggle={handleToggle} />
            </div>

            <div
              ref={sectionRefs.huntingLog}
              id="hunting-log"
              style={{ display: "flex", flexDirection: "column" }}
            >
              <LogSection
                ref={logEndRef}
                entries={logEntries}
                totalRounds={totalRounds}
                currentRound={currentRound}
                autoScroll={autoScroll}
                onAutoScrollChange={setAutoScroll}
                onPageChange={setCurrentRound}
                onClear={clearLogs}
                onExportCurrent={exportCurrentPage}
                onExportAll={exportAllRounds}
              />
            </div>

            <LogAnalysisSection ref={sectionRefs.logAnalysis} />
            <SoftwareUpdatesSection
              ref={sectionRefs.softwareUpdates}
              updateStatus={updateStatus}
              updateInfo={updateInfo}
              updateError={updateError}
              latestVersion={latestVersion}
              downloadProgress={downloadProgress}
              githubUrl={githubUrl}
              onCheck={checkForUpdates}
              onUpdate={handleUpdate}
              onOpenGithub={openExternal}
            />
            <UsageGuideSection ref={sectionRefs.usageGuide} />
          </div>
        </div>
      </MainLayout>
    </>
  );
}