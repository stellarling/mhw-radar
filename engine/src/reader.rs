//! 游戏数据读取器：进程连接、怪物/玩家数据读取、角度计算、招式变化检测
//!
//! DataReader 在独立线程 20FPS 运行，通过共享雷达数据与 UI 线程通信。
//! 本模块不包含任何 UI 状态或渲染逻辑。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use process_memory::{ProcessHandle, TryIntoProcessHandle};
use sysinfo::System;

use crate::game_data::{load_monster_ai_configs, load_monster_names, load_quest_names, lookup_action_name};
use crate::log::{ConnectionEventType, ConnectionLogger, Logger, QuestState};
use crate::memory::{self, read_ptr_chain};
use crate::monster_scanner::MonsterScanner;
use crate::target_selector::{TargetContext, TargetSelector};
use crate::types::{MonsterData, RadarData};

// ── 内存偏移常量 ─────────────────────────────────────────────────

/// 玩家实体指针链偏移
const PLAYER_OFFSETS: [u64; 2] = [0x50, 0x0];
/// 任务数据指针链偏移
const QUEST_OFFSETS: [u64; 1] = [0x0];
/// 动作指针基址偏移
const ACTION_PTR_OFFSET: u64 = 0x61C8;
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

/// 每个怪物的独立动作/血量变化跟踪状态
#[derive(Clone)]
struct MonsterTickState {
    last_action_id: i32,
    action_id_init: bool,
    last_frame: f32,
    last_hp: f32,
    last_action_state_ptr: u64,
    cached_action_name_en: Option<(i32, String)>,
    last_grouped_name: Option<&'static str>,
    last_grouped_time: Instant,
    last_logged_ai_dist: Option<f32>,
    last_logged_ai_angle: Option<f32>,
    last_logged_ai_name: Option<&'static str>,
    last_action_change: Instant,
}

impl Default for MonsterTickState {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            last_action_id: 0,
            action_id_init: false,
            last_frame: 0.0,
            last_hp: 0.0,
            last_action_state_ptr: 0,
            cached_action_name_en: None,
            last_grouped_name: None,
            last_grouped_time: now - Duration::from_secs(10),
            last_logged_ai_dist: None,
            last_logged_ai_angle: None,
            last_logged_ai_name: None,
            last_action_change: now - Duration::from_secs(1),
        }
    }
}

/// 在后台线程运行，负责游戏进程连接、内存读取、动作检测
struct DataReader {
    handle: Option<ProcessHandle>,
    player_base: u64,
    quest_base: u64,
    counterattack_base: u64,
    game_base: u64,
    disconnect_frames: u32,
    /// 当前连接状态: "init" | "waiting" | "connected" | "disconnected"
    connection_state: String,
    /// 是否曾经成功连接过（用于判断首次连接 vs 重连）
    was_ever_connected: bool,
    /// 断线后下次重连是否应标记为 Reconnected（独立于 connection_state）
    needs_reconnect_event: bool,
    /// 游戏进程 PID
    pid: Option<u32>,
    /// 游戏进程模块基址
    module_base: Option<u64>,
    flash_until: Instant,
    quest_elapsed_frozen: Option<Duration>,
    counterattack_scaled: bool,
    /// 各怪物AI决策值的地址配置
    monster_ai_configs: HashMap<i32, crate::game_data::MonsterAiConfig>,
    monster_names: HashMap<i32, &'static str>,
    quest_names: HashMap<i32, &'static str>,
    logger: Logger,
    connection_logger: ConnectionLogger,
    prev_quest_state: QuestState,
    prev_quest_id: i32,
    quest_counter: i32,
    quest_start_time: Option<String>,
    // -- multi monster new fields --
    /// 各怪物的独立状态跟踪（key=怪物内存地址）
    monster_states: HashMap<u64, MonsterTickState>,
    monster_scanner: MonsterScanner,
    target_selector: TargetSelector,
}

impl DataReader {
    fn new(
        monster_names: HashMap<i32, &'static str>,
        quest_names: HashMap<i32, &'static str>,
        logger: Logger,
        connection_logger: ConnectionLogger,
    ) -> Self {
        let now = Instant::now();
        Self {
            handle: None,
            player_base: 0,
            quest_base: 0,
            counterattack_base: 0,
            game_base: 0,
            disconnect_frames: 0,
            connection_state: "init".to_string(),
            was_ever_connected: false,
            needs_reconnect_event: false,
            pid: None,
            module_base: None,
            flash_until: now - Duration::from_secs(1),
            quest_elapsed_frozen: None,
            counterattack_scaled: false,
            monster_ai_configs: load_monster_ai_configs(),
            monster_names,
            quest_names,
            logger,
            connection_logger,
            prev_quest_state: QuestState::None,
            prev_quest_id: 0,
            quest_counter: 0,
            quest_start_time: None,
            monster_states: HashMap::new(),
            monster_scanner: MonsterScanner::new(),
            target_selector: TargetSelector::new(),
        }
    }

