//! 结构化日志系统 + 任务状态机 + 分轮次存储
//!
//! 提供线程安全的日志记录，支持按任务轮次分组、
//! 自动检测任务生命周期（开始/结束），并记录战斗事件。
//! 日志以轮次（round）为单位存储，最多保留 100 轮，每轮最多 2000 条。

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

/// 最大保留轮次数
pub const MAX_ROUNDS: usize = 100;
/// 每轮最大日志条数
pub const MAX_LOG_ENTRIES: usize = 2000;

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
        }
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
        let mut rounds = VecDeque::new();
        rounds.push_back(VecDeque::with_capacity(MAX_LOG_ENTRIES));
        Self { rounds }
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

    /// 清空所有日志，重置为 1 个空轮次
    pub fn clear(&mut self) {
        self.rounds.clear();
        self.rounds.push_back(VecDeque::with_capacity(MAX_LOG_ENTRIES));
    }

    /// 合并所有轮次的日志（用于"导出全部"）
    pub fn all_entries(&self) -> Vec<LogEntry> {
        self.rounds.iter().flatten().cloned().collect()
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

    /// 记录带有怪物 ID 和动作 ID 的日志（用于前端高亮匹配）
    pub fn action_change(&self, msg: impl Into<String>, monster_id: i32, action_id: i32) {
        let mut entry = LogEntry::new(LogLevel::Info, None, msg.into());
        entry.monster_id = Some(monster_id);
        entry.action_id = Some(action_id);
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

    /// 将最新一轮日志保存到 exe 所在目录的 logs/ 下，以任务开始时间命名
    pub fn save_latest_round(&self, start_time: &str) -> std::io::Result<String> {
        let storage = self.storage.lock().map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::Other, "failed to lock log storage")
        })?;

        let round = storage.rounds.back().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "no rounds")
        })?;

        if round.is_empty() {
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "round is empty"));
        }

        let mut content: String = round.iter()
            .map(|e| format!("[{}] [{:?}] {}", e.timestamp, e.level, e.message))
            .collect::<Vec<_>>()
            .join("\r\n");
        content.push_str("\r\n");

        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let log_dir = exe_dir.join("logs");
        std::fs::create_dir_all(&log_dir)?;

        let file_path = log_dir.join(format!("{}.txt", start_time));
        std::fs::write(&file_path, &content)?;

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

