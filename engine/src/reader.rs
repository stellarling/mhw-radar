//! 游戏数据读取器：进程连接、怪物/玩家数据读取、角度计算、招式变化检测
//!
//! DataReader 在独立线程 20FPS 运行，通过共享雷达数据与 UI 线程通信。
//! 本模块不包含任何 UI 状态或渲染逻辑。

use std::collections::HashMap;
use std::f32::consts::PI;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use process_memory::{ProcessHandle, TryIntoProcessHandle};
use sysinfo::System;

use crate::game_data::{load_monster_ai_configs, load_monster_names, load_quest_names, lookup_action_name};
use crate::log::{Logger, QuestState};
use crate::memory::{self, read_ptr_chain};
use crate::types::{MonsterHp, RadarData};

// ── 内存偏移常量 ─────────────────────────────────────────────────

/// 玩家实体指针链偏移
const PLAYER_OFFSETS: [u64; 2] = [0x50, 0x0];
/// 怪物实体指针链偏移
const MONSTER_OFFSETS: [u64; 4] = [0x698, 0x0, 0x138, 0x0];
/// 任务数据指针链偏移
const QUEST_OFFSETS: [u64; 1] = [0x0];
/// 怪物动作ID偏移（actionPtr + 0x61C8 → ptr, *ptr + 0xB0 → actionId）
const ACTION_ID_OFFSET: u64 = 0x61C8 + 0xB0;
/// 动作指针基址偏移
const ACTION_PTR_OFFSET: u64 = 0x61C8;
/// 怪物ID偏移量
const MONSTER_ID_OFFSET: u64 = 0x12280;
/// 动作状态结构体指针偏移
const ACTION_STATE_OFFSET: u64 = 0x468;
/// 当前动作帧偏移（float，新动作从 0 开始递增）
const ACTION_CURRENT_FRAME: u64 = 0x10C;
/// 黑龙下压值指针链基址偏移
const COUNTERATTACK_BASE_OFFSET: u64 = 0x05013C50;
const COUNTERATTACK_OFFSETS: [u64; 2] = [0xE58, 0x18388];
/// Quest timer pointer offset from quest_base
const QUEST_TIMER_PTR_OFFSET: u64 = 0x13180;
/// Known max quest timer raw values → 15/20/30/35/50 minutes
const MAX_QUEST_TIMERS: [u32; 5] = [54000, 72000, 108000, 126000, 180000];

// ── 工具函数 ────────────────────────────────────────────────────

/// 格式化任务计时：M'SS'CC（分钟'秒'百分秒）
pub fn format_quest_time(ms: u64) -> String {
    let total_cs = ms / 10;
    let minutes = total_cs / 6000;
    let seconds = (total_cs / 100) % 60;
    let centis = total_cs % 100;
    format!("{}'{:02}'{:02}", minutes, seconds, centis)
}

/// 清理游戏内动作字符串：提取可读的英文招式名
///
/// 原始字符串格式如 `em001:em001Action<...>`，
/// 处理后得到 `em001 Action`
pub fn sanitize_action_string(raw: &str) -> Option<String> {
    let name = raw.split('<').next()?.split(':').last()?;
    if name.is_empty() {
        return None;
    }
    let result: String = name
        .chars()
        .enumerate()
        .map(|(i, c)| {
            if i > 0 && c.is_uppercase() {
                format!(" {}", c)
            } else {
                c.to_string()
            }
        })
        .collect();
    Some(result)
}

/// 从游戏进程读取怪物的英文招式名称
pub fn read_action_name_en(
    handle: ProcessHandle,
    monster_addr: u64,
    action_id: i32,
) -> Option<String> {
    let action_ptr_base = monster_addr + ACTION_PTR_OFFSET;
    let ptr_offsets = [
        0x78u64,
        (action_id as u64).wrapping_mul(8),
        0,
        0x20,
    ];
    let ptr4 = read_ptr_chain(handle, action_ptr_base, &ptr_offsets)?;
    let action_offset = memory::read_memory::<u32>(handle, ptr4 + 3)?;
    let action_ref = memory::read_memory::<u64>(handle, ptr4 + action_offset as u64 + 7 + 8)?;
    let raw = memory::read_string(handle, action_ref, 64)?;
    sanitize_action_string(&raw)
}

