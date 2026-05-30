//! 结构化日志系统 + 任务状态机 + 分轮次存储
//!
//! 提供线程安全的日志记录，支持按任务轮次分组、
//! 自动检测任务生命周期（开始/结束），并记录战斗事件。
//! 日志以轮次（round）为单位存储，最多保留 100 轮，每轮最多 2000 条。

use std::collections::VecDeque;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// 最大保留轮次数
pub const MAX_ROUNDS: usize = 100;
/// 每轮最大日志条数
pub const MAX_LOG_ENTRIES: usize = 2000;
/// 连接诊断日志最大条数
pub const MAX_CONNECTION_ENTRIES: usize = 200;

/// 任务状态（来自游戏内存 offset 0x54）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum QuestState {
    None = 0,
    Ready = 1,
    InQuest = 2,
    Success = 3,
    Completed = 4,
    Failed = 5,
    Abandon = 6,
    Quit = 7,
}

impl QuestState {
    /// 该状态是否表示任务已结束
    pub fn is_over(self) -> bool {
        matches!(
            self,
            QuestState::Success
                | QuestState::Completed
                | QuestState::Failed
                | QuestState::Abandon
                | QuestState::Quit
        )
    }

    /// 从原始 i32 值转换
    pub const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => QuestState::None,
            1 => QuestState::Ready,
            2 => QuestState::InQuest,
            3 => QuestState::Success,
            4 => QuestState::Completed,
            5 => QuestState::Failed,
            6 => QuestState::Abandon,
            7 => QuestState::Quit,
            _ => QuestState::None,
        }
    }

    /// 友好的中文描述
    pub fn label(self) -> &'static str {
        match self {
            QuestState::Success | QuestState::Completed => "成功",
            QuestState::Failed => "失败",
            QuestState::Abandon => "放弃",
            QuestState::Quit => "退出",
            _ => "结束",
        }
    }
}

/// 日志级别
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum LogLevel {
    Info,
    #[allow(dead_code)]
    Warning,
    #[allow(dead_code)]
    Success,
    #[allow(dead_code)]
    Error,
    Combat,
    Quest,
}

/// 获取 UTC+8 当前时间的文件名友好格式：2026-05-16_14-30-25
pub fn format_datetime_utc8_for_filename() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let total_secs = now.as_secs() + 8 * 3600;
    let days = total_secs / 86400;
    let remaining = total_secs % 86400;
    let h = remaining / 3600;
    let m = (remaining / 60) % 60;
    let s = remaining % 60;
    let (y, mo, d) = days_to_ymd(days);
    format!("{:04}-{:02}-{:02}_{:02}-{:02}-{:02}", y, mo, d, h, m, s)
}

/// days since epoch → (year, month, day)
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    let z = days + 719468;
    let era = z / 146097;
    let doe = z % 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (yoe * 365 + yoe / 4 - yoe / 100);
    let mp = (doy * 5 + 2) / 153;
    let d = doy - (mp * 153 + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// 单条日志
#[derive(Clone, Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    #[allow(dead_code)]
    pub quest_id: Option<i32>,
    pub message: String,
    pub monster_id: Option<i32>,
    pub action_id: Option<i32>,
    /// 招式名称（中文优先，无翻译时用英文）
    pub action_name: Option<String>,
}

impl LogEntry {
    fn new(level: LogLevel, quest_id: Option<i32>, message: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let total_secs = now.as_secs() + 8 * 3600; // UTC+8
        let h = (total_secs / 3600) % 24;
        let m = (total_secs / 60) % 60;
        let s = total_secs % 60;
        let timestamp = format!("{:02}:{:02}:{:02}", h, m, s);
        Self {
            timestamp,
            level,
            quest_id,
            message,
            monster_id: None,
            action_id: None,
            action_name: None,
        }
    }
}

// ── 连接诊断日志 ────────────────────────────────────────────────

/// 连接事件类型
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ConnectionEventType {
    Waiting,
    Connected,
    Disconnected,
    Reconnected,
    ReadError,
    Info,
}

impl ConnectionEventType {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectionEventType::Waiting => "waiting",
            ConnectionEventType::Connected => "connected",
            ConnectionEventType::Disconnected => "disconnected",
            ConnectionEventType::Reconnected => "reconnected",
            ConnectionEventType::ReadError => "read_error",
            ConnectionEventType::Info => "info",
        }
    }
}

