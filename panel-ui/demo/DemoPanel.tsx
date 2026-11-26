import { useState } from "react";

import { GITHUB_RELEASES_URL } from "../src/constants";
import { LogAnalysis } from "../src/features/LogAnalysis";
import type { LogFilter } from "../src/features/LogSection";
import { PanelView } from "../src/layouts/PanelView";
import { LogoImage } from "../src/ui/LogoImage";
import type { LogEntry, Settings } from "../src/types";
import { demoActionStats, demoConnectionLogs, demoQuestStats, demoRounds, demoSettings, demoStatus } from "./examples";

const VERSION = "0.3.7";
const latestRound = demoRounds.length - 1;

function exportExample(entries: LogEntry[], file: string) {
  if (entries.length === 0) return;
  const text = entries.map(({ timestamp, level, message }) => `[${timestamp}] [${level}] ${message}`).join("\r\n");
  const url = URL.createObjectURL(new Blob([text, "\r\n"], { type: "text/plain;charset=utf-8" }));
  const link = document.createElement("a");
  link.href = url;
  link.download = file;
  document.body.append(link);
  link.click();
  link.remove();
  window.setTimeout(() => URL.revokeObjectURL(url), 1000);
}

export function DemoPanel() {
  const [settings, setSettings] = useState(demoSettings);
  const [round, setRound] = useState(latestRound);
  const [showLogs, setShowLogs] = useState(true);
  const [autoScroll, setAutoScroll] = useState(true);
  const [filter, setFilter] = useState<LogFilter>("all");
  const [checkedUpdates, setCheckedUpdates] = useState(false);
  const entries = showLogs ? demoRounds[round] : [];

  function updateSettings(patch: Partial<Settings>) {
    setSettings((current) => ({ ...current, ...patch }));
  }

  return (
    <PanelView
      version={VERSION}
      logo={<LogoImage fill src={new URL("../../engine/resources/title-zh-en.png", import.meta.url).href} />}
      status={{ status: demoStatus, displayTime: demoStatus.quest_elapsed_ms, demo: true }}
      opacity={{ settings, onChange: updateSettings }}
      toggles={{ settings, onToggle: (key, value) => updateSettings({ [key]: value }) }}
      diagnostics={{ logs: demoConnectionLogs }}
      logs={{
        entries, currentRound: showLogs ? round : 0, totalRounds: showLogs ? demoRounds.length : 0,
        autoScroll, logFilter: filter, onAutoScrollChange: setAutoScroll, onFilterChange: setFilter,
        onPageChange: setRound, onClear: () => setShowLogs(false),
        onExportCurrent: () => exportExample(entries, `mhw-radar-demo-${round + 1}.txt`),
        onExportAll: () => { if (showLogs) exportExample(demoRounds.flat(), "mhw-radar-demo.txt"); },
      }}
      analysis={<LogAnalysis stats={demoQuestStats} actionStats={demoActionStats} />}
      updates={{
        updateStatus: checkedUpdates ? "latest" : "idle", updateInfo: null, updateError: "",
        latestVersion: VERSION, downloadProgress: null, githubUrl: GITHUB_RELEASES_URL,
        onCheck: () => setCheckedUpdates(true),
        onOpenGithub: (url) => window.open(url, "_blank", "noopener,noreferrer"),
      }}
    />
  );
}