/// 匹配最接近的已知任务时间上限
fn approximate_high(value: u32, values: &[u32]) -> u32 {
    for &v in values {
        if v >= value {
            return v;
        }
    }
    value
}

// ── 后台数据读取器 ──────────────────────────────────────────────

/// 在后台线程运行，负责游戏进程连接、内存读取、动作检测
struct DataReader {
    handle: Option<ProcessHandle>,
    player_base: u64,
    monster_base: u64,
    quest_base: u64,
    counterattack_base: u64,
    game_base: u64,
    disconnect_frames: u32,
    no_game_printed: bool,
    connected_logged: bool,
    last_action_id: i32,
    action_id_init: bool,
    last_frame: f32,
    last_hp: f32,
    flash_until: Instant,
    quest_elapsed_frozen: Option<Duration>,
    last_action_change: Instant,
    last_action_state_ptr: u64,
    cached_action_name_en: Option<(i32, String)>,
    counterattack_scaled: bool,
    /// 上次实际输出的招式中文名（用于同招分组防抖）
    last_grouped_name: Option<&'static str>,
    /// 上次输出招式的时间（用于同招分组防抖）
    last_grouped_time: Instant,
    /// 各怪物AI决策值的地址配置
    monster_ai_configs: HashMap<i32, crate::game_data::MonsterAiConfig>,
    /// 上次记录的AI决策距离（用于AI模式的新动作检测）
    last_logged_ai_dist: Option<f32>,
    /// 上次记录的AI决策角度
    last_logged_ai_angle: Option<f32>,
    /// 上次记录的招式中文名（AI模式下按名称变化兜底检测）
    last_logged_ai_name: Option<&'static str>,
    monster_names: HashMap<i32, &'static str>,
    quest_names: HashMap<i32, &'static str>,
    logger: Logger,
    prev_quest_state: QuestState,
    prev_quest_id: i32,
    quest_counter: i32,
    quest_start_time: Option<String>,
}

impl DataReader {
    fn new(monster_names: HashMap<i32, &'static str>, quest_names: HashMap<i32, &'static str>, logger: Logger) -> Self {
        let now = Instant::now();
        Self {
            handle: None,
            player_base: 0,
            monster_base: 0,
            quest_base: 0,
            counterattack_base: 0,
            game_base: 0,
            disconnect_frames: 0,
            no_game_printed: false,
            connected_logged: false,
            last_action_id: 0,
            action_id_init: false,
            last_frame: 0.0,
            last_hp: 0.0,
            flash_until: now - Duration::from_secs(1),
            quest_elapsed_frozen: None,
            last_action_change: now - Duration::from_secs(1),
            last_action_state_ptr: 0,
            cached_action_name_en: None,
            counterattack_scaled: false,
            last_grouped_name: None,
            last_grouped_time: now - Duration::from_secs(10),
            monster_ai_configs: load_monster_ai_configs(),
            last_logged_ai_dist: None,
            last_logged_ai_angle: None,
            last_logged_ai_name: None,
            monster_names,
            quest_names,
            logger,
            prev_quest_state: QuestState::None,
            prev_quest_id: 0,
            quest_counter: 0,
            quest_start_time: None,
        }
    }

    /// 扫描游戏进程并建立内存连接
    fn try_attach_game(&mut self) {
        let mut sys = System::new_all();
        sys.refresh_all();
        if let Some(process) = sys.processes_by_exact_name("MonsterHunterWorld.exe").next() {
            let pid = process.pid().as_u32();
            if let Ok(handle) = pid.try_into_process_handle() {
                if let Some(base) = memory::get_module_base(pid) {
                    self.handle = Some(handle);
                    self.player_base = base + 0x050139A0;
                    self.monster_base = base + 0x051238C8;
                    self.quest_base = base + 0x0500ED30;
                    self.counterattack_base = base + COUNTERATTACK_BASE_OFFSET;
                    self.game_base = base;
                    self.disconnect_frames = 0;
                    if !self.connected_logged {
                        self.logger.info(format!("成功捕获游戏进程! PID: {}, 基址: 0x{:X}", pid, base));
                        self.connected_logged = true;
                    }
                    self.no_game_printed = false;
                    return;
                }
            }
        }
        if !self.no_game_printed {
            self.logger.info("未检测到游戏进程，等待游戏启动...");
            self.no_game_printed = true;
        }
    }