/// 单条连接诊断日志
#[derive(Clone, Serialize)]
pub struct ConnectionLogEntry {
    pub timestamp: String,
    pub event_type: ConnectionEventType,
    pub message: String,
    pub pid: Option<u32>,
    pub module_base: Option<u64>,
}

impl ConnectionLogEntry {
    fn new(event_type: ConnectionEventType, message: String, pid: Option<u32>, module_base: Option<u64>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let total_secs = now.as_secs() + 8 * 3600;
        let h = (total_secs / 3600) % 24;
        let m = (total_secs / 60) % 60;
        let s = total_secs % 60;
        let timestamp = format!("{:02}:{:02}:{:02}", h, m, s);
        Self {
            timestamp,
            event_type,
            message,
            pid,
            module_base,
        }
    }
}

/// 连接诊断日志存储
#[derive(Clone)]
pub struct ConnectionLogStorage {
    pub entries: VecDeque<ConnectionLogEntry>,
}

impl ConnectionLogStorage {
    pub fn new() -> Self {
        Self {
            entries: VecDeque::with_capacity(MAX_CONNECTION_ENTRIES),
        }
    }

    pub fn push(&mut self, entry: ConnectionLogEntry) {
        if self.entries.len() >= MAX_CONNECTION_ENTRIES {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    pub fn all_entries(&self) -> Vec<ConnectionLogEntry> {
        self.entries.iter().cloned().collect()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// 线程安全的连接诊断日志句柄
#[derive(Clone)]
pub struct ConnectionLogger {
    storage: std::sync::Arc<std::sync::Mutex<ConnectionLogStorage>>,
}

impl ConnectionLogger {
    pub fn new() -> (Self, std::sync::Arc<std::sync::Mutex<ConnectionLogStorage>>) {
        let storage = std::sync::Arc::new(std::sync::Mutex::new(ConnectionLogStorage::new()));
        let handle = Self {
            storage: storage.clone(),
        };
        (handle, storage)
    }

    pub fn log(&self, event_type: ConnectionEventType, message: impl Into<String>) {
        self.log_with(event_type, message.into(), None, None);
    }

    pub fn log_with(
        &self,
        event_type: ConnectionEventType,
        message: impl Into<String>,
        pid: Option<u32>,
        module_base: Option<u64>,
    ) {
        if let Ok(mut storage) = self.storage.lock() {
            storage.push(ConnectionLogEntry::new(event_type, message.into(), pid, module_base));
        }
    }
}

/// 任务统计数据（通过解析 Quest 级别日志消息生成）
#[derive(Clone, Serialize)]
pub struct QuestStat {
    pub quest_name: String,
    pub quest_id: i32,
    pub total: u32,
    pub success: u32,
    pub fail: u32,
    pub abandon: u32,
    pub avg_abandon_ms: u64,
    /// 平均完成时间（ms），仅成功任务
    pub avg_completion_ms: u64,
    /// 最快完成时间（ms）
    pub fastest_ms: u64,
    /// 最慢完成时间（ms）
    pub slowest_ms: u64,
    /// 最近 N 次完成耗时（最多 20 条），供前端展示
    pub recent_completions: Vec<u64>,
}

/// 怪物招式计数项
#[derive(Clone, Serialize)]
pub struct ActionCountItem {
    pub action_name: String,
    pub action_id: i32,
    pub count: u32,
}

/// 怪物招式统计
#[derive(Clone, Serialize)]
pub struct MonsterActionStats {
    pub monster_id: i32,
    pub monster_name: String,
    pub total_actions: u32,
    pub actions: Vec<ActionCountItem>,
}

/// 从 `M'Ss'CC` 格式解析毫秒（reader.rs format_quest_time 的逆运算）
fn parse_quest_elapsed(s: &str) -> u64 {
    let parts: Vec<&str> = s.split('\'').collect();
    if parts.len() == 3 {
        let minutes: u64 = parts[0].parse().unwrap_or(0);
        let seconds: u64 = parts[1].parse().unwrap_or(0);
        let centis: u64 = parts[2].parse().unwrap_or(0);
        minutes * 60_000 + seconds * 1_000 + centis * 10
    } else {
        0
    }
}

/// 日志存储（Mutex 内部数据），按轮次组织
#[derive(Clone)]
pub struct LogStorage {
    /// 每轮一个 VecDeque，尾部为最新轮次
    pub rounds: VecDeque<VecDeque<LogEntry>>,
}

impl LogStorage {
    pub fn new() -> Self {
        Self {
            rounds: VecDeque::new(),
        }
    }

    /// 开始新一轮次（任务开始时调用）。返回轮次索引（0-based）。
    ///
    /// 如果当前最新轮次还是空的，则直接复用它，避免清空日志后第一轮前面多出空页。
    pub fn new_round(&mut self) -> usize {
        if self.rounds.back().map(|r| r.is_empty()).unwrap_or(true) {
            if self.rounds.is_empty() {
                self.rounds.push_back(VecDeque::with_capacity(MAX_LOG_ENTRIES));
            }
            return self.rounds.len() - 1;
        }

        if self.rounds.len() >= MAX_ROUNDS {
            self.rounds.pop_front();
        }
        self.rounds.push_back(VecDeque::with_capacity(MAX_LOG_ENTRIES));
        self.rounds.len() - 1
    }

    /// 向当前（最新）轮次追加日志
    pub fn push(&mut self, entry: LogEntry) {
        if let Some(current) = self.rounds.back_mut() {
            if current.len() >= MAX_LOG_ENTRIES {
                current.pop_front();
            }
            current.push_back(entry);
        }
    }

    /// 获取指定轮次的日志，越界返回 None
    pub fn get_round(&self, index: usize) -> Option<&VecDeque<LogEntry>> {
        self.rounds.get(index)
    }

    /// 当前总轮次数
    pub fn round_count(&self) -> usize {
        self.rounds.len()
    }

    /// 清空所有日志
    pub fn clear(&mut self) {
        self.rounds.clear();
    }

    /// 合并所有轮次的日志（用于"导出全部"）
    pub fn all_entries(&self) -> Vec<LogEntry> {
        self.rounds.iter().flatten().cloned().collect()
    }

    /// 遍历所有日志，解析 Quest 级别消息，按任务名称统计完成情况。
    ///
    /// 返回按 total 降序排列的 Vec<QuestStat>。
    pub fn compute_quest_stats(&self) -> Vec<QuestStat> {
        use std::collections::HashMap;

        // quest_name → (quest_id, total, success, fail, abandon, abandon_ms_sum, abandon_count, completion_times)
        let mut stats: HashMap<String, (i32, u32, u32, u32, u32, u64, u32, Vec<u64>)> = HashMap::new();
        let mut current_quest: Option<(String, i32)> = None;

        for entry in self.rounds.iter().flatten() {
            if entry.level != LogLevel::Quest {
                continue;
            }

            // 任务开始: "[quest_name] 任务开始(ID:id)，本轮第n次任务"
            if let Some(start_body) = entry.message.strip_prefix('[') {
                if let Some(rest) = start_body.split_once("] ") {
                    let quest_name = rest.0.to_string();
                    if rest.1.contains("任务开始") {
                        // 提取 quest_id
                        let qid = rest.1
                            .split("ID:")
                            .nth(1)
                            .and_then(|s| s.split(')').next())
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0);
                        current_quest = Some((quest_name, qid));
                        continue;
                    }
                }
            }

            // 任务结束: "任务{label}！耗时 {time}"
            // label ∈ [成功, 失败, 放弃, 退出]
            if let Some(msg) = entry.message.strip_prefix("任务") {
                if let Some(end) = msg.split_once("！耗时 ") {
                    let label = end.0;
                    let elapsed_str = end.1;
                    let elapsed_ms = parse_quest_elapsed(elapsed_str);

                    let (name, qid) = match &current_quest {
                        Some((n, id)) => (n.clone(), *id),
                        None => ("未知".to_string(), 0),
                    };

                    let total_add = 1u32;
                    let (success_add, fail_add, abandon_add, abandon_ms_add, abandon_cnt_add) =
                        match label {
                            "成功" => (1u32, 0, 0, 0, 0),
                            "失败" => (0, 1u32, 0, 0, 0),
                            "放弃" => (0, 0, 1u32, elapsed_ms, 1u32),
                            _ => (0, 0, 0, 0, 0),
                        };

                    let entry = stats.entry(name).or_insert((qid, 0, 0, 0, 0, 0, 0, Vec::new()));
                    entry.0 = qid;
                    entry.1 += total_add;
                    entry.2 += success_add;
                    entry.3 += fail_add;
                    entry.4 += abandon_add;
                    entry.5 += abandon_ms_add;
                    entry.6 += abandon_cnt_add;
                    // 记录成功完成时间
                    if label == "成功" {
                        entry.7.push(elapsed_ms);
                    }

                    current_quest = None;
                    continue;
                }
            }

            // 任务中断: "任务中断！耗时 {time}" — 不计入完成，只清当前任务
            if entry.message.starts_with("任务中断") {
                current_quest = None;
                continue;
            }

            // 任务记录中断（连接丢失）：清当前任务
            if entry.message == "任务记录中断：游戏连接丢失" {
                current_quest = None;
                continue;
            }
        }

        let mut result: Vec<QuestStat> = stats
            .into_iter()
            .map(|(name, (qid, total, success, fail, abandon, abandon_ms_sum, abandon_cnt, completion_times))| {
                let avg = if abandon_cnt > 0 {
                    abandon_ms_sum / abandon_cnt as u64
                } else {
                    0
                };

                // 完成时间分析：排序后求极值，原始顺序保留最近的
                let sorted: Vec<u64> = {
                    let mut t = completion_times.clone();
                    t.sort_unstable();
                    t
                };
                let len = sorted.len() as u64;
                let avg_completion = if len > 0 {
                    sorted.iter().sum::<u64>() / len
                } else {
                    0
                };
                let fastest = sorted.first().copied().unwrap_or(0);
                let slowest = sorted.last().copied().unwrap_or(0);
                // 最多 20 条最近完成记录（chronological order）
                let recent: Vec<u64> = completion_times.iter().rev().take(20).copied().collect();

                QuestStat {
                    quest_name: name,
                    quest_id: qid,
                    total,
                    success,
                    fail,
                    abandon,
                    avg_abandon_ms: avg,
                    avg_completion_ms: avg_completion,
                    fastest_ms: fastest,
                    slowest_ms: slowest,
                    recent_completions: recent,
                }
            })
            .collect();

        result.sort_by(|a, b| b.total.cmp(&a.total).then_with(|| a.quest_name.cmp(&b.quest_name)));
        result
    }

    /// 遍历所有日志，按怪物 + 招式名称聚合出招次数。
    ///
    /// 返回按 total_actions 降序排列的 Vec<MonsterActionStats>。
    pub fn compute_action_stats(&self) -> Vec<MonsterActionStats> {
        use std::collections::HashMap;

        let monster_names = &crate::game_data::MONSTER_NAMES_CACHE;
        // monster_id → (monster_name, HashMap<action_name, (action_id, count)>)
        let mut stats: HashMap<i32, (String, HashMap<String, (i32, u32)>)> = HashMap::new();

        for entry in self.rounds.iter().flatten() {
            let (Some(monster_id), Some(action_id)) = (entry.monster_id, entry.action_id) else {
                continue;
            };

            let monster_name = monster_names
                .get(&monster_id)
                .copied()
                .unwrap_or("未知")
                .to_string();

            // 招式名称：优先使用日志中已解析的名称，否则查表
            let action_name = entry
                .action_name
                .clone()
                .unwrap_or_else(|| {
                    crate::game_data::lookup_action_name(monster_id, action_id)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("动作 #{}", action_id))
                });

            let (_, actions) = stats
                .entry(monster_id)
                .or_insert_with(|| (monster_name, HashMap::new()));

            let (_, count) = actions
                .entry(action_name)
                .or_insert((action_id, 0));
            *count += 1;
        }

        let mut result: Vec<MonsterActionStats> = stats
            .into_iter()
            .map(|(monster_id, (monster_name, actions))| {
                let total_actions: u32 = actions.values().map(|(_, c)| c).sum();
                let mut action_list: Vec<ActionCountItem> = actions
                    .into_iter()
                    .map(|(name, (act_id, count))| ActionCountItem {
                        action_name: name,
                        action_id: act_id,
                        count,
                    })
                    .collect();
                action_list.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.action_id.cmp(&b.action_id)));
                MonsterActionStats {
                    monster_id,
                    monster_name,
                    total_actions,
                    actions: action_list,
                }
            })
            .collect();

        result.sort_by(|a, b| b.total_actions.cmp(&a.total_actions).then_with(|| a.monster_id.cmp(&b.monster_id)));
        result
    }
}

