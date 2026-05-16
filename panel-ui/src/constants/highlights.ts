import type { LogEntry } from "../types";

export const LOG_COLORS: Record<string, string> = {
  Success: "#4caf50",
  Warning: "#ffeb3b",
  Error: "#f44336",
  Combat: "#ffa500",
  Quest: "#64c8ff",
};

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
