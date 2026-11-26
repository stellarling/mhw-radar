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
  onUpdate?: () => void;
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

        <div className="mb-4 grid gap-3 md:grid-cols-2">
          <Card>
            <h3 className="m-0 mb-1.5 text-sm font-semibold text-panel-gold">自动更新</h3>
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
              {updateStatus === "available" && updateInfo && onUpdate && (
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

          <Card>
            <h3 className="m-0 mb-1.5 text-sm font-semibold text-panel-gold">GitHub</h3>
            <a
              href={githubUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="text-sm break-all text-sky-300 underline"
              onClick={(event) => {
                event.preventDefault();
                onOpenGithub(githubUrl);
              }}
            >{githubUrl}</a>
            <div style={{ color: "#8c8c8c", fontSize: 12, lineHeight: 1.7, marginTop: 8 }}>
              自动更新依赖 GitHub Release 附件下载。网络不可达、被代理拦截或 CDN 较慢时，自动更新会失败或变慢。
            </div>
          </Card>
        </div>

        {/* Changelog */}
        <Card style={{ marginBottom: 16 }}>
          <h3 className="m-0 mb-3 text-sm font-semibold text-panel-gold">更新日志</h3>
          {[
            {
              ver: "v0.3.7", date: "2026-11-26", items: [
                "更新控制面板样式，首次进入日志分析时加载统计图表",
              ]
            },
            {
              ver: "v0.3.6", date: "2026-11-23", items: [
                "出招日志直接记录游戏 AI 的三维距离与水平距离",
                "修复聚魔之地的怪物显血和 R3 锁定跟随",
                "统一雷达坐标来源，修正高度提示与不可用状态",
                "按怪物实例缓存招式名称，修复狩猎日志自动滚动",
              ]
            },
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
            <div key={entry.ver} className="mb-3 border-0 border-b border-solid border-panel-border pb-3">
              <div className="mb-1.5 flex items-baseline gap-2">
                <span className="text-sm font-medium text-panel-text">{entry.ver}</span>
                <span className="text-xs text-panel-muted">{entry.date}</span>
              </div>
              <ul className="m-0 list-disc pl-4 text-xs leading-relaxed text-panel-text">
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