/// 获取日志保存根目录。
///
/// 优先级：
/// 1. Tauri 面板启动 engine 时传入的 MHW_RADAR_APP_DIR；
/// 2. 当前工作目录中存在 MHW Radar.exe 时，使用当前工作目录；
/// 3. 如果是开发环境下的 engine/target/release/mhw-radar.exe，向上查找项目根目录；
/// 4. 回退到当前 engine exe 所在目录。
fn app_root_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("MHW_RADAR_APP_DIR") {
        let path = PathBuf::from(dir);
        if !path.as_os_str().is_empty() {
            return path;
        }
    }

    if let Ok(current_dir) = std::env::current_dir() {
        if current_dir.join("MHW Radar.exe").exists() {
            return current_dir;
        }

        if looks_like_project_root(&current_dir) {
            return current_dir;
        }
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let mut dir = exe_dir.to_path_buf();

            loop {
                if looks_like_project_root(&dir) {
                    return dir;
                }

                if !dir.pop() {
                    break;
                }
            }

            return exe_dir.to_path_buf();
        }
    }

    std::env::current_dir().unwrap_or_default()
}

fn looks_like_project_root(dir: &std::path::Path) -> bool {
    dir.join("package.json").exists()
        && dir.join("engine").exists()
        && dir.join("panel-ui").exists()
}

