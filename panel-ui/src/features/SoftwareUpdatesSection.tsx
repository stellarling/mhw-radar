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
                    : "#b0b0b0",
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
                    fontSize: 13,
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
                  fontSize: 13,
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
              style={{ color: "#8ab4f8", fontSize: 13, cursor: "pointer", wordBreak: "break-all" }}
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
                fontSize: 13,
                color: "#8c8c8c",
                border: "1px solid #555",
              }}
            >
              <span
                onClick={() => onOpenGithub(githubUrl)}
                style={{ color: "#8ab4f8", fontSize: 13, cursor: "pointer" }}
              >打开 GitHub</span>
            </div>
          </Card>
        </div>

        {/* Changelog */}
        <Card style={{ marginBottom: 16 }}>
          <div style={{ color: "#bfa76b", fontSize: 14, marginBottom: 12 }}>更新日志</div>
          {[
            {
              ver: "v0.2.0", date: "2026-05-18", items: [
                "修复手动保存日志可能无文件的问题，优化导出稳定性与错误提示",
                "新增每轮战斗日志自动保存到 logs/ 目录，按任务时间命名",
                "时间戳改为 UTC+8，追加黑龙破头倒地等特殊事件日志行高亮",
              ]
            },
            {
              ver: "v0.1.0", date: "2026-05-16", items: [
                "初次发布，支持怪物血量、位置距离、攻击角度等基础信息显示",
                "内存地址精确寻址，支持 Alatreon 等怪物 AI 决策值读取",
                "回合制狩猎日志系统，自动记录每次攻击与怪物动作变更",
                "日志分页查看，支持导出全部/当前页",
                "透明悬浮窗覆盖层，支持快捷键 Ctrl+Shift+U 切换",
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
        </Card>
      </SectionWrapper>
    );
  });
