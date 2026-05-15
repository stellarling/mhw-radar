//! 共享数据模型：UI 配置、雷达数据传输对象、怪物血量
//!
//! 纯数据类型，不包含任何读取或渲染逻辑，
//! 被 reader/overlay/panel 模块共同引用。

use serde::{Deserialize, Serialize};

/// 怪物血量数据
#[derive(Default, Clone, Copy, Serialize)]
pub struct MonsterHp {
    pub current: f32,
    pub max: f32,
}

/// 显示设置
#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub show_time: bool,
    pub show_monster_name: bool,
    pub show_hp: bool,
    pub show_dist_h: bool,
    pub show_dist_v: bool,
    pub show_angle: bool,
    pub show_action_id: bool,
    pub show_action_name: bool,
    pub show_counterattack: bool,
    pub window_opacity: f32,
    pub text_opacity: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            show_time: true,
            show_monster_name: true,
            show_hp: true,
            show_dist_h: true,
            show_dist_v: true,
            show_angle: true,
            show_action_id: true,
            show_action_name: true,
            show_counterattack: true,
            window_opacity: 180.0 / 255.0,
            text_opacity: 1.0,
        }
    }
}

/// 每帧从游戏读取的显示数据
#[derive(Default, Clone, Serialize)]
pub struct RadarData {
    pub connected: bool,
    pub has_monster: bool,
    /// 水平距离（XZ 平面）
    pub dist_h: f32,
    /// 垂直高度差（Y 轴落差）
    pub dist_v: f32,
    pub angle: f32,
    pub flashing: bool,
    /// 任务计时（ms），None=无计时器可显示
    pub quest_elapsed_ms: Option<u64>,
    /// 任务ID
    pub quest_id: i32,
    /// 任务中文名称
    pub quest_name: Option<&'static str>,
    /// 怪物血量
    pub monster_hp: Option<MonsterHp>,
    /// 当前怪物动作ID
    pub action_id: i32,
    /// 怪物ID（来自游戏进程）
    pub monster_id: i32,
    /// 怪物中文名称
    pub monster_name: Option<&'static str>,
    /// 招式中文名称（根据怪物ID+动作ID查表）
    pub action_name: Option<&'static str>,
    /// 招式英文名称（从游戏内存读取，备选显示）
    pub action_name_en: Option<String>,
    /// 黑龙下压值（仅怪物ID=101时有效）
    pub counterattack_value: Option<f32>,
    /// 下压值是否已进入 179 动作后换算模式（/0.7）
    pub counterattack_scaled: bool,
}

/// 返回给面板的状态摘要
#[derive(Clone, Serialize)]
pub struct PanelStatus {
    pub connected: bool,
    pub in_quest: bool,
    pub has_monster: bool,
    pub monster_name: Option<&'static str>,
    pub quest_elapsed_ms: Option<u64>,
    pub quest_name: Option<&'static str>,
}
