import type { BoolKey } from "../types";

export const TOGGLES: { key: BoolKey; label: string }[] = [
  ["show_time", "显示时间"],
  ["show_monster_name", "显示怪物名称"],
  ["show_hp", "显示血量"],
  ["show_counterattack", "显示下压值(黑龙)"],
  ["show_radar", "显示雷达图"],
  ["show_action_id", "显示怪物动作ID"],
  ["show_action_name", "显示实际招式"],
].map(([key, label]) => ({ key: key as BoolKey, label }));
