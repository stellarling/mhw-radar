export interface Settings {
  show_time: boolean;
  show_monster_name: boolean;
  show_hp: boolean;
  show_dist_h: boolean;
  show_dist_v: boolean;
  show_angle: boolean;
  show_action_id: boolean;
  show_action_name: boolean;
  show_counterattack: boolean;
  window_opacity: number;
  text_opacity: number;
}

export interface PanelStatus {
  connected: boolean;
  in_quest: boolean;
  has_monster: boolean;
  monster_name: string | null;
  quest_elapsed_ms: number | null;
  quest_name: string | null;
  connection_state: string;
  pid: number | null;
  module_base: number | null;
}

export interface LogEntry {
  timestamp: string;
  level: string;
  message: string;
  monster_id?: number;
  action_id?: number;
}

export interface LogResponse {
  entries: LogEntry[];
  round: number;
  total_rounds: number;
}

export interface QuestStat {
  quest_name: string;
  quest_id: number;
  total: number;
  success: number;
  fail: number;
  abandon: number;
  avg_abandon_ms: number;
  avg_completion_ms: number;
  fastest_ms: number;
  slowest_ms: number;
  recent_completions: number[];
}

export interface ActionCountItem {
  action_name: string;
  action_id: number;
  count: number;
}

export interface MonsterActionStats {
  monster_id: number;
  monster_name: string;
  total_actions: number;
  actions: ActionCountItem[];
}

export interface UpdateInfo {
  tag: string;
  url: string;
  fileName: string;
}

export interface UpdateDownloadProgress {
  downloaded: number;
  total: number | null;
  percent: number | null;
  message: string;
}

export interface DownloadUpdateResult {
  path: string;
  size: number;
  elapsedMs: number;
}

export type ConnectionEventType = "Waiting" | "Connected" | "Disconnected" | "Reconnected" | "ReadError" | "Info";

export interface ConnectionLogEntry {
  timestamp: string;
  event_type: ConnectionEventType;
  message: string;
  pid: number | null;
  module_base: number | null;
}

export interface ConnectionLogResponse {
  entries: ConnectionLogEntry[];
}

export type UpdateStatus =
  | "idle"
  | "checking"
  | "available"
  | "latest"
  | "downloading"
  | "installing"
  | "error";

export interface GitHubReleaseAsset {
  name: string;
  browser_download_url: string;
}

export type BoolKey =
  | "show_time"
  | "show_monster_name"
  | "show_hp"
  | "show_counterattack"
  | "show_dist_h"
  | "show_dist_v"
  | "show_angle"
  | "show_action_id"
  | "show_action_name";
