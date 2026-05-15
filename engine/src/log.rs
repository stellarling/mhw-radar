//! 结构化日志系统 + 任务状态机
//!
//! 提供线程安全的日志记录，支持按任务分组、
//! 自动检测任务生命周期（开始/结束），并记录战斗事件。
//! 日志存储在内存环形缓冲区中，供 UI 面板读取显示。

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

/// 最大保留日志条数
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
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
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


/// 单条日志
#[derive(Clone, Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    #[allow(dead_code)]
    pub quest_id: Option<i32>,
    pub message: String,
}

impl LogEntry {
    fn new(level: LogLevel, quest_id: Option<i32>, message: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let total_secs = now.as_secs();
        let h = (total_secs / 3600) % 24;
        let m = (total_secs / 60) % 60;
        let s = total_secs % 60;
        let timestamp = format!("{:02}:{:02}:{:02}", h, m, s);
        Self {
            timestamp,
            level,
            quest_id,
            message,
        }
    }
}

/// 日志存储（Mutex 内部数据）
#[derive(Clone)]
pub struct LogStorage {
    pub entries: VecDeque<LogEntry>,
}

impl LogStorage {
    pub fn new() -> Self {
        Self {
            entries: VecDeque::with_capacity(MAX_LOG_ENTRIES),
        }
    }

    pub fn push(&mut self, entry: LogEntry) {
        if self.entries.len() >= MAX_LOG_ENTRIES {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
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

    pub fn push(&self, level: LogLevel, quest_id: Option<i32>, msg: String) {
        if let Ok(mut storage) = self.storage.lock() {
            storage.push(LogEntry::new(level, quest_id, msg));
        }
    }

    pub fn separator(&self) {
        self.push(LogLevel::Info, None, "─".repeat(50));
    }
}

impl Clone for Logger {
    fn clone(&self) -> Self {
        Self {
            storage: self.storage.clone(),
        }
    }
}
