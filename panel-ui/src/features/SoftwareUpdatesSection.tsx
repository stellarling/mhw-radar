import { forwardRef } from "react";
import { btnStyle } from "../constants";
import { Card } from "../ui/Card";
import { SectionHeader } from "../ui/SectionHeader";
import { SectionWrapper } from "../layouts/SectionWrapper";
import { formatProgress, formatUpdateStatus } from "../utils/version";
import type { UpdateDownloadProgress, UpdateInfo, UpdateStatus } from "../types";

interface SoftwareUpdatesSectionProps {
  updateStatus: UpdateStatus;
  updateInfo: UpdateInfo | null;
  updateError: string;
  latestVersion: string;
  githubUrl: string;
  downloadProgress: UpdateDownloadProgress | null;
  onCheck: () => void;
  onUpdate: () => void;
  onOpenGithub: (url: string) => void;
}

export const SoftwareUpdatesSection = forwardRef<HTMLDivElement, SoftwareUpdatesSectionProps>(
  function SoftwareUpdatesSection({
    updateStatus,
    updateInfo,
    updateError,
    latestVersion,
    githubUrl,
    downloadProgress,
    onCheck,
    onUpdate,
    onOpenGithub,
  }, ref) {
    const busy = updateStatus === "checking" || updateStatus === "downloading" || updateStatus === "installing";
    const progressPercent =
      downloadProgress?.percent === null || downloadProgress?.percent === undefined
        ? null
        : Math.max(0, Math.min(100, downloadProgress.percent));

    return (
      <SectionWrapper ref={ref} id="software-updates">
        <SectionHeader title="软件更新" description="版本管理、更新与更新日志" />

        <div style={{ display: "flex", gap: 12, marginBottom: 16 }}>
          <Card style={{ flex: 1 }}>
            <div style={{ color: "#bfa76b", fontSize: 14, marginBottom: 6 }}>自动更新</div>
            <div
              style={{
                color: updateStatus === "available" || updateStatus === "downloading" || updateStatus === "installing"
                  ? "#bfa76b"
                  : updateStatus === "error"
                    ? "#ff8a80"
                    : "#dcdcdc",
                fontSize: 14,
                lineHeight: 1.7,
              }}
            >
              {formatUpdateStatus(updateStatus, updateInfo, updateError, latestVersion, downloadProgress)}
            </div>

            {(updateStatus === "downloading" || updateStatus === "installing") && (
              <div style={{ marginTop: 10 }}>
                <div
                  style={{
                    height: 8,
                    borderRadius: 999,
                    background: "#2a1a10",
                    overflow: "hidden",
                    border: "1px solid #443018",
                  }}
                >
                  <div
                    style={{
                      height: "100%",
                      width: progressPercent === null ? "18%" : `${progressPercent}%`,
                      minWidth: progressPercent === null ? 24 : 0,
                      background: "#bfa76b",
                      transition: "width 0.2s ease",
                    }}
                  />
                </div>
                <div style={{ marginTop: 6, color: "#8c8c8c", fontSize: 12, lineHeight: 1.6 }}>
                  {downloadProgress?.message || "正在下载更新包..."}
                  {downloadProgress ? ` · ${formatProgress(downloadProgress)}` : ""}
                </div>
              </div>
            )}

            {updateStatus === "error" && (
              <div style={{ marginTop: 8, color: "#8c8c8c", fontSize: 12, lineHeight: 1.7 }}>
                如果 GitHub 连接缓慢或失败，可以点击右侧 GitHub 链接手动下载最新 ZIP 后覆盖安装。
              </div>
            )}

            <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginTop: 8 }}>
              {updateStatus === "available" && updateInfo && (
                <button
                  type="button"
                  style={{
                    ...btnStyle,
                    padding: "2px 8px",
                    fontSize: 14,
                    color: "#bfa76b",
                    border: "1px solid #bfa76b",
                  }}
                  onClick={onUpdate}
                >
                  立即更新
                </button>
              )}
              <button
                type="button"
                style={{
                  ...btnStyle,
                  padding: "2px 8px",
                  fontSize: 14,
                  color: "#8c8c8c",
                  border: "1px solid #555",
                  opacity: busy ? 0.55 : 1,
                  cursor: busy ? "not-allowed" : "pointer",
                }}
                disabled={busy}
                onClick={onCheck}
              >
                {updateStatus === "checking" ? "检查中" : "重新检查"}
              </button>
            </div>
          </Card>

          <Card style={{ flex: 1 }}>
            <div style={{ color: "#bfa76b", fontSize: 14, marginBottom: 6 }}>GitHub</div>
            <div
              onClick={() => onOpenGithub(githubUrl)}
              style={{ color: "#8ab4f8", fontSize: 14, cursor: "pointer", wordBreak: "break-all" }}
            >{githubUrl}</div>
            <div style={{ color: "#8c8c8c", fontSize: 12, lineHeight: 1.7, marginTop: 8 }}>
              自动更新依赖 GitHub Release 附件下载。网络不可达、被代理拦截或 CDN 较慢时，自动更新会失败或变慢。
            </div>
            <div
              style={{
                display: "inline-block",
                marginTop: 8,
                padding: "2px 8px",
                borderRadius: 3,
                fontSize: 14,
                color: "#8c8c8c",
                border: "1px solid #555",
              }}
            >
              <span
                onClick={() => onOpenGithub(githubUrl)}
                style={{ color: "#8ab4f8", fontSize: 14, cursor: "pointer" }}
              >打开 GitHub</span>
            </div>
          </Card>
        </div>

        {/* Changelog */}
        <Card style={{ marginBottom: 16 }}>
          <div style={{ color: "#bfa76b", fontSize: 14, marginBottom: 12 }}>更新日志</div>
          {[
            {
              ver: "v0.3.0", date: "2026-05-30", items: [
                "【重大变更】独立更新器 mhw-radar-updater.exe：更新逻辑从临时 .cmd 脚本迁移为独立 Rust 程序，支持自更新、失败回滚、安装锁防并发",
                "【安全加固】更新包 SHA-256 校验、下载/路径 URL 白名单限制、ZIP bomb 防护、Zip Slip 防护",
                "日志分析面板：结算率统计、完成时间分析、出招频率统计",
                "招式数据补充、下压值日志显示、UI 布局调整",
                "启动速度优化：移除 startup taskkill、更新检查延迟 3s、ECharts 懒加载",
              ]
            },
            {
              ver: "v0.2.0", date: "2026-05-18", items: [
                "优化自动下载与更新逻辑，修复导出/时间戳/滚动等问题",
                "追加黑龙特殊事件日志高亮，日志存储路径优化",
              ]
            },
            {
              ver: "v0.1.0", date: "2026-05-16", items: [
                "初次发布，支持怪物血量、位置、角度等基础信息显示",
                "透明悬浮窗 overlay，回合制狩猎日志系统",
                "日志分页查看与导出",
              ]
            },
          ].map((entry) => (
            <div key={entry.ver} style={{ marginBottom: 12, paddingBottom: 12, borderBottom: "1px solid #2a1a10" }}>
              <div style={{ display: "flex", alignItems: "baseline", gap: 8, marginBottom: 6 }}>
                <span style={{ color: "#dcdcdc", fontSize: 14, fontWeight: 500 }}>{entry.ver}</span>
                <span style={{ color: "#8c8c8c", fontSize: 11 }}>{entry.date}</span>
              </div>
              <ul style={{ margin: 0, paddingLeft: 16, color: "#dcdcdc", fontSize: 12, lineHeight: 1.8 }}>
                {entry.items.map((item, i) => (
                  <li key={i}>{item}</li>
                ))}
              </ul>
            </div>
          ))}
          <div style={{ color: "#8c8c8c", fontSize: 12, textAlign: "center" }}>
            - 更多更新记录请访问 GitHub Releases -
          </div>
        </Card>
      </SectionWrapper>
    );
  });