    /// 扫描游戏进程并建立内存连接，通过 connection_state 去重
    fn try_attach_game(&mut self) {
        let mut sys = System::new_all();
        sys.refresh_all();
        if let Some(process) = sys.processes_by_exact_name("MonsterHunterWorld.exe").next() {
            let pid = process.pid().as_u32();
            if let Ok(handle) = pid.try_into_process_handle() {
                if let Some(base) = memory::get_module_base(pid) {
                    // 从非连接状态首次进入连接
                    let was_disconnected = self.connection_state != "connected";

                    self.handle = Some(handle);
                    self.player_base = base + 0x050139A0;
                    self.quest_base = base + 0x0500ED30;
                    self.counterattack_base = base + COUNTERATTACK_BASE_OFFSET;
                    self.game_base = base;
                    self.pid = Some(pid);
                    self.module_base = Some(base);
                    self.disconnect_frames = 0;

                    if was_disconnected {
                        let event_type = if self.needs_reconnect_event {
                            self.needs_reconnect_event = false;
                            ConnectionEventType::Reconnected
                        } else {
                            ConnectionEventType::Connected
                        };
                        self.connection_logger.log_with(
                            event_type,
                            format!("成功捕获游戏进程! PID: {}, 基址: 0x{:X}", pid, base),
                            Some(pid),
                            Some(base),
                        );
                        self.connection_state = "connected".to_string();
                        self.was_ever_connected = true;
                    }
                    return;
                }
            }
        }

        // 未找到进程时，只在状态变化时写一次
        if self.connection_state == "init" {
            // 初始状态 → 首次未找到，写一条 Waiting
            self.connection_logger.log(
                ConnectionEventType::Waiting,
                "未检测到游戏进程，等待游戏启动...",
            );
            self.connection_state = "waiting".to_string();
        } else if self.connection_state == "connected" || self.connection_state == "disconnected" {
            // 从连接/断线状态进入等待
            self.connection_logger.log(
                ConnectionEventType::Waiting,
                "未检测到游戏进程，等待游戏启动...",
            );
            self.connection_state = "waiting".to_string();
        }
        // connection_state == "waiting" 时不做任何事（已去重）
    }