/// Windows 文件名安全处理。
fn sanitize_filename_part(value: &str) -> String {
    let mut out = String::with_capacity(value.len());

    for ch in value.chars() {
        match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => out.push('_'),
            c if c.is_control() => out.push('_'),
            c => out.push(c),
        }
    }

    let trimmed = out.trim().trim_matches('.').to_string();

    if trimmed.is_empty() {
        "untitled".to_string()
    } else {
        trimmed
    }
}

/// 线程安全的日志句柄
pub struct Logger {
    storage: std::sync::Arc<std::sync::Mutex<LogStorage>>,
}

impl Logger {
    pub fn new() -> (Self, std::sync::Arc<std::sync::Mutex<LogStorage>>) {
        let storage = std::sync::Arc::new(std::sync::Mutex::new(LogStorage::new()));
        let handle = Self {
            storage: storage.clone(),
        };
        (handle, storage)
    }

    /// 开始新一轮次（任务开始时调用）
    pub fn new_round(&self) -> usize {
        self.storage.lock().map(|mut s| s.new_round()).unwrap_or(0)
    }

    pub fn info(&self, msg: impl Into<String>) {
        self.push(LogLevel::Info, None, msg.into());
    }

    #[allow(dead_code)]
    pub fn success(&self, msg: impl Into<String>) {
        self.push(LogLevel::Success, None, msg.into());
    }