    /// 一次数据读取 + 检测（每 50ms 调用一次）
    fn tick(&mut self) -> RadarData {
        let mut data = RadarData::default();

        if self.handle.is_some() {
            let handle = self.handle.unwrap();
            if memory::read_memory::<u16>(handle, self.game_base).is_none() {
                self.disconnect_frames += 1;
                if self.disconnect_frames >= 10 {
                    self.logger.info("检测到游戏进程已退出，等待重连...");
                    self.handle = None;
                    let now = Instant::now();
                    self.no_game_printed = false;
                    self.connected_logged = false;
                    self.action_id_init = false;
                    self.last_action_id = 0;
                    self.last_hp = 0.0;
                    self.last_frame = 0.0;
                    self.last_action_state_ptr = 0;
                    self.cached_action_name_en = None;
                    self.counterattack_scaled = false;
                    self.last_grouped_name = None;
                    self.last_grouped_time = now - Duration::from_secs(10);
                    self.last_logged_ai_dist = None;
                    self.last_logged_ai_angle = None;
                    self.last_logged_ai_name = None;
                    self.quest_elapsed_frozen = None;
                    self.flash_until = now - Duration::from_secs(1);
                    self.last_action_change = now - Duration::from_secs(1);
                }
            } else {
                self.disconnect_frames = 0;
            }
        }

        if self.handle.is_none() {
            self.try_attach_game();
        }

        if let Some(handle) = self.handle {
            data.connected = true;

            let player_pos = memory::resolve_pointer(handle, self.player_base, &PLAYER_OFFSETS)
                .and_then(|addr| memory::read_memory::<memory::Vector3>(handle, addr + 0x390))
                .unwrap_or_default();

            let (quest_state_raw, quest_id) =
                memory::resolve_pointer(handle, self.quest_base, &QUEST_OFFSETS)
                    .and_then(|addr| {
                        let state = memory::read_memory::<i32>(handle, addr + 0x54)?;
                        let id = memory::read_memory::<i32>(handle, addr + 0x4C)?;
                        Some((state, id))
                    })
                    .unwrap_or((0, 0));

            let in_quest = quest_state_raw == 2 && quest_id > 0;
            data.quest_id = quest_id;
            data.quest_name = self.quest_names.get(&quest_id).copied();

            // 任务计时器
            if in_quest {
                let timer_ptr =
                    memory::resolve_pointer(handle, self.quest_base, &[QUEST_TIMER_PTR_OFFSET]);
                if let Some(ptr) = timer_ptr {
                    let timer_raw = memory::read_memory::<u64>(handle, ptr).unwrap_or(0);
                    let max_timer_raw =
                        memory::read_memory::<u32>(handle, ptr + 0x10).unwrap_or(0);
                    if timer_raw > 0 && max_timer_raw > 0 {
                        let remaining_secs = timer_raw as f64 / 60.0;
                        let max_secs =
                            approximate_high(max_timer_raw, &MAX_QUEST_TIMERS) as f64 / 60.0;
                        let elapsed_secs = (max_secs - remaining_secs).max(0.0);
                        let elapsed_ms = (elapsed_secs * 1000.0) as u64;
                        data.quest_elapsed_ms = Some(elapsed_ms);
                        self.quest_elapsed_frozen = Some(Duration::from_millis(elapsed_ms));
                    }
                }
            } else if quest_state_raw >= 3 && quest_id > 0 {
                if let Some(frozen) = self.quest_elapsed_frozen {
                    data.quest_elapsed_ms = Some(frozen.as_millis() as u64);
                }
            } else {
                self.quest_elapsed_frozen = None;
            }

            // 任务状态机
            let current_state = QuestState::from_raw(quest_state_raw);
            if quest_id > 0 || self.prev_quest_id > 0 {
                let was_in_quest =
                    self.prev_quest_state == QuestState::InQuest && self.prev_quest_id > 0;

                if !was_in_quest && in_quest {
                    self.quest_counter += 1;
                    self.quest_start_time = Some(crate::log::format_datetime_utc8_for_filename());
                    let quest_name = self.quest_names.get(&quest_id).copied().unwrap_or("未知");
                    // 每次任务开始都进入独立轮次，避免第一轮任务混入启动/连接日志。
                    self.logger.new_round();
                    self.logger.separator();
                    self.logger.quest(format!(
                        "[{}] 任务开始(ID:{})，本轮第{}次任务",
                        quest_name, quest_id, self.quest_counter
                    ));
                    self.last_hp = 0.0;
                }

                if was_in_quest && current_state.is_over() {
                    let elapsed = data
                        .quest_elapsed_ms
                        .map(format_quest_time)
                        .unwrap_or_else(|| "??".to_string());
                    self.logger.quest(format!(
                        "任务{}！耗时 {}",
                        current_state.label(),
                        elapsed
                    ));
                    self.logger.separator();
                    self.logger.info("");
                    if let Some(ref start) = self.quest_start_time {
                        match self.logger.save_latest_round(start) {
                            Ok(path) => self.logger.info(format!("本轮日志已自动保存：{}", path)),
                            Err(err) => self.logger.error(format!("本轮日志自动保存失败：{}", err)),
                        }
                    }
                    self.quest_start_time = None;
                }

                if was_in_quest && current_state == QuestState::None {
                    let elapsed = data
                        .quest_elapsed_ms
                        .map(format_quest_time)
                        .unwrap_or_else(|| "??".to_string());
                    self.logger.quest(format!("任务中断！耗时 {}", elapsed));
                    self.logger.separator();
                    self.logger.info("");
                    if let Some(ref start) = self.quest_start_time {
                        match self.logger.save_latest_round(start) {
                            Ok(path) => self.logger.info(format!("本轮日志已自动保存：{}", path)),
                            Err(err) => self.logger.error(format!("本轮日志自动保存失败：{}", err)),
                        }
                    }
                    self.quest_start_time = None;
                }
            }
            self.prev_quest_state = current_state;
            self.prev_quest_id = quest_id;

            let monster_addr = if in_quest {
                memory::resolve_pointer(handle, self.monster_base, &MONSTER_OFFSETS)
            } else {
                None
            };

            let monster_pos = monster_addr
                .and_then(|a| memory::read_memory::<memory::Vector3>(handle, a + 0x160))
                .unwrap_or_default();

            data.has_monster = in_quest && (monster_pos.x != 0.0 || monster_pos.z != 0.0);

            let monster_yaw = monster_addr
                .and_then(|a| {
                    let qx = memory::read_memory::<f32>(handle, a + 0x170)?;
                    let qy = memory::read_memory::<f32>(handle, a + 0x174)?;
                    let qz = memory::read_memory::<f32>(handle, a + 0x178)?;
                    let qw = memory::read_memory::<f32>(handle, a + 0x17C)?;
                    let len_sq = qx * qx + qy * qy + qz * qz + qw * qw;
                    if (len_sq - 1.0).abs() > 0.1 {
                        return None;
                    }
                    let yaw_rad =
                        (2.0 * (qw * qy + qx * qz)).atan2(1.0 - 2.0 * (qx * qx + qy * qy));
                    let mut deg = yaw_rad * (180.0 / PI);
                    if deg < 0.0 {
                        deg += 360.0;
                    }
                    Some(deg)
                })
                .unwrap_or(0.0);

            if data.has_monster {
                let addr = monster_addr.unwrap();

                // ── 招式变化检测 ──
                let action_id =
                    memory::read_memory::<i32>(handle, addr + ACTION_ID_OFFSET).unwrap_or(0);
                data.action_id = action_id;

                // 怪物名称
                let monster_id =
                    memory::read_memory::<i32>(handle, addr + MONSTER_ID_OFFSET)
                        .unwrap_or(-1);
                data.monster_id = monster_id;
                data.monster_name = self.monster_names.get(&monster_id).copied();

                // 距离/角度
                let dx = player_pos.x - monster_pos.x;
                let dz = player_pos.z - monster_pos.z;
                data.dist_h = (dx * dx + dz * dz).sqrt();
                data.dist_v = (player_pos.y - monster_pos.y).abs();
                let dir = dx.atan2(dz) * (180.0 / PI);
                let dir = if dir < 0.0 { dir + 360.0 } else { dir };
                data.angle = (dir - monster_yaw + 360.0) % 360.0;

                // ── 怪物AI决策值读取（地址配置自 monster_ai_addresses.json）──
                let ai_config = self.monster_ai_configs.get(&monster_id);
                let ai_decision = ai_config.and_then(|config| {
                    memory::resolve_pointer(handle, self.game_base + config.base_offset, &config.pointer_offsets)
                        .and_then(|addr| {
                            Some((
                                memory::read_memory::<f32>(handle, addr + config.dist_field_offset)?,
                                memory::read_memory::<f32>(handle, addr + config.angle_field_offset)?,
                            ))
                        })
                });
                data.ai_dist = ai_decision.map(|(d, _)| d);
                data.ai_angle = ai_decision.map(|(_, a)| a);

                // 怪物血量
                if let Some(addr) = monster_addr {
                    if let Some(hp_ptr) = memory::read_memory::<u64>(handle, addr + 0x7670) {
                        if hp_ptr != 0 {
                            if let Some([max_hp, cur_hp]) =
                                memory::read_memory::<[f32; 2]>(handle, hp_ptr + 0x60)
                            {
                                if max_hp > 0.0 {
                                    data.monster_hp =
                                        Some(MonsterHp { current: cur_hp, max: max_hp });

                                    if self.last_hp > 0.0 && cur_hp < self.last_hp - 0.5 {
                                        let elapsed_str = data
                                            .quest_elapsed_ms
                                            .map(format_quest_time)
                                            .unwrap_or_else(|| "??".to_string());
                                        let pct = cur_hp / max_hp * 100.0;
                                        let delta = self.last_hp - cur_hp;
                                        self.logger.combat(format!(
                                            "[{}] 玩家攻击命中! 距离:{:.0} 角度:{:.1}° 血量:{:.0} ({:.1}%) 变化:-{:.0}",
                                            elapsed_str, data.dist_h, data.angle, cur_hp, pct, delta
                                        ));
                                    }
                                    self.last_hp = cur_hp;
                                }
                            }
                        }
                    }
                }

                // 黑龙下压值
                if monster_id == 101 {
                    data.counterattack_value = memory::resolve_pointer(handle, self.counterattack_base, &COUNTERATTACK_OFFSETS)
                        .and_then(|addr| memory::read_memory::<f32>(handle, addr))
                        .map(|v| if self.counterattack_scaled { v / 0.7 } else { v });
                    data.counterattack_scaled = self.counterattack_scaled;
                }

                // 招式名称
                data.action_name = lookup_action_name(monster_id, action_id);

                if monster_id == 101 && action_id == 179 {
                    self.counterattack_scaled = true;
                }

                if data.action_name.is_none() {
                    data.action_name_en = self.cached_action_name_en.as_ref()
                        .filter(|(id, _)| *id == action_id)
                        .map(|(_, name)| name.clone())
                        .or_else(|| {
                            let name = read_action_name_en(handle, addr, action_id);
                            if let Some(ref n) = name {
                                self.cached_action_name_en = Some((action_id, n.clone()));
                            }
                            name
                        });
                }

                // ── 动作变化检测 ──
                let (is_new_action, log_dist, log_angle) = if ai_config.is_some() {
                    // AI 模式：
                    //   AI值变了 → 新动作（用AI距离/角度）
                    //   AI值没变但中文招式名变了 → 新动作（用计算值，因为AI值没更新）
                    ai_decision
                        .map(|(d, a)| {
                            let ai_changed = self.last_logged_ai_dist.map_or(true, |ld| d != ld)
                                || self.last_logged_ai_angle.map_or(true, |la| a != la);
                            if ai_changed {
                                self.last_logged_ai_dist = Some(d);
                                self.last_logged_ai_angle = Some(a);
                                self.last_logged_ai_name = data.action_name;
                                (true, d, a)
                            } else {
                                let name_changed = data.action_name.is_some()
                                    && data.action_name != self.last_logged_ai_name;
                                if name_changed {
                                    self.last_logged_ai_name = data.action_name;
                                    (true, data.dist_h, data.angle)
                                } else {
                                    (false, 0.0, 0.0)
                                }
                            }
                        })
                        .unwrap_or((false, 0.0, 0.0))
                } else {
                    // 传统模式：ID变化 + 帧复位 + 防抖
                    let frame_reset =
                        memory::read_memory::<u64>(handle, addr + ACTION_STATE_OFFSET)
                            .and_then(|ptr| {
                                let frame =
                                    memory::read_memory::<f32>(handle, ptr + ACTION_CURRENT_FRAME)?;
                                Some((ptr, frame))
                            })
                            .map(|(ptr, frame)| {
                                let ptr_unchanged = self.last_action_state_ptr == ptr
                                    || self.last_action_state_ptr == 0;
                                self.last_action_state_ptr = ptr;
                                let reset =
                                    self.action_id_init && self.last_frame > 1.0 && frame < 0.5;
                                self.last_frame = frame;
                                reset && ptr_unchanged
                            })
                            .unwrap_or(false);

                    let id_changed = self.action_id_init && action_id != self.last_action_id;
                    let debounced = !id_changed
                        && self.last_action_change.elapsed() < Duration::from_millis(1700);
                    let mut candidate = (id_changed || frame_reset) && !debounced;

                    // 同中文名分组防抖（仅传统模式）
                    if candidate {
                        let current_name = data.action_name;
                        let same_name_group = current_name.is_some()
                            && current_name == self.last_grouped_name
                            && self.last_grouped_time.elapsed() < Duration::from_millis(1200);
                        if same_name_group {
                            self.last_grouped_time = Instant::now();
                            candidate = false;
                        } else {
                            self.last_grouped_name = current_name;
                            self.last_grouped_time = Instant::now();
                        }
                    }
                    (candidate, data.dist_h, data.angle)
                };

                if is_new_action {
                    self.last_action_change = Instant::now();
                    self.last_frame = 0.0;
                    self.flash_until = Instant::now() + Duration::from_millis(200);

                    let elapsed_str = data
                        .quest_elapsed_ms
                        .map(format_quest_time)
                        .unwrap_or_else(|| "??".to_string());
                    if let Some(hp) = data.monster_hp {
                        let pct = hp.current / hp.max * 100.0;
                        let action_name =
                            data.action_name.or(data.action_name_en.as_deref()).unwrap_or("未知");
                        self.logger.action_change(format!(
                            "[{}] 怪物动作变更! 距离:{:.0} 角度:{:.1}° 血量:{:.0} ({:.1}%) 动作ID:{} ({})",
                            elapsed_str, log_dist, log_angle, hp.current, pct, action_id, action_name
                        ), monster_id, action_id);
                    } else {
                        let action_name =
                            data.action_name.or(data.action_name_en.as_deref()).unwrap_or("未知");
                        self.logger.action_change(format!(
                            "[{}] 怪物动作变更! 距离:{:.0} 角度:{:.1}° 动作ID:{} ({})",
                            elapsed_str, log_dist, log_angle, action_id, action_name
                        ), monster_id, action_id);
                    }
                }
                self.last_action_id = action_id;
                self.action_id_init = true;
            } else {
                self.action_id_init = false;
                self.last_action_id = 0;
                self.last_frame = 0.0;
                self.last_action_state_ptr = 0;
                self.cached_action_name_en = None;
                self.counterattack_scaled = false;
            }

            data.flashing = Instant::now() < self.flash_until;
        }

        data
    }
}

/// 启动后台数据读取线程（20FPS），返回共享雷达数据
pub fn spawn_data_reader(logger: Logger) -> Arc<Mutex<RadarData>> {
    let shared = Arc::new(Mutex::new(RadarData::default()));
    let shared_clone = shared.clone();
    let monster_names = load_monster_names();
    let quest_names = load_quest_names();
    thread::spawn(move || {
        let mut reader = DataReader::new(monster_names, quest_names, logger);
        loop {
            thread::sleep(Duration::from_millis(4));
            let data = reader.tick();
            if let Ok(mut guard) = shared_clone.lock() {
                *guard = data;
            }
        }
    });
    shared
}

