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
    `MHW-Radar-v${normalized}.zip`,
  ];

  return assets.find((asset) => expectedNames.includes(asset.name)) ?? null;
}

export function findChecksumAsset(data: unknown, zipAssetName: string): GitHubReleaseAsset | null {
  const assets = (data as { assets?: GitHubReleaseAsset[] }).assets ?? [];
  const expectedName = `${zipAssetName}.sha256`;
  return assets.find((asset) => asset.name === expectedName) ?? null;
}

export function parseSha256Sidecar(text: string, expectedFileName: string): string {
  const line = text.split(/\r?\n/).map((l) => l.trim()).find(Boolean);
  if (!line) throw new Error("SHA-256 文件为空");

  const match = line.match(/^([a-fA-F0-9]{64})(?:\s+(.+))?$/);
  if (!match) throw new Error("SHA-256 文件格式无效");

  const hash = match[1].toLowerCase();
  const fileName = match[2]?.trim();

  if (!fileName) {
    throw new Error("SHA-256 文件缺少文件名");
  }

  if (fileName !== expectedFileName) {
    throw new Error(`SHA-256 文件名不匹配: expected ${expectedFileName}, got ${fileName}`);
  }

  return hash;
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
      return "更新包已下载，正在启动更新器...";
    case "latest":
      return latestVersion ? `已是最新（${latestVersion}）` : "已是最新";
    case "error":
      return `更新失败${error ? `: ${error}` : ""}`;
    case "idle":
    default:
      return "尚未检查";
  }
}
