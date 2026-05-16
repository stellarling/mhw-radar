import { btnStyle } from "../constants";
import { GITHUB_RELEASES_URL } from "../constants/api";
import { formatProgress } from "../utils/version";
import type { UpdateDownloadProgress, UpdateInfo, UpdateStatus } from "../types";

export function UpdateBanner({
  status,
  updateInfo,
  updateError,
  downloadProgress,
  onUpdate,
  onDismiss,
  onRetry,
  onOpenGithub,
}: {
  status: UpdateStatus;
  updateInfo: UpdateInfo | null;
  updateError: string;
  downloadProgress: UpdateDownloadProgress | null;
  onUpdate: () => void;
  onDismiss: () => void;
  onRetry: () => void;
  onOpenGithub: (url: string) => void;
}) {
  if (status === "available" && updateInfo) {
    return (
      <div style={{
        display: "flex", alignItems: "center", gap: 10,
        padding: "8px 20px",
        background: "rgba(191,167,107,0.12)",
        borderBottom: "1px solid #bfa76b",
      }}>
        <span style={{ color: "#bfa76b", fontSize: 13, fontWeight: 500 }}>
          发现新版本 {updateInfo.tag}
        </span>
        <button onClick={onUpdate} style={btnStyle}>立即更新</button>
        <button
          onClick={onDismiss}
          style={{ ...btnStyle, background: "transparent", color: "#8c8c8c", fontSize: 12 }}
        >以后再说</button>
      </div>
    );
  }

  if (status === "downloading") {
    return (
      <div style={{
        display: "flex", alignItems: "center", gap: 8,
        padding: "8px 20px",
        background: "rgba(191,167,107,0.08)",
        borderBottom: "1px solid #331e12",
      }}>
        <span style={{ color: "#b0b0b0", fontSize: 13 }}>
          {downloadProgress?.message || "正在下载更新包..."}
          {downloadProgress ? ` · ${formatProgress(downloadProgress)}` : ""}
        </span>
      </div>
    );
  }

  if (status === "installing") {
    return (
      <div style={{
        display: "flex", alignItems: "center", gap: 8,
        padding: "8px 20px",
        background: "rgba(191,167,107,0.08)",
        borderBottom: "1px solid #331e12",
      }}>
        <span style={{ color: "#b0b0b0", fontSize: 13 }}>
          更新包已下载，正在启动安装脚本...
        </span>
      </div>
    );
  }

  if (status === "error") {
    return (
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
          onClick={onRetry}
          style={{ ...btnStyle, background: "transparent", color: "#8ab4f8", fontSize: 12 }}
        >重试</button>
        <button
          onClick={() => onOpenGithub(GITHUB_RELEASES_URL)}
          style={{ ...btnStyle, background: "transparent", color: "#8ab4f8", fontSize: 12 }}
        >打开 GitHub 手动下载</button>
      </div>
    );
  }

  return null;
}