    /// 一次数据读取 + 检测（每 50ms 调用一次）
    fn tick(&mut self) -> RadarData {
        let mut data = RadarData::default();

        if self.handle.is_some() {
            let handle = self.handle.unwrap();
            if memory::read_memory::<u16>(handle, self.game_base).is_none() {
                self.disconnect_frames += 1;
                if self.disconnect_frames >= 10 {
                    // 任务中断时输出语义摘要到狩猎日志，然后封口任务状态
                    if self.prev_quest_state == QuestState::InQuest && self.prev_quest_id > 0 {
                        self.logger.quest("任务记录中断：游戏连接丢失");
                        self.logger.separator();
                        if let Some(ref start) = self.quest_start_time {
                            let _ = self.logger.save_latest_round(start);
                        }
                        self.quest_start_time = None;
                        self.quest_elapsed_frozen = None;
                    }
                    // 封口任务会话，避免重连后继续污染旧 round 或触发重复中断日志
                    self.prev_quest_state = QuestState::None;
                    self.prev_quest_id = 0;

                    // 保存断线时的 PID/基址供诊断日志使用，然后清空当前状态
                    let old_pid = self.pid;
                    let old_base = self.module_base;
                    self.connection_logger.log_with(
                        ConnectionEventType::Disconnected,
                        "游戏进程退出，等待重连",
                        old_pid,
                        old_base,
                    );

                    self.handle = None;
                    self.pid = None;
                    self.module_base = None;
                    self.connection_state = "disconnected".to_string();
                    self.needs_reconnect_event = true;
                    self.monster_states.clear();
                    self.counterattack_scaled = false;
                    self.quest_elapsed_frozen = None;
                    self.flash_until = Instant::now() - Duration::from_secs(1);
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
            data.connection_state = self.connection_state.clone();
            data.pid = self.pid;
            data.module_base = self.module_base;

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
            data.in_quest = in_quest;
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
                    self.monster_states.clear();
                    self.flash_until = Instant::now() - Duration::from_secs(1);
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

            // -- multi monster scanning --
            if in_quest {
                let player_v3 = memory::Vector3 { x: player_pos.x, y: player_pos.y, z: player_pos.z };
                let monsters = self.monster_scanner.scan_all_monsters(
                    handle, self.game_base, player_v3, &self.monster_names,
                );
                data.monsters = monsters;

                if !data.monsters.is_empty() {
                    data.has_monster = true;

                    let locked_on_index = self.read_locked_on_index(handle).unwrap_or(-1);

                    let target_idx = self.target_selector.select_target(
                        &data.monsters,
                        &TargetContext { locked_on_index },
                    );

                    for m in data.monsters.iter_mut() {
                        m.is_locked_on = locked_on_index >= 0 && m.double_list_index == locked_on_index;
                    }
                    let is_multi = data.monsters.len() > 1;

                    if let Some(idx) = target_idx {
                        let target_monster_id = data.monsters[idx].monster_id;
                        let target_action_id = data.monsters[idx].action_id;
                        let target_addr = data.monsters[idx].addr;

                        let act_name = lookup_action_name(target_monster_id, target_action_id);
                        let act_name_en = if act_name.is_none() {
                            self.read_action_name_en_with_cache(handle, target_addr, target_action_id)
                        } else {
                            None
                        };

                        let ai_config = self.monster_ai_configs.get(&target_monster_id).cloned();
                        let ai_decision = ai_config.as_ref().and_then(|config| {
                            memory::resolve_pointer(handle, self.game_base + config.base_offset, &config.pointer_offsets)
                                .and_then(|addr| Some((
                                    memory::read_memory::<f32>(handle, addr + config.dist_field_offset)?,
                                    memory::read_memory::<f32>(handle, addr + config.angle_field_offset)?,
                                )))
                        });

                        if let Some(m) = data.monsters.get_mut(idx) {
                            m.is_target = true;

                            data.monster_id = m.monster_id;
                            data.monster_name = m.monster_name;
                            data.monster_hp = m.monster_hp;
                            data.dist_h = m.dist_h;
                            data.dist_v = m.dist_v;
                            data.angle = m.angle;
                            data.action_id = m.action_id;
                            data.action_name = act_name;
                            m.action_name = act_name;
                            data.action_name_en.clone_from(&act_name_en);
                            m.action_name_en = act_name_en;

                            if target_monster_id == 101 {
                                if target_action_id == 179 {
                                    self.counterattack_scaled = true;
                                }
                                let cv = memory::resolve_pointer(handle, self.counterattack_base, &COUNTERATTACK_OFFSETS)
                                    .and_then(|addr| memory::read_memory::<f32>(handle, addr))
                                    .map(|v| if self.counterattack_scaled { v / 0.7 } else { v });
                                data.counterattack_value = cv;
                                data.counterattack_scaled = self.counterattack_scaled;
                                m.counterattack_value = cv;
                                m.counterattack_scaled = self.counterattack_scaled;
                            } else {
                                self.counterattack_scaled = false;
                                data.counterattack_value = None;
                                data.counterattack_scaled = false;
                            }

                            data.ai_dist = ai_decision.map(|(d, _)| d);
                            data.ai_angle = ai_decision.map(|(_, a)| a);
                            m.ai_dist = data.ai_dist;
                            m.ai_angle = data.ai_angle;

                            let flashed = self.detect_target_changes(
                                handle, m, is_multi, ai_config, data.quest_elapsed_ms,
                            );
                            if flashed {
                                self.flash_until = Instant::now() + Duration::from_millis(200);
                            }
                        }
                    }

                    for m in &data.monsters {
                        if m.is_target { continue; }
                        self.detect_monster_action_change(handle, m, data.quest_elapsed_ms);
                    }

                    let active_addrs: Vec<u64> = data.monsters.iter().map(|m| m.addr).collect();
                    self.monster_states.retain(|addr, _| active_addrs.contains(addr));
                } else {
                    data.has_monster = false;
                }
            } else {
                data.monsters.clear();
                self.monster_states.clear();
                self.counterattack_scaled = false;
            }

            data.flashing = Instant::now() < self.flash_until;
        } else {
            // 未连接时，使用断线/try_attach_game 处理后的最新状态
            data.connection_state = self.connection_state.clone();
            data.pid = self.pid;
            data.module_base = self.module_base;
        }

        data
    }
}

// ── 锁目标读取 ──

const LOCKON_ADDR: u64 = 0x0500ECA0;
const LOCKED_INDEX_OFFSETS: [u64; 6] = [0x1618, 0x12608, 0x3340, 0x0, 0x48, 0x0];
const LOCKED_INDEX_FIELD: u64 = 0x950;

impl DataReader {
    fn read_locked_on_index(&self, handle: ProcessHandle) -> Option<i32> {
        memory::resolve_pointer(handle, self.game_base + LOCKON_ADDR, &LOCKED_INDEX_OFFSETS)
            .and_then(|addr| memory::read_memory::<i32>(handle, addr + LOCKED_INDEX_FIELD))
    }

    fn read_action_name_en_with_cache(
        &mut self,
        handle: ProcessHandle,
        monster_addr: u64,
        action_id: i32,
    ) -> Option<String> {
        for state in self.monster_states.values() {
            if let Some((id, ref name)) = state.cached_action_name_en {
                if id == action_id {
                    return Some(name.clone());
                }
            }
        }
        let name = read_action_name_en(handle, monster_addr, action_id);
        if name.is_some() {
            for state in self.monster_states.values_mut() {
                if state.cached_action_name_en.is_none() {
                    state.cached_action_name_en = Some((action_id, name.clone().unwrap()));
                    break;
                }
            }
        }
        name
    }

    fn detect_target_changes(
        &mut self,
        handle: ProcessHandle,
        monster: &MonsterData,
        is_multi: bool,
        ai_config: Option<crate::game_data::MonsterAiConfig>,
        quest_elapsed_ms: Option<u64>,
    ) -> bool {
        let state = self.monster_states.entry(monster.addr).or_default();
        let action_id = monster.action_id;
        let mut flashed = false;

        let static_name = monster.action_name;
        let display_name = static_name.or(monster.action_name_en.as_deref()).unwrap_or("未知");
        let is_new_action = if ai_config.is_some() {
            let ai_dist = monster.ai_dist;
            let ai_angle = monster.ai_angle;
            let ai_changed = ai_dist.is_some()
                && (state.last_logged_ai_dist.map_or(true, |ld| ld != ai_dist.unwrap())
                    || state.last_logged_ai_angle.map_or(true, |la| la != ai_angle.unwrap()));
            if ai_changed {
                state.last_logged_ai_dist = ai_dist;
                state.last_logged_ai_angle = ai_angle;
                state.last_logged_ai_name = static_name;
                true
            } else {
                let name_changed = static_name.is_some()
                    && static_name != state.last_logged_ai_name;
                if name_changed {
                    state.last_logged_ai_name = static_name;
                    true
                } else {
                    false
                }
            }
        } else {
            let frame_reset = memory::read_memory::<u64>(handle, monster.addr + ACTION_STATE_OFFSET)
                .and_then(|ptr| {
                    let frame = memory::read_memory::<f32>(handle, ptr + ACTION_CURRENT_FRAME)?;
                    Some((ptr, frame))
                })
                .map(|(ptr, frame)| {
                    let ptr_unchanged = state.last_action_state_ptr == ptr
                        || state.last_action_state_ptr == 0;
                    state.last_action_state_ptr = ptr;
                    let reset = state.action_id_init && state.last_frame > 1.0 && frame < 0.5;
                    state.last_frame = frame;
                    reset && ptr_unchanged
                })
                .unwrap_or(false);

            let id_changed = state.action_id_init && action_id != state.last_action_id;
            let debounced = !id_changed
                && state.last_action_change.elapsed() < Duration::from_millis(1700);
            let mut candidate = (id_changed || frame_reset) && !debounced;

            if candidate {
                let same_name_group = static_name.is_some()
                    && static_name == state.last_grouped_name
                    && state.last_grouped_time.elapsed() < Duration::from_millis(1200);
                if same_name_group {
                    state.last_grouped_time = Instant::now();
                    candidate = false;
                } else {
                    state.last_grouped_name = static_name;
                    state.last_grouped_time = Instant::now();
                }
            }
            candidate
        };

        if is_new_action {
            state.last_action_change = Instant::now();
            state.last_frame = 0.0;
            flashed = true;

            let elapsed = quest_elapsed_ms.map(format_quest_time).unwrap_or_else(|| "??".to_string());
            let prefix = if is_multi {
                format!("[{}] ", monster.monster_name.unwrap_or("未知"))
            } else {
                String::new()
            };
            let name_owned = display_name.to_string();

            if let Some(hp) = monster.monster_hp {
                let pct = hp.current / hp.max * 100.0;
                self.logger.action_change(
                    format!("{}[{}] 怪物动作变更! 距离:{:.0} 角度:{:.1}° 血量:{:.0} ({:.1}%) 动作ID:{} ({})",
                        prefix, elapsed, monster.dist_h, monster.angle, hp.current, pct, action_id, display_name),
                    monster.monster_id, action_id, Some(name_owned),
                );
            } else {
                self.logger.action_change(
                    format!("{}[{}] 怪物动作变更! 距离:{:.0} 角度:{:.1}° 动作ID:{} ({})",
                        prefix, elapsed, monster.dist_h, monster.angle, action_id, display_name),
                    monster.monster_id, action_id, Some(name_owned),
                );
            }
        }

        state.last_action_id = action_id;
        state.action_id_init = true;

        // HP change detection
        if let Some(hp) = monster.monster_hp {
            if state.last_hp > 0.0 && hp.current < state.last_hp - 0.5 {
                let elapsed = quest_elapsed_ms.map(format_quest_time).unwrap_or_else(|| "??".to_string());
                let prefix = if is_multi {
                    format!("[{}] ", monster.monster_name.unwrap_or("未知"))
                } else {
                    String::new()
                };
                let pct = hp.current / hp.max * 100.0;
                let delta = state.last_hp - hp.current;

                let counter_text = if monster.monster_id == 101 {
                    monster.counterattack_value
                        .map(|v| if monster.counterattack_scaled {
                            format!(" 下压值(换算):{:.0}", v)
                        } else {
                            format!(" 下压值:{:.0}", v)
                        })
                        .unwrap_or_else(|| " 下压值:??".to_string())
                } else {
                    String::new()
                };

                self.logger.combat_with_monster(
                    format!(
                        "{}[{}] 玩家攻击命中! 距离:{:.0} 角度:{:.1}° 血量:{:.0} ({:.1}%) 变化:-{:.0}{}",
                        prefix, elapsed, monster.dist_h, monster.angle, hp.current, pct, delta, counter_text,
                    ),
                    monster.monster_id,
                    monster.monster_name.map(|s| s.to_string()),
                );
            }
            state.last_hp = hp.current;
        } else {
            state.last_hp = 0.0;
        }

        flashed
    }

    fn detect_monster_action_change(
        &mut self,
        handle: ProcessHandle,
        monster: &MonsterData,
        quest_elapsed_ms: Option<u64>,
    ) {
        let action_id = monster.action_id;
        let addr = monster.addr;
        // 优先查表名，其次读内存英文名兜底
        let fallback_name = monster.action_name.or_else(|| {
            monster.action_name_en.as_deref()
        }).map(|s| s.to_string()).or_else(|| {
            self.read_action_name_en_with_cache(handle, addr, action_id)
        });

        let state = self.monster_states.entry(addr).or_default();

        if state.action_id_init && action_id != state.last_action_id {
            let name_ref = fallback_name.as_deref().unwrap_or("未知");
            let elapsed = quest_elapsed_ms.map(format_quest_time).unwrap_or_else(|| "??".to_string());
            let prefix = format!("[{}] ", monster.monster_name.unwrap_or("未知"));

            let msg = if let Some(hp) = monster.monster_hp {
                let pct = hp.current / hp.max * 100.0;
                format!("{}[{}] 怪物动作变更! 距离:{:.0} 角度:{:.1}° 血量:{:.0} ({:.1}%) 动作ID:{} ({})",
                    prefix, elapsed, monster.dist_h, monster.angle, hp.current, pct, action_id, name_ref)
            } else {
                format!("{}[{}] 怪物动作变更! 距离:{:.0} 角度:{:.1}° 动作ID:{} ({})",
                    prefix, elapsed, monster.dist_h, monster.angle, action_id, name_ref)
            };

            self.logger.action_change(
                msg,
                monster.monster_id, action_id, Some(name_ref.to_string()),
            );
        }

        state.last_action_id = action_id;
        state.action_id_init = true;
    }
}

/// 启动后台数据读取线程（20FPS），返回共享雷达数据
pub fn spawn_data_reader(
    logger: Logger,
    connection_logger: ConnectionLogger,
) -> Arc<Mutex<RadarData>> {
    let shared = Arc::new(Mutex::new(RadarData::default()));
    let shared_clone = shared.clone();
    let monster_names = load_monster_names();
    let quest_names = load_quest_names();
    thread::spawn(move || {
        let mut reader = DataReader::new(monster_names, quest_names, logger, connection_logger);
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

