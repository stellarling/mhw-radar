import type { CSSProperties } from "react";
import type { BoolKey, LogEntry } from "./types";

export const API = "http://127.0.0.1:17320";

// ── 自动更新 ──
export const GITHUB_REPO = "stellarling/mhw-radar";
export const GITHUB_API_LATEST = `https://api.github.com/repos/${GITHUB_REPO}/releases/latest`;
export const GITHUB_RELEASE_DOWNLOAD = (tag: string, file: string) =>
  `https://github.com/${GITHUB_REPO}/releases/download/${tag}/${file}`;

export const TOGGLES: { key: BoolKey; label: string }[] = [
  ["show_time", "显示时间"],
  ["show_monster_name", "显示怪物名称"],
  ["show_hp", "显示血量"],
  ["show_counterattack", "显示下压值(黑龙)"],
  ["show_dist_h", "显示水平距离"],
  ["show_dist_v", "显示高度落差"],
  ["show_angle", "显示角度"],
  ["show_action_id", "显示怪物动作ID"],
  ["show_action_name", "显示实际招式"],
].map(([key, label]) => ({ key: key as BoolKey, label }));

export const LOG_COLORS: Record<string, string> = {
  Success: "#4caf50",
  Warning: "#ffeb3b",
  Error: "#f44336",
  Combat: "#ffa500",
  Quest: "#64c8ff",
};

// ── 日志高亮规则 ──
// 匹配怪物 ID + 动作 ID，对整行应用特殊样式
// 如需新增规则，在此数组追加即可
export interface HighlightRule {
  label: string;
  match: (entry: LogEntry) => boolean;
  style: { backgroundColor: string; color?: string };
}

export const HIGHLIGHT_RULES: HighlightRule[] = [
  {
    label: "黑龙破头倒地",
    match: (e) => e.monster_id === 101 && (e.action_id === 241 || e.action_id === 242),
    style: { backgroundColor: "rgba(76, 175, 80, 0.25)", color: "#8bc34a" },
  },
];

export const btnStyle: CSSProperties = {
  background: "#331e12",
  color: "#dcdcdc",
  border: "none",
  borderRadius: 4,
  padding: "6px 12px",
  fontSize: 14,
  cursor: "pointer",
};

export const circleBtnStyle: CSSProperties = {
  width: 28,
  height: 28,
  borderRadius: "50%",
  background: "#2a1a10",
  color: "#dcdcdc",
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  cursor: "pointer",
  fontSize: 16,
  userSelect: "none",
  lineHeight: 1,
  flexShrink: 0,
  border: "none",
  fontFamily: "inherit",
  padding: 0,
};

export function formatTime(ms: number): string {
  const total_cs = Math.floor(ms / 10);
  const minutes = Math.floor(total_cs / 6000);
  const seconds = Math.floor((total_cs / 100) % 60);
  const centis = total_cs % 100;
  return `${minutes}'${String(seconds).padStart(2, "0")}'${String(centis).padStart(2, "0")}`;
}
