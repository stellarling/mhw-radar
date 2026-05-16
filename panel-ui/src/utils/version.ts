import type {
  GitHubReleaseAsset,
  UpdateDownloadProgress,
  UpdateInfo,
  UpdateStatus,
} from "../types";

export function compareVersions(tag: string, local: string): number {
  const pa = tag.replace(/^v/i, "").split(".").map(Number);
  const pb = local.replace(/^v/i, "").split(".").map(Number);
  for (let i = 0; i < 3; i++) {
    if ((pa[i] ?? 0) > (pb[i] ?? 0)) return 1;
    if ((pa[i] ?? 0) < (pb[i] ?? 0)) return -1;
  }
  return 0;
}

export function findUpdateAsset(data: unknown, latestTag: string): GitHubReleaseAsset | null {
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

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let index = 0;

  while (value >= 1024 && index < units.length - 1) {
    value /= 1024;
    index += 1;
  }

  return `${value.toFixed(index === 0 ? 0 : 1)} ${units[index]}`;
}

export function formatProgress(progress: UpdateDownloadProgress | null): string {
  if (!progress) return "";

  const downloaded = formatBytes(progress.downloaded);
  const total = progress.total ? formatBytes(progress.total) : "";
  const percent =
    progress.percent === null || progress.percent === undefined
      ? ""
      : `${progress.percent.toFixed(1)}%`;

  if (total && percent) return `${percent}（${downloaded} / ${total}）`;
  if (total) return `${downloaded} / ${total}`;
  return downloaded;
}

export function formatUpdateStatus(
  status: UpdateStatus,
  updateInfo: UpdateInfo | null,
  error: string,
  latestVersion: string,
  progress?: UpdateDownloadProgress | null,
): string {
  switch (status) {
    case "checking":
      return "正在检查更新...";
    case "available":
      return `发现新版本 ${updateInfo?.tag ?? ""}`;
    case "downloading": {
      const detail = formatProgress(progress ?? null);
      const message = progress?.message || "正在下载更新包...";
      return detail ? `${message} ${detail}` : message;
    }
    case "installing":
      return "更新包已下载，正在启动安装脚本...";
    case "latest":
      return latestVersion ? `已是最新（${latestVersion}）` : "已是最新";
    case "error":
      return `更新失败${error ? `: ${error}` : ""}`;
    case "idle":
    default:
      return "尚未检查";
  }
}
