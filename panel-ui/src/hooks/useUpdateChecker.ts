import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { GITHUB_API_LATEST } from "../constants";
import { compareVersions, findUpdateAsset, findChecksumAsset, parseSha256Sidecar } from "../utils/version";
import { ensureHttpsUrl } from "../utils/url";
import type {
  DownloadUpdateResult,
  UpdateDownloadProgress,
  UpdateInfo,
  UpdateStatus,
} from "../types";

const CHECK_TIMEOUT_MS = 15_000;

async function fetchJsonWithTimeout(url: string): Promise<unknown> {
  const controller = new AbortController();
  const timer = window.setTimeout(() => controller.abort(), CHECK_TIMEOUT_MS);

  try {
    const res = await fetch(url, {
      cache: "no-store",
      signal: controller.signal,
      headers: {
        Accept: "application/vnd.github+json",
      },
    });

    if (!res.ok) throw new Error(`GitHub API 返回 ${res.status}`);
    return await res.json();
  } catch (err) {
    if (err instanceof DOMException && err.name === "AbortError") {
      throw new Error("连接 GitHub API 超时，请检查网络或稍后重试");
    }
    throw err;
  } finally {
    window.clearTimeout(timer);
  }
}

export function useUpdateChecker() {
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus>("checking");
  const [updateError, setUpdateError] = useState("");
  const [appVersion, setAppVersion] = useState("");
  const [latestVersion, setLatestVersion] = useState("");
  const [downloadProgress, setDownloadProgress] = useState<UpdateDownloadProgress | null>(null);

  useEffect(() => {
    let mounted = true;

    const unlistenPromise = listen<UpdateDownloadProgress>(
      "update-download-progress",
      (event) => {
        if (mounted) setDownloadProgress(event.payload);
      },
    );

    return () => {
      mounted = false;
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  const checkForUpdates = useCallback(async () => {
    setUpdateStatus("checking");
    setUpdateError("");
    setDownloadProgress(null);

    try {
      const currentVersion = await invoke<string>("get_version");
      setAppVersion(currentVersion);

      const data = await fetchJsonWithTimeout(GITHUB_API_LATEST);
      const latestTag = String((data as { tag_name?: unknown }).tag_name ?? "");
      if (!latestTag) throw new Error("GitHub Release 未返回 tag_name");

      setLatestVersion(latestTag);

      if (compareVersions(latestTag, currentVersion) > 0) {
        const asset = findUpdateAsset(data, latestTag);
        if (!asset) {
          throw new Error(`Release ${latestTag} 中未找到更新包`);
        }

        // 查找 SHA-256 校验文件
        const checksumAsset = findChecksumAsset(data, asset.name);
        if (!checksumAsset) {
          throw new Error(`Release ${latestTag} 缺少 SHA-256 校验文件`);
        }

        // 下载 SHA-256 内容
        const shaRes = await fetch(ensureHttpsUrl(checksumAsset.browser_download_url), {
          cache: "no-store",
        });
        if (!shaRes.ok) {
          throw new Error(`SHA-256 校验文件下载失败: HTTP ${shaRes.status}`);
        }
        const shaText = await shaRes.text();
        const sha256 = parseSha256Sidecar(shaText, asset.name);

        setUpdateInfo({
          tag: latestTag,
          url: ensureHttpsUrl(asset.browser_download_url),
          fileName: asset.name,
          sha256,
        });
        setUpdateStatus("available");
      } else {
        setUpdateInfo(null);
        setUpdateStatus("latest");
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setUpdateInfo(null);
      setUpdateError(message);
      setUpdateStatus("error");
    }
  }, []);

  useEffect(() => {
    void checkForUpdates();
  }, [checkForUpdates]);

  const openExternal = useCallback(async (url: string) => {
    try {
      await invoke("open_external_url", { url: ensureHttpsUrl(url) });
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setUpdateError(message);
      setUpdateStatus((current) => (current === "available" ? current : "error"));
    }
  }, []);

  const handleUpdate = useCallback(async () => {
    if (!updateInfo) return;

    setUpdateStatus("downloading");
    setUpdateError("");
    setDownloadProgress({
      downloaded: 0,
      total: null,
      percent: null,
      message: "正在准备下载更新包...",
    });

    try {
      const tempDir = await invoke<string>("get_temp_dir");
      const safeTag = updateInfo.tag.replace(/[^a-zA-Z0-9._-]/g, "_");
      const unique = `${Date.now()}-${Math.random().toString(16).slice(2)}`;
      const zipPath = `${tempDir}\\mhw-radar-update-${safeTag}-${unique}\\${updateInfo.fileName}`;

      const result = await invoke<DownloadUpdateResult>("download_update", {
        url: updateInfo.url,
        dest: zipPath,
        expectedSha256: updateInfo.sha256,
      });

      setDownloadProgress({
        downloaded: result.size,
        total: result.size,
        percent: 100,
        message: `更新包下载完成，用时 ${(result.elapsedMs / 1000).toFixed(1)} 秒`,
      });

      setUpdateStatus("installing");

      const appDir = await invoke<string>("get_app_dir");
      await invoke("spawn_updater", { appDir, zipPath: result.path || zipPath });

      await getCurrentWindow().destroy();
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      console.error("Update failed:", err);
      setUpdateError(message);
      setUpdateStatus("error");
    }
  }, [updateInfo]);

  return {
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
  };
}
