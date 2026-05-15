import { useState, useEffect, useCallback, useRef, useLayoutEffect } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { API, GITHUB_API_LATEST, btnStyle } from "./constants";
import { useApi } from "./api";
import { Sidebar } from "./components/Sidebar";
import { HeaderBar } from "./components/HeaderBar";
import { StatusBar } from "./components/StatusBar";
import { OpacitySection } from "./components/OpacitySection";
import { ToggleSection } from "./components/ToggleSection";
import { LogSection } from "./components/LogSection";
import type { Settings, LogEntry, LogResponse, BoolKey } from "./types";

export default function App() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const status = useApi<PanelStatus>("/api/status", 1000);
  const [logEntries, setLogEntries] = useState<LogEntry[]>([]);
  const [currentRound, setCurrentRound] = useState(0);
  const [totalRounds, setTotalRounds] = useState(0);
  const logEndRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = useState(true);

  // ── 自动更新 ──
  const [updateInfo, setUpdateInfo] = useState<{ tag: string; url: string; fileName: string } | null>(null);
  const [updateStatus, setUpdateStatus] = useState<"idle" | "checking" | "available" | "downloading" | "error">("idle");
  const [updateError, setUpdateError] = useState("");
  const [appVersion, setAppVersion] = useState("");

  // ── 任务计时平滑插值 ──
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

  // ── 轮询 settings ──
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

  // ── 轮次日志轮询 ──
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

  // 自动滚动时跟随最新轮次
  useEffect(() => {
    if (autoScroll && totalRounds > 0) {
      setCurrentRound(totalRounds - 1);
    }
  }, [autoScroll, totalRounds]);

  useEffect(() => {
    if (!autoScroll || logEntries.length === 0 || !logEndRef.current) return;

    const logContainer = logEndRef.current.parentElement as HTMLDivElement | null;

    if (logContainer) {
      logContainer.scrollTo({
        top: logContainer.scrollHeight,
        behavior: "smooth",
      });
    }
  }, [logEntries, autoScroll]);

  // ── 检查更新 ──
  useEffect(() => {
    const check = async () => {
      try {
        const currentVersion = await invoke<string>("get_version");
        setAppVersion(currentVersion);
        const res = await fetch(GITHUB_API_LATEST);
        if (!res.ok) throw new Error("API error");
        const data = await res.json();
        const latestTag = data.tag_name as string;

        if (compareVersions(latestTag, currentVersion) > 0) {
          const asset = findUpdateAsset(data, latestTag);
          if (!asset) {
            throw new Error(`Release ${latestTag} 中未找到 MHW-Radar-vX.Y.Z.zip 更新包`);
          }

          setUpdateInfo({
            tag: latestTag,
            url: asset.browser_download_url,
            fileName: asset.name,
          });
          setUpdateError("");
          setUpdateStatus("available");
        }
      } catch {
        /* repo private or network unavailable — silent skip */
      }
    };
    check();
  }, []);

  // ── 执行更新（下载 → bat → 退出） ──
  const handleUpdate = useCallback(async () => {
    if (!updateInfo) return;
    setUpdateStatus("downloading");
    try {
      setUpdateError("");
      const tempDir = await invoke<string>("get_temp_dir");
      const zipPath = `${tempDir}\\${updateInfo.fileName}`;
      await invoke("download_update", { url: updateInfo.url, dest: zipPath });
      const appDir = await invoke<string>("get_app_dir");
      await invoke("spawn_updater", { appDir, zipPath });
      await getCurrentWindow().destroy();
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      console.error("Update failed:", err);
      setUpdateError(message);
      setUpdateStatus("error");
    }
  }, [updateInfo]);

  // ── 设置更新 ──
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

  // ── Scroll-spy ──
  type SectionId = "basic-tools" | "hunting-log" | "log-analysis" | "software-updates" | "usage-guide";

  const [activeSection, setActiveSection] = useState<SectionId>("basic-tools");

  const mainPanelRef = useRef<HTMLDivElement>(null);
  const basicToolsRef = useRef<HTMLDivElement>(null);
  const huntingLogRef = useRef<HTMLDivElement>(null);
  const logAnalysisRef = useRef<HTMLDivElement>(null);
  const softwareUpdatesRef = useRef<HTMLDivElement>(null);
  const usageGuideRef = useRef<HTMLDivElement>(null);

  const scrollLockRef = useRef<SectionId | null>(null);
  const scrollLockTimerRef = useRef<number | null>(null);

  const resetMainPanelToTop = useCallback(() => {
    const panel = mainPanelRef.current;
    if (!panel) return;

    panel.scrollTo({ top: 0, left: 0, behavior: "auto" });
    setActiveSection("basic-tools");
  }, []);

  useLayoutEffect(() => {
    resetMainPanelToTop();

    // 首帧绘制后再次重置，避免 WebView 的 scroll restoration 覆盖掉初始设置
    const raf = window.requestAnimationFrame(resetMainPanelToTop);

    return () => window.cancelAnimationFrame(raf);
  }, [resetMainPanelToTop]);

  useEffect(() => {
    const panel = mainPanelRef.current;
    if (!panel) return;

    const getSectionTop = (el: HTMLElement) => el.offsetTop;

    const onScroll = () => {
      if (scrollLockRef.current) {
        setActiveSection(scrollLockRef.current);
        return;
      }

      const threshold = 80;
      const scrollTop = panel.scrollTop;
      const maxScrollTop = panel.scrollHeight - panel.clientHeight;

      if (maxScrollTop > 0 && scrollTop >= maxScrollTop - 2) {
        setActiveSection("usage-guide");
        return;
      }

      const sections: Array<{ id: SectionId; el: HTMLDivElement | null }> = [
        { id: "basic-tools", el: basicToolsRef.current },
        { id: "hunting-log", el: huntingLogRef.current },
        { id: "log-analysis", el: logAnalysisRef.current },
        { id: "software-updates", el: softwareUpdatesRef.current },
        { id: "usage-guide", el: usageGuideRef.current },
      ];

      let current: SectionId = "basic-tools";

      for (const { id, el } of sections) {
        if (!el) continue;

        if (scrollTop + threshold >= getSectionTop(el)) {
          current = id;
        }
      }

      setActiveSection(current);
    };

    onScroll();

    panel.addEventListener("scroll", onScroll, { passive: true });
    window.addEventListener("resize", onScroll);

    return () => {
      panel.removeEventListener("scroll", onScroll);
      window.removeEventListener("resize", onScroll);

      if (scrollLockTimerRef.current != null) {
        window.clearTimeout(scrollLockTimerRef.current);
        scrollLockTimerRef.current = null;
      }
    };
  }, []);

  const scrollToSection = (id: SectionId) => {
    setActiveSection(id);
    scrollLockRef.current = id;

    if (scrollLockTimerRef.current != null) {
      window.clearTimeout(scrollLockTimerRef.current);
    }

    scrollLockTimerRef.current = window.setTimeout(() => {
      scrollLockRef.current = null;
      scrollLockTimerRef.current = null;
    }, 700);

    const panel = mainPanelRef.current;
    if (!panel) return;

    const SCROLL_OFFSET = 44;

    if (id === "basic-tools") {
      panel.scrollTo({ top: 0, behavior: "smooth" });
      return;
    }

    const target = panel.querySelector<HTMLElement>(`#${id}`);
    if (!target) return;

    panel.scrollTo({
      top: Math.max(0, target.offsetTop - SCROLL_OFFSET),
      behavior: "smooth",
    });
  };

  const scrollToBottom = () => {
    mainPanelRef.current?.scrollTo({
      top: mainPanelRef.current.scrollHeight,
      behavior: "smooth",
    });
  };

  // ── 关闭窗口 ──
  const closeWindow = async (e: React.MouseEvent) => {
    e.stopPropagation();

    try {
      await invoke("kill_engine");
    } catch {
      /* command may not exist in some contexts */
    }

    try {
      await getCurrentWindow().destroy();
    } catch (err) {
      console.error("destroy failed:", err);

      try {
        await getCurrentWindow().close();
      } catch {
        /* give up */
      }
    }
  };

  // ── 日志操作 ──
  const clearLogs = async () => {
    setLogEntries([]);
    setCurrentRound(0);
    setTotalRounds(1);

    try {
      await fetch(`${API}/api/logs/clear`, { method: "POST" });
    } catch {
      /* ignore */
    }
  };

  const saveLogFile = useCallback(async (text: string) => {
    let defaultPath = `mhw-radar-${new Date().toISOString().slice(0, 10)}.txt`;

    try {
      const res = await fetch(`${API}/api/desktop-path`);
      const data = await res.json();
      if (data.path) {
        defaultPath = `${data.path}\\${defaultPath}`;
      }
    } catch { /* ignore */ }

    const filePath = await save({
      defaultPath,
      filters: [{ name: "文本文件", extensions: ["txt"] }],
    });

    if (!filePath) return;

    await fetch(`${API}/api/logs/export`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ path: filePath, content: text }),
    });
  }, []);

  const exportCurrentPage = useCallback(() => {
    if (logEntries.length === 0) return;
    const text = logEntries
      .map((e) => `[${e.timestamp}] [${e.level}] ${e.message}`)
      .join("\n");
    saveLogFile(text);
  }, [logEntries, saveLogFile]);

  const exportAllRounds = useCallback(async () => {
    try {
      const res = await fetch(`${API}/api/logs`);
      const data: LogResponse = await res.json();
      if (data.entries.length === 0) return;
      const text = data.entries
        .map((e) => `[${e.timestamp}] [${e.level}] ${e.message}`)
        .join("\n");
      saveLogFile(text);
    } catch { /* ignore */ }
  }, [saveLogFile]);

  // ── Render ──
  return (
    <>
      <style>{`
        .drag-region { -webkit-app-region: drag; app-region: drag; }
        .drag-region .no-drag,
        .drag-region [data-no-drag] { -webkit-app-region: no-drag; app-region: no-drag; }
      `}</style>

      <div
        style={{
          display: "flex",
          width: "100vw",
          height: "100vh",
          fontFamily: "system-ui, -apple-system, sans-serif",

          borderRadius: 20,
          overflow: "hidden",

          backgroundImage: "url(/background.png)",
          backgroundSize: "100% 100%",
          backgroundPosition: "center",
          backgroundRepeat: "no-repeat",

          border: "none",
          outline: "none",
          boxShadow: "none",
        }}
      >
        <Sidebar
          activeSection={activeSection}
          onNavigate={(id) => scrollToSection(id as SectionId)}
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

            {/* ── 更新通知栏 ── */}
            {updateStatus === "available" && updateInfo && (
              <div style={{
                display: "flex", alignItems: "center", gap: 10,
                padding: "8px 20px",
                background: "rgba(191,167,107,0.12)",
                borderBottom: "1px solid #bfa76b",
              }}>
                <span style={{ color: "#bfa76b", fontSize: 13, fontWeight: 500 }}>
                  发现新版本 {updateInfo.tag}
                </span>
                <button onClick={handleUpdate} style={btnStyle}>立即更新</button>
                <button
                  onClick={() => setUpdateStatus("idle")}
                  style={{ ...btnStyle, background: "transparent", color: "#8c8c8c", fontSize: 12 }}
                >以后再说</button>
              </div>
            )}
            {updateStatus === "downloading" && (
              <div style={{
                display: "flex", alignItems: "center", gap: 8,
                padding: "8px 20px",
                background: "rgba(191,167,107,0.08)",
                borderBottom: "1px solid #331e12",
              }}>
                <span style={{ color: "#b0b0b0", fontSize: 13 }}>正在下载更新...</span>
              </div>
            )}
            {updateStatus === "error" && (
              <div style={{
                display: "flex", alignItems: "center", gap: 10,
                padding: "8px 20px",
                background: "rgba(244,67,54,0.1)",
                borderBottom: "1px solid #f44336",
              }}>
                <span style={{ color: "#f44336", fontSize: 13 }}>
                  更新失败{updateError ? `：${updateError}` : "，请稍后重试"}
                </span>
                <button
                  onClick={() => setUpdateStatus("available")}
                  style={{ ...btnStyle, background: "transparent", color: "#8ab4f8", fontSize: 12 }}
                >重试</button>
              </div>
            )}

            <OpacitySection settings={settings} onChange={updateSetting} />

            <div ref={basicToolsRef} id="basic-tools">
              <ToggleSection settings={settings} onToggle={handleToggle} />
            </div>

            <div ref={huntingLogRef} id="hunting-log" style={{ display: "flex", flexDirection: "column" }}>
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

            <div ref={logAnalysisRef} id="log-analysis" style={{ padding: "16px 20px", borderTop: "1px solid #331e12" }}>
              <h2 style={{ color: "#dcdcdc", fontSize: 16, margin: "0 0 4px" }}>日志分析</h2>
              <p style={{ color: "#8c8c8c", fontSize: 12, margin: "0 0 16px" }}>开发中，敬请期待</p>

              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
                {[
                  { label: "DPS 曲线重构", desc: "基于时间轴的伤害输出图表" },
                  { label: "出招统计", desc: "各招式使用频率与命中率" },
                  { label: "结算率统计", desc: "任务完成率与狩猎效率分析" },
                  { label: "时空位置回溯", desc: "战斗过程中怪物与玩家的时空轨迹回放" },
                  { label: "受击伤害统计", desc: "每次受击的伤害来源与数值记录" },
                ].map((item) => (
                  <div
                    key={item.label}
                    style={{
                      padding: 16,
                      borderRadius: 6,
                      border: "1px solid #331e12",
                      background: "rgba(0,0,0,0.15)",
                    }}
                  >
                    <div style={{ color: "#bfa76b", fontSize: 14, marginBottom: 6 }}>{item.label}</div>
                    <div style={{ color: "#6c6c6c", fontSize: 12 }}>{item.desc}</div>
                    <div
                      style={{
                        display: "inline-block",
                        marginTop: 8,
                        padding: "2px 8px",
                        borderRadius: 3,
                        fontSize: 11,
                        color: "#8c8c8c",
                        border: "1px solid #555",
                      }}
                    >
                      开发中
                    </div>
                  </div>
                ))}
              </div>
            </div>

            <div ref={softwareUpdatesRef} id="software-updates" style={{ padding: "16px 20px", borderTop: "1px solid #331e12" }}>
              <h2 style={{ color: "#dcdcdc", fontSize: 16, margin: "0 0 4px" }}>软件更新</h2>
              <p style={{ color: "#8c8c8c", fontSize: 12, margin: "0 0 16px" }}>版本管理、更新与更新日志</p>

              <div style={{ display: "flex", gap: 12, marginBottom: 16 }}>
                <div
                  style={{
                    flex: 1,
                    padding: 16,
                    borderRadius: 6,
                    border: "1px solid #331e12",
                    background: "rgba(0,0,0,0.15)",
                  }}
                >
                  <div style={{ color: "#bfa76b", fontSize: 14, marginBottom: 6 }}>自动更新</div>
                  <div style={{ color: "#b0b0b0", fontSize: 12 }}>
                    已是最新
                  </div>
                  <div
                    style={{
                      display: "inline-block",
                      marginTop: 8,
                      padding: "2px 8px",
                      borderRadius: 3,
                      fontSize: 11,
                      color: "#8c8c8c",
                      border: "1px solid #555",
                    }}
                  >
                    已是最新
                  </div>
                </div>

                <div
                  style={{
                    flex: 1,
                    padding: 16,
                    borderRadius: 6,
                    border: "1px solid #331e12",
                    background: "rgba(0,0,0,0.15)",
                  }}
                >
                  <div style={{ color: "#bfa76b", fontSize: 14, marginBottom: 6 }}>GitHub</div>
                  <a href="https://github.com/stellarling/mhw-radar" target="_blank" rel="noopener noreferrer" style={{ color: "#8ab4f8", fontSize: 12 }}>github.com/stellarling/mhw-radar</a>
                  <div
                    style={{
                      display: "inline-block",
                      marginTop: 8,
                      padding: "2px 8px",
                      borderRadius: 3,
                      fontSize: 11,
                      color: "#8c8c8c",
                      border: "1px solid #555",
                    }}
                  >
                    <a href="https://github.com/stellarling/mhw-radar" target="_blank" rel="noopener noreferrer" style={{ color: "#8ab4f8", fontSize: 11 }}>打开 GitHub</a>
                  </div>
                </div>
              </div>

              {/* ── 更新日志 ── */}
              <div
                style={{
                  padding: 16,
                  borderRadius: 6,
                  border: "1px solid #331e12",
                  background: "rgba(0,0,0,0.15)",
                  marginBottom: 16,
                }}
              >
                <div style={{ color: "#bfa76b", fontSize: 14, marginBottom: 12 }}>更新日志</div>
                {[
                  {
                    ver: "v0.1.0", date: "2026-05-16", items: [
                      "初次发布，支持怪物血量、位置距离、攻击角度等基础信息显示",
                      "内存地址精确寻址，支持 Alatreon 等怪物 AI 决策值读取",
                      "回合制狩猎日志系统，自动记录每次攻击与怪物动作变更",
                      "日志分页查看，支持导出全部/当前页",
                      "透明悬浮窗覆盖层，支持快捷键 Ctrl+Shift+U 切换",
                      "管理员权限适配与杀毒软件误报说明",
                      "新增使用说明文档",
                    ]
                  },
                ].map((entry) => (
                  <div key={entry.ver} style={{ marginBottom: 12, paddingBottom: 12, borderBottom: "1px solid #2a1a10" }}>
                    <div style={{ display: "flex", alignItems: "baseline", gap: 8, marginBottom: 6 }}>
                      <span style={{ color: "#dcdcdc", fontSize: 14, fontWeight: 500 }}>{entry.ver}</span>
                      <span style={{ color: "#6c6c6c", fontSize: 11 }}>{entry.date}</span>
                    </div>
                    <ul style={{ margin: 0, paddingLeft: 16, color: "#b0b0b0", fontSize: 12, lineHeight: 1.8 }}>
                      {entry.items.map((item, i) => (
                        <li key={i}>{item}</li>
                      ))}
                    </ul>
                  </div>
                ))}
                <div style={{ color: "#6c6c6c", fontSize: 11, textAlign: "center" }}>
                  - 更多更新记录请访问 GitHub Releases -
                </div>
              </div>
            </div>

            <div ref={usageGuideRef} id="usage-guide" style={{ padding: "16px 20px", borderTop: "1px solid #331e12" }}>
              <h2 style={{ color: "#dcdcdc", fontSize: 16, margin: "0 0 4px" }}>使用说明</h2>
              <p style={{ color: "#8c8c8c", fontSize: 12, margin: "0 0 16px" }}>关于本软件与相关声明</p>

              <div
                style={{
                  padding: 16,
                  borderRadius: 6,
                  border: "1px solid #331e12",
                  background: "rgba(0,0,0,0.15)",
                  marginBottom: 16,
                }}
              >
                <div style={{ color: "#bfa76b", fontSize: 15, marginBottom: 8 }}>关于本软件</div>
                <div style={{ color: "#d0d0d0", fontSize: 14, lineHeight: 1.8 }}>
                  <div style={{ marginBottom: 10 }}>
                    MHW Radar 是一款为《怪物猎人：世界》玩家打造的实时狩猎辅助工具。
                    目前还在早期的开发阶段，请谨慎使用。
                    通过读取内存数据并输出到可视化透明覆盖层，在游戏画面中实时显示怪物血量、位置距离、
                    攻击角度、伤害数值及任务进度等信息，帮助猎人更精准地掌握战局、优化输出节奏。
                  </div>

                  <div style={{ marginBottom: 10 }}>
                    但事实上我们并不鼓励开启悬浮窗，本软件更合适的用途是用于战斗的复盘和更好得狩猎，因此软件
                    内置完整的狩猎日志系统，自动记录每次攻击与怪物动作变更，支持导出分析与即时复盘。
                  </div>

                  <div>
                    本工具仅提供信息呈现，不修改任何游戏文件或内存数据，所有解析均基于客户端只读方式，
                    不影响游戏原有逻辑与网络通信，符合公平游戏原则。
                  </div>
                </div>
              </div>

              <div
                style={{
                  padding: 16,
                  borderRadius: 6,
                  border: "1px solid #331e12",
                  background: "rgba(0,0,0,0.08)",
                }}
              >
                <div style={{ color: "#b0b0b0", fontSize: 13, lineHeight: 2 }}>
                  本工具为开源免费软件，仅供学习交流使用，禁止用于商业用途。<br />
                  所有游戏相关数据、美术素材及商标版权均归属 CAPCOM CO., LTD.<br />
                  《怪物猎人：世界》《怪物猎人：世界·冰原》© CAPCOM CO., LTD. ALL RIGHTS RESERVED.<br />
                  使用本工具所产生的任何后果由使用者自行承担。
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </>
  );
}

