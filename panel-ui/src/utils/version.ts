import type { GitHubReleaseAsset, UpdateInfo, UpdateStatus } from "../types";

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

export function formatUpdateStatus(
  status: UpdateStatus,
  updateInfo: UpdateInfo | null,
  error: string,
  latestVersion: string,
): string {
  switch (status) {
    case "checking":
      return "正在检查更新...";
    case "available":
      return `发现新版本 ${updateInfo?.tag ?? ""}`;
    case "downloading":
      return "正在下载更新...";
    case "latest":
      return latestVersion ? `已是最新（${latestVersion}）` : "已是最新";
    case "error":
      return `检查失败${error ? `: ${error}` : ""}`;
    case "idle":
    default:
      return "尚未检查";
  }
}
