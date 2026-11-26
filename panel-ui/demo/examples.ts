import type { ConnectionLogEntry, LogEntry, MonsterActionStats, PanelStatus, QuestStat, Settings } from "../src/types";

export const demoSettings: Settings = {
  show_time: true,
  show_monster_name: true,
  show_hp: true,
  show_radar: true,
  show_action_id: false,
  show_action_name: true,
  show_counterattack: false,
  window_opacity: 0.7,
  text_opacity: 1,
};

export const demoStatus: PanelStatus = {
  connected: true,
  in_quest: true,
  has_monster: true,
  monster_name: "黑龙",
  quest_elapsed_ms: 151480,
  quest_name: "传说中的黑龙",
  connection_state: "Connected",
  pid: null,
  module_base: null,
};

export const demoConnectionLogs: ConnectionLogEntry[] = [
  { timestamp: "20:14:02", event_type: "Connected", message: "示例记录 · 游戏客户端已连接", pid: null, module_base: null },
];

const fatalis = { monster_id: 101, monster_name: "黑龙" };
const rajang = { monster_id: 92, monster_name: "激昂金狮子" };

export const demoRounds: LogEntry[][] = [
  [
    { timestamp: "19:52:00", level: "Quest", message: "进入任务：终点" },
    { timestamp: "19:52:24", level: "Combat", message: "命中激昂金狮子，造成 86 点伤害", ...rajang },
    { timestamp: "19:52:41", level: "Info", message: "激昂金狮子：进入红手状态 · AI 3D 距离 5.28 / 水平 5.21", action_id: 197, ...rajang },
    { timestamp: "19:53:07", level: "Combat", message: "命中激昂金狮子，造成 142 点伤害", ...rajang },
    { timestamp: "19:54:11", level: "Info", message: "激昂金狮子：破头倒地 · AI 3D 距离 3.42 / 水平 3.38", action_id: 198, ...rajang },
    { timestamp: "20:00:21", level: "Success", message: "任务完成：终点" },
  ],
  [
    { timestamp: "20:14:05", level: "Quest", message: "进入任务：传说中的黑龙" },
    { timestamp: "20:14:23", level: "Info", message: "已锁定目标：黑龙", ...fatalis },
    { timestamp: "20:14:42", level: "Combat", message: "命中黑龙，造成 132 点伤害", ...fatalis },
    { timestamp: "20:15:08", level: "Combat", message: "命中黑龙，造成 218 点伤害", ...fatalis },
    { timestamp: "20:15:26", level: "Info", message: "黑龙：破头倒地 · AI 3D 距离 4.82 / 水平 4.74", action_id: 241, ...fatalis },
    { timestamp: "20:15:39", level: "Combat", message: "命中黑龙，造成 305 点伤害", ...fatalis },
    { timestamp: "20:16:09", level: "Combat", message: "命中黑龙，造成 96 点伤害", ...fatalis },
    { timestamp: "20:16:30", level: "Combat", message: "命中黑龙，造成 161 点伤害", ...fatalis },
  ],
];

export const demoQuestStats: QuestStat[] = [
  {
    quest_id: 1, quest_name: "传说中的黑龙", total: 12, success: 8, fail: 3, abandon: 1,
    avg_abandon_ms: 175300, avg_completion_ms: 745820, fastest_ms: 651420, slowest_ms: 912360,
    recent_completions: [734410, 651420, 807900, 712120, 756480, 695420, 912360, 696450],
  },
  {
    quest_id: 2, quest_name: "终点", total: 7, success: 5, fail: 1, abandon: 1,
    avg_abandon_ms: 90210, avg_completion_ms: 517050, fastest_ms: 456370, slowest_ms: 616920,
    recent_completions: [501000, 456370, 478710, 532250, 616920],
  },
];

export const demoActionStats: MonsterActionStats[] = [
  {
    ...fatalis,
    total_actions: 70,
    actions: [
      { action_id: 1, action_name: "直线吐息", count: 36 },
      { action_id: 2, action_name: "扇形吐息", count: 22 },
      { action_id: 241, action_name: "破头倒地", count: 12 },
    ],
  },
  {
    ...rajang,
    total_actions: 50,
    actions: [
      { action_id: 3, action_name: "雷光吐息", count: 29 },
      { action_id: 197, action_name: "进入红手状态", count: 16 },
      { action_id: 198, action_name: "破头倒地", count: 5 },
    ],
  },
];