    #[allow(dead_code)]
    pub fn error(&self, msg: impl Into<String>) {
        self.push(LogLevel::Error, None, msg.into());
    }

    pub fn combat(&self, msg: impl Into<String>) {
        self.push(LogLevel::Combat, None, msg.into());
    }

    pub fn quest(&self, msg: impl Into<String>) {
        self.push(LogLevel::Quest, None, msg.into());
    }

    /// 记录带有怪物 ID、动作 ID 和招式名称的日志（用于前端高亮匹配和出招统计）
    pub fn action_change(&self, msg: impl Into<String>, monster_id: i32, action_id: i32, action_name: Option<String>) {
        let mut entry = LogEntry::new(LogLevel::Info, None, msg.into());
        entry.monster_id = Some(monster_id);
        entry.action_id = Some(action_id);
        entry.action_name = action_name;
        if let Ok(mut storage) = self.storage.lock() {
            storage.push(entry);
        }
    }

    pub fn push(&self, level: LogLevel, quest_id: Option<i32>, msg: String) {
        if let Ok(mut storage) = self.storage.lock() {
            storage.push(LogEntry::new(level, quest_id, msg));
        }
    }

    pub fn separator(&self) {
        self.push(LogLevel::Info, None, "─".repeat(50));
    }

    /// 将最新一轮日志保存到应用根目录的 logs/ 下，以任务开始时间命名。
    ///   <项目根目录>/logs/2026-05-16_14-30-25.txt
    pub fn save_latest_round(&self, start_time: &str) -> std::io::Result<String> {
        let storage = self.storage.lock().map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::Other, "failed to lock log storage")
        })?;

        let round = storage.rounds.back().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "no rounds")
        })?;

        if round.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "round is empty",
            ));
        }

        let mut content: String = round
            .iter()
            .map(|e| format!("[{}] [{:?}] {}", e.timestamp, e.level, e.message))
            .collect::<Vec<_>>()
            .join("\r\n");
        content.push_str("\r\n");

        let log_dir = app_root_dir().join("logs");
        std::fs::create_dir_all(&log_dir)?;

        let file_name = sanitize_filename_part(start_time);
        let file_path = log_dir.join(format!("{}.txt", file_name));

        std::fs::write(&file_path, content.as_bytes())?;

        Ok(file_path.to_string_lossy().to_string())
    }
}

impl Clone for Logger {
    fn clone(&self) -> Self {
        Self {
            storage: self.storage.clone(),
        }
    }
}