// ── Local types ──
interface PanelStatus {
  connected: boolean;
  in_quest: boolean;
  has_monster: boolean;
  monster_name: string | null;
  quest_elapsed_ms: number | null;
  quest_name: string | null;
}


type GitHubReleaseAsset = {
  name: string;
  browser_download_url: string;
};

function findUpdateAsset(data: unknown, latestTag: string): GitHubReleaseAsset | null {
  const assets = (data as { assets?: GitHubReleaseAsset[] }).assets ?? [];
  const normalized = latestTag.replace(/^v/i, "");
  const expectedNames = [
    `MHW-Radar-${latestTag}.zip`,
    `MHW-Radar-v${normalized}.zip`,
  ];

  return (
    assets.find((asset) => expectedNames.includes(asset.name)) ??
    assets.find((asset) => /^MHW-Radar-v?\d+\.\d+\.\d+\.zip$/i.test(asset.name)) ??
    null
  );
}

function compareVersions(tag: string, local: string): number {
  const pa = tag.replace(/^v/i, "").split(".").map(Number);
  const pb = local.replace(/^v/i, "").split(".").map(Number);
  for (let i = 0; i < 3; i++) {
    if ((pa[i] ?? 0) > (pb[i] ?? 0)) return 1;
    if ((pa[i] ?? 0) < (pb[i] ?? 0)) return -1;
  }
  return 0;
}