//! 程序入口
//!
//! 初始化日志、设置、后台数据读取线程、热键监听，
//! 然后启动 eframe 事件循环。
//! 悬浮窗渲染在 overlay 模块，工具面板在 panel 模块。

#![windows_subsystem = "windows"]

// 提示 NVIDIA Optimus / AMD PowerXpress 笔记本：
// 本程序使用高性能 GPU，避免 overlay 跑在核显上导致跨 GPU 合成锁 30 帧。
#[no_mangle]
pub static NvOptimusEnablement: u32 = 0x00000001;
#[no_mangle]
pub static AmdPowerXpressRequestHighPerformance: i32 = 1;

mod game_data;
mod ipc;
mod log;
mod memory;
mod overlay;
mod reader;
mod types;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use log::{ConnectionLogger, ConnectionLogStorage, Logger, LogStorage};
use overlay::RadarApp;
use reader::spawn_data_reader;
use types::Settings;
use winapi::um::timeapi::timeBeginPeriod;
use winapi::um::winuser::{
    GetMessageW, RegisterHotKey, MOD_CONTROL, MOD_SHIFT, MSG, WM_HOTKEY,
};

fn main() -> Result<(), eframe::Error> {
    unsafe { timeBeginPeriod(1); }

    if std::mem::size_of::<usize>() != 8 {
        eprintln!("============== 致命错误 ==============");
        eprintln!("游戏是64位的，但你的 Rust 编译成了 32位 程序！");
        eprintln!("这会导致指针截断，必定返回 0.00！");
        std::process::exit(1);
    }

    // 创建狩猎日志系统
    let (logger, logger_storage) = Logger::new();
    let logger_storage: Arc<Mutex<LogStorage>> = logger_storage;

    // 创建连接诊断日志系统
    let (connection_logger, connection_log_storage) = ConnectionLogger::new();
    let connection_log_storage: Arc<Mutex<ConnectionLogStorage>> = connection_log_storage;

    // 创建共享设置
    let settings = Arc::new(Mutex::new(Settings::default()));

    // 启动后台数据读取线程
    let shared_data = spawn_data_reader(logger, connection_logger);

    // 启动 IPC API 服务器（供 Tauri 面板调用）
    ipc::start_server(
        17320,
        settings.clone(),
        logger_storage,
        connection_log_storage,
        shared_data.clone(),
    );

    // 注册全局热键 Ctrl+Shift+U
    let hotkey_signal = Arc::new(AtomicBool::new(false));
    let thread_signal = hotkey_signal.clone();
    thread::spawn(move || {
        unsafe {
            const MOD_NOREPEAT: isize = 0x4000;
            RegisterHotKey(
                std::ptr::null_mut(),
                1,
                (MOD_CONTROL | MOD_SHIFT | MOD_NOREPEAT) as u32,
                'U' as u32,
            );
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) != 0 {
                if msg.message == WM_HOTKEY {
                    thread_signal.store(true, Ordering::SeqCst);
                }
            }
        }
    });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_transparent(true)
            .with_decorations(false)
            .with_always_on_top()
            .with_inner_size([280.0, 105.0])
            .with_position([80.0, 170.0])
            .with_taskbar(false),
        ..Default::default()
    };

    let app = RadarApp::with_hotkey_signal(hotkey_signal, shared_data, settings);

    eframe::run_native(
        "MHW Radar",
        options,
        Box::new(|cc| {
            let ctx = &cc.egui_ctx;
            let mut fonts = egui::FontDefinitions::default();
            if let Ok(data) = std::fs::read("C:\\Windows\\Fonts\\msyh.ttc") {
                fonts
                    .font_data
                    .insert("msyh".to_owned(), egui::FontData::from_owned(data));
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, "msyh".to_owned());
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, "msyh".to_owned());
            }
            ctx.set_fonts(fonts);
            Box::new(app)
        }),
    )
}
