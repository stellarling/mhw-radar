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
