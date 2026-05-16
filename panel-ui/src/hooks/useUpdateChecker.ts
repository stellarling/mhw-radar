import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { GITHUB_API_LATEST } from "../constants";
import { compareVersions, findUpdateAsset } from "../utils/version";
import { ensureHttpsUrl } from "../utils/url";
import type { UpdateInfo, UpdateStatus } from "../types";

export function useUpdateChecker() {
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus>("checking");
  const [updateError, setUpdateError] = useState("");
  const [appVersion, setAppVersion] = useState("");
  const [latestVersion, setLatestVersion] = useState("");

  const checkForUpdates = useCallback(async () => {
    setUpdateStatus("checking");
    setUpdateError("");

    try {
      const currentVersion = await invoke<string>("get_version");
      setAppVersion(currentVersion);

      const res = await fetch(GITHUB_API_LATEST, { cache: "no-store" });
      if (!res.ok) throw new Error(`GitHub API 返回 ${res.status}`);

      const data = await res.json();
      const latestTag = String((data as { tag_name?: unknown }).tag_name ?? "");
      if (!latestTag) throw new Error("GitHub Release 未返回 tag_name");

      setLatestVersion(latestTag);

      if (compareVersions(latestTag, currentVersion) > 0) {
        const asset = findUpdateAsset(data, latestTag);
        if (!asset) {
          throw new Error(`Release ${latestTag} 中未找到更新包`);
        }
        setUpdateInfo({
          tag: latestTag,
          url: ensureHttpsUrl(asset.browser_download_url),
          fileName: asset.name,
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
      setUpdateStatus((current) => current === "available" ? current : "error");
    }
  }, []);

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

  return {
    updateInfo,
    updateStatus,
    updateError,
    appVersion,
    latestVersion,
    checkForUpdates,
    openExternal,
    handleUpdate,
    setUpdateStatus,
  };
}
