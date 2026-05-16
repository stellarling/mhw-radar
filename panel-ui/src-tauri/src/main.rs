// Tauri 窗口外壳 —— 管理 mhw-radar.exe 生命周期 + 提供面板窗口

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs::File;
use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::ffi::c_void;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

use tauri::{Emitter, Window};

const RADAR_EXE_NAME: &str = "mhw-radar.exe";
const PANEL_EXE_NAME: &str = "MHW Radar.exe";

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(windows)]
const ERROR_ALREADY_EXISTS: u32 = 183;

#[cfg(windows)]
const SW_RESTORE: i32 = 9;

#[cfg(windows)]
const FLASHW_TRAY: u32 = 0x0000_0002;

#[cfg(windows)]
const FLASHW_TIMERNOFG: u32 = 0x0000_000C;

static RADAR_PROCESS: Mutex<Option<Child>> = Mutex::new(None);
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
type Handle = *mut c_void;

#[cfg(windows)]
#[repr(C)]
struct FLASHWINFO {
    cb_size: u32,
    hwnd: Handle,
    dw_flags: u32,
    u_count: u32,
    dw_timeout: u32,
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn CreateMutexW(
        lp_mutex_attributes: *mut c_void,
        b_initial_owner: i32,
        lp_name: *const u16,
    ) -> Handle;
    fn GetLastError() -> u32;
    fn CloseHandle(h_object: Handle) -> i32;
}

#[cfg(windows)]
#[link(name = "user32")]
extern "system" {
    fn FindWindowW(lp_class_name: *const u16, lp_window_name: *const u16) -> Handle;
    fn ShowWindow(h_wnd: Handle, n_cmd_show: i32) -> i32;
    fn SetForegroundWindow(h_wnd: Handle) -> i32;
    fn FlashWindowEx(pfwi: *mut FLASHWINFO) -> i32;
}

#[cfg(windows)]
struct SingleInstanceGuard {
    handle: Handle,
}

#[cfg(windows)]
impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        unsafe {
            if !self.handle.is_null() {
                let _ = CloseHandle(self.handle);
            }
        }
    }
}

#[cfg(windows)]
fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 面板单实例保护。
///
/// 使用 Local 命名空间：同一 Windows 登录会话内，不允许任意版本 / 任意目录的
/// MHW Radar 面板同时运行。这里不区分 release/dev，因为 release 包也不需要和 dev
/// 同时运行。
#[cfg(windows)]
fn acquire_single_instance() -> Option<SingleInstanceGuard> {
    let name = wide_null("Local\\MHW_RADAR_PANEL_SINGLE_INSTANCE");

    let handle = unsafe { CreateMutexW(std::ptr::null_mut(), 1, name.as_ptr()) };

    if handle.is_null() {
        eprintln!("[single-instance] CreateMutexW failed; aborting second launch defensively");
        return None;
    }

    let last_error = unsafe { GetLastError() };

    if last_error == ERROR_ALREADY_EXISTS {
        unsafe {
            let _ = CloseHandle(handle);
        }

        eprintln!("[single-instance] another MHW Radar panel instance is already running");
        return None;
    }

    Some(SingleInstanceGuard { handle })
}

#[cfg(not(windows))]
struct SingleInstanceGuard;

#[cfg(not(windows))]
fn acquire_single_instance() -> Option<SingleInstanceGuard> {
    Some(SingleInstanceGuard)
}

/// 当检测到已有实例时，唤醒旧面板窗口，然后当前进程退出。
///
/// 简单版实现：按窗口标题查找 Tauri 主窗口。当前项目窗口标题应为 "MHW Radar"。
/// 如果后续 tauri.conf.json 中的窗口标题改名，这里也要同步修改。
#[cfg(windows)]
fn activate_existing_panel_window() {
    let title = wide_null("MHW Radar");
    let hwnd = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };

    if hwnd.is_null() {
        eprintln!("[single-instance] existing instance found, but panel window was not found");
        return;
    }

    unsafe {
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let _ = SetForegroundWindow(hwnd);

        let mut flash = FLASHWINFO {
            cb_size: std::mem::size_of::<FLASHWINFO>() as u32,
            hwnd,
            dw_flags: FLASHW_TRAY | FLASHW_TIMERNOFG,
            u_count: 3,
            dw_timeout: 0,
        };

        let _ = FlashWindowEx(&mut flash);
    }

    eprintln!("[single-instance] activated existing MHW Radar panel window");
}

#[cfg(not(windows))]
fn activate_existing_panel_window() {}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadProgress {
    downloaded: u64,
    total: Option<u64>,
    percent: Option<f64>,
    message: String,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadReport {
    path: String,
    size: u64,
    elapsed_ms: u128,
}

/// 获取 mhw-radar.exe 路径（dev 模式下相对于源码目录）
fn radar_exe_path() -> std::path::PathBuf {
    // env!("CARGO_MANIFEST_DIR") = panel-ui/src-tauri/
    // 回退两级到项目根目录 → engine/target/release/mhw-radar.exe
    let dev_path = project_root_dir()
        .join("engine")
        .join("target")
        .join("release")
        .join(RADAR_EXE_NAME);

    if dev_path.exists() {
        return dev_path;
    }

    let exe_dir = std::env::current_exe()
        .unwrap_or_default()
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .to_path_buf();

    // 生产环境：与面板 exe 同目录 或 resources/ 子目录
    for candidate in &[
        exe_dir.join(RADAR_EXE_NAME),
        exe_dir.join("resources").join(RADAR_EXE_NAME),
        exe_dir.join("resources").join("bin").join(RADAR_EXE_NAME),
    ] {
        if candidate.exists() {
            return candidate.clone();
        }
    }

    exe_dir.join(RADAR_EXE_NAME)
}

/// 项目根目录。
///
/// 对 Tauri dev/build 均可用，因为 CARGO_MANIFEST_DIR 指向 panel-ui/src-tauri。
fn project_root_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// MHW Radar.exe 所在目录。
fn app_root_dir() -> std::path::PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    if exe_dir.join(PANEL_EXE_NAME).exists() {
        return exe_dir;
    }

    project_root_dir()
}

fn command_silent(program: &str) -> Command {
    let mut cmd = Command::new(program);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    cmd
}

/// 兜底清理可能由上一次异常退出留下的雷达进程。
///
/// 注意：这里按镜像名清理的是 mhw-radar.exe，不会匹配 Tauri 面板进程本身。
fn kill_radar_by_image_name() {
    #[cfg(windows)]
    {
        let _ = command_silent("taskkill")
            .args(["/IM", RADAR_EXE_NAME, "/T", "/F"])
            .status();
    }
}

/// 优先按当前 Child PID 杀进程树，避免只杀父进程导致子线程/子进程残留。
fn kill_radar_by_pid(pid: u32) {
    #[cfg(windows)]
    {
        let _ = command_silent("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .status();
    }
}

fn spawn_radar() {
    let path = radar_exe_path();

    if !path.exists() {
        eprintln!("[launcher] mhw-radar.exe not found at: {:?}", path);
        return;
    }

    let app_dir = app_root_dir();

    // 避免面板异常退出后，旧的悬浮窗进程继续占着 IPC 端口或 UI 资源。
    kill_radar_by_image_name();

    if let Ok(mut guard) = RADAR_PROCESS.lock() {
        if let Some(child) = guard.as_mut() {
            match child.try_wait() {
                Ok(None) => {
                    eprintln!("[launcher] mhw-radar.exe already running, skip spawn");
                    return;
                }
                Ok(Some(_)) | Err(_) => {
                    *guard = None;
                }
            }
        }
    }

    let mut cmd = Command::new(&path);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        // 关键：让 engine 的当前工作目录指向应用根目录。
        .current_dir(&app_dir)
        // 关键：让 engine 保存日志时优先使用 MHW Radar.exe 所在目录。
        .env("MHW_RADAR_APP_DIR", &app_dir);

    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    match cmd.spawn() {
        Ok(child) => {
            let pid = child.id();

            if let Ok(mut guard) = RADAR_PROCESS.lock() {
                *guard = Some(child);
            }

            eprintln!(
                "[launcher] mhw-radar.exe started, pid={}, app_dir={}",
                pid,
                app_dir.to_string_lossy()
            );
        }
        Err(e) => {
            eprintln!("[launcher] failed to start mhw-radar.exe: {}", e);
        }
    }
}

fn kill_radar() {
    // 前端 close 按钮会先 invoke kill_engine，随后窗口 close 事件还会再进一次。
    // 用全局开关保证关闭链路只执行一次，避免 kill/wait 竞态。
    if SHUTTING_DOWN.swap(true, Ordering::SeqCst) {
        return;
    }

    let child = if let Ok(mut guard) = RADAR_PROCESS.lock() {
        guard.take()
    } else {
        None
    };

    if let Some(mut child) = child {
        let pid = child.id();

        match child.try_wait() {
            Ok(Some(status)) => {
                eprintln!(
                    "[launcher] mhw-radar.exe already exited, pid={}, status={}",
                    pid, status
                );
            }
            Ok(None) => {
                kill_radar_by_pid(pid);

                // Child::wait 回收句柄。taskkill 已发出强制结束请求；
                // 如果进程已经被 taskkill 回收，wait 也可能直接返回。
                let _ = child.wait();

                eprintln!("[launcher] mhw-radar.exe stopped, pid={}", pid);
            }
            Err(e) => {
                eprintln!(
                    "[launcher] failed to query mhw-radar.exe state, pid={}, error={}",
                    pid, e
                );

                kill_radar_by_pid(pid);
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    // 兜底：处理 Child 句柄丢失、上一次面板异常退出留下的 orphan 进程。
    kill_radar_by_image_name();
}

fn main() {
    let _single_instance_guard = match acquire_single_instance() {
        Some(guard) => guard,
        None => {
            activate_existing_panel_window();
            return;
        }
    };

    spawn_radar();

    let run_result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            kill_engine,
            get_version,
            get_app_dir,
            get_temp_dir,
            download_update,
            spawn_updater,
            open_external_url,
        ])
        .on_window_event(|_window, event| {
            if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                kill_radar();
            }
        })
        .run(tauri::generate_context!());

    // 兜底：如果窗口事件没有触发，或者应用由系统/托盘/异常路径退出，
    // run 返回后仍然尝试清理一次。
    kill_radar();

    run_result.expect("error while running tauri application");
}

#[tauri::command]
fn kill_engine() {
    kill_radar();
}

/// 返回当前版本号（来自 Cargo.toml）
#[tauri::command]
fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// 返回 MHW Radar.exe 所在目录（用于 bat 知道在哪覆盖文件）
#[tauri::command]
fn get_app_dir() -> String {
    app_root_dir().to_string_lossy().to_string()
}

/// 返回系统临时目录
#[tauri::command]
fn get_temp_dir() -> String {
    std::env::temp_dir().to_string_lossy().to_string()
}

fn normalize_external_url(url: &str) -> Result<String, String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err("链接为空".to_string());
    }

    let normalized = if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
        trimmed.to_string()
    } else {
        format!("https://{}", trimmed.trim_start_matches('/'))
    };

    if !normalized.starts_with("https://github.com/") && normalized != "https://github.com" {
        return Err("只允许打开 GitHub 链接".to_string());
    }

    Ok(normalized)
}

/// 使用系统默认浏览器打开 GitHub 链接。
#[tauri::command]
fn open_external_url(url: String) -> Result<(), String> {
    let normalized = normalize_external_url(&url)?;

    #[cfg(windows)]
    {
        let status = command_silent("rundll32.exe")
            .args(["url.dll,FileProtocolHandler", &normalized])
            .status()
            .map_err(|e| format!("无法唤起系统浏览器: {}", e))?;

        if !status.success() {
            return Err("系统浏览器打开失败".to_string());
        }

        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        let status = Command::new("open")
            .arg(&normalized)
            .status()
            .map_err(|e| format!("无法唤起系统浏览器: {}", e))?;

        if !status.success() {
            return Err("系统浏览器打开失败".to_string());
        }

        return Ok(());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let status = Command::new("xdg-open")
            .arg(&normalized)
            .status()
            .map_err(|e| format!("无法唤起系统浏览器: {}", e))?;

        if !status.success() {
            return Err("系统浏览器打开失败".to_string());
        }

        return Ok(());
    }

    #[allow(unreachable_code)]
    Err("当前平台不支持自动打开链接".to_string())
}

fn cmd_set_value(value: &str) -> String {
    value.replace('"', "\"\"")
}

fn is_allowed_update_host(host: &str) -> bool {
    matches!(
        host,
        "github.com"
            | "objects.githubusercontent.com"
            | "github-releases.githubusercontent.com"
            | "release-assets.githubusercontent.com"
    )
}

fn validate_update_url(url: &str) -> Result<reqwest::Url, String> {
    let parsed = reqwest::Url::parse(url).map_err(|e| format!("更新包地址无效: {}", e))?;

    if parsed.scheme() != "https" {
        return Err("更新包必须使用 HTTPS 地址".to_string());
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| "更新包地址缺少域名".to_string())?;

    if !is_allowed_update_host(host) {
        return Err(format!("更新包下载地址不是可信的 GitHub 地址: {}", host));
    }

    Ok(parsed)
}

fn emit_download_progress(
    window: &Window,
    downloaded: u64,
    total: Option<u64>,
    message: impl Into<String>,
) {
    let percent = total
        .filter(|v| *v > 0)
        .map(|v| (downloaded as f64 / v as f64 * 100.0).min(100.0));

    let _ = window.emit(
        "update-download-progress",
        DownloadProgress {
            downloaded,
            total,
            percent,
            message: message.into(),
        },
    );
}

/// 使用 Rust HTTP 客户端直接下载 GitHub Release 附件。
#[tauri::command]
async fn download_update(window: Window, url: String, dest: String) -> Result<DownloadReport, String> {
    tauri::async_runtime::spawn_blocking(move || download_update_blocking(window, url, dest))
        .await
        .map_err(|e| format!("下载线程异常: {}", e))?
}

/// 真正执行阻塞下载的函数。只能在 blocking 线程中调用。
fn download_update_blocking(window: Window, url: String, dest: String) -> Result<DownloadReport, String> {
    let parsed_url = validate_update_url(&url)?;

    let dest_path = std::path::PathBuf::from(&dest);
    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("无法创建临时目录: {}", e))?;
    }

    let part_path = dest_path.with_extension("zip.part");
    let _ = std::fs::remove_file(&dest_path);
    let _ = std::fs::remove_file(&part_path);

    emit_download_progress(&window, 0, None, "正在连接 GitHub Release...");

    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(600))
        .redirect(reqwest::redirect::Policy::limited(10))
        .user_agent(format!("MHW-Radar-Updater/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| format!("无法创建下载客户端: {}", e))?;

    let started = Instant::now();
    let mut last_err = String::new();

    for attempt in 1..=3 {
        let attempt_message = if attempt == 1 {
            "正在下载更新包..."
        } else {
            "正在重试下载更新包..."
        };
        emit_download_progress(&window, 0, None, attempt_message);

        let request_started = Instant::now();
        let mut response = match client.get(parsed_url.clone()).send() {
            Ok(resp) => resp,
            Err(e) => {
                last_err = format!("连接 GitHub 失败: {}", e);
                continue;
            }
        };

        let final_url = response.url().clone();
        if let Some(host) = final_url.host_str() {
            if !is_allowed_update_host(host) {
                return Err(format!("GitHub 下载跳转到了非信任域名: {}", host));
            }
        } else {
            return Err("GitHub 下载跳转地址缺少域名".to_string());
        }

        if !response.status().is_success() {
            last_err = format!("GitHub 返回 HTTP {}", response.status());
            continue;
        }

        let total = response.content_length();
        let mut file =
            File::create(&part_path).map_err(|e| format!("无法创建临时更新包: {}", e))?;

        let mut downloaded = 0u64;
        let mut buf = [0u8; 64 * 1024];
        let mut last_emit = Instant::now();

        loop {
            let n = response
                .read(&mut buf)
                .map_err(|e| format!("下载中断: {}", e))?;

            if n == 0 {
                break;
            }

            file.write_all(&buf[..n])
                .map_err(|e| format!("写入更新包失败: {}", e))?;

            downloaded += n as u64;

            if last_emit.elapsed() >= Duration::from_millis(200) {
                let speed = downloaded as f64 / request_started.elapsed().as_secs_f64().max(0.001);
                let message = format!("正在下载更新包，速度约 {:.1} MB/s", speed / 1024.0 / 1024.0);
                emit_download_progress(&window, downloaded, total, message);
                last_emit = Instant::now();
            }
        }

        file.flush()
            .map_err(|e| format!("写入更新包失败: {}", e))?;
        drop(file);

        let size = std::fs::metadata(&part_path)
            .map_err(|e| format!("无法读取临时更新包: {}", e))?
            .len();

        if size == 0 {
            last_err = "下载失败：更新包为空文件".to_string();
            let _ = std::fs::remove_file(&part_path);
            continue;
        }

        std::fs::rename(&part_path, &dest_path).map_err(|e| {
            format!(
                "无法保存更新包到 {}: {}",
                dest_path.to_string_lossy(),
                e
            )
        })?;

        emit_download_progress(&window, size, Some(size), "更新包下载完成，准备安装...");

        return Ok(DownloadReport {
            path: dest_path.to_string_lossy().to_string(),
            size,
            elapsed_ms: started.elapsed().as_millis(),
        });
    }

    Err(if last_err.is_empty() {
        "下载失败，请检查网络连接或手动从 GitHub Releases 下载".to_string()
    } else {
        format!("下载失败：{}。请检查网络连接，或手动从 GitHub Releases 下载 ZIP。", last_err)
    })
}

/// 生成更新 cmd 并静默启动，然后面板退出。
///
/// `app_dir` — MHW Radar.exe 所在目录
/// `zip_path` — 已下载到本地的更新包完整路径
#[tauri::command]
fn spawn_updater(app_dir: String, zip_path: String) -> Result<(), String> {
    let app_exe = std::path::Path::new(&app_dir).join(PANEL_EXE_NAME);
    if !app_exe.exists() {
        return Err(format!("未找到主程序: {}", app_exe.to_string_lossy()));
    }

    if !std::path::Path::new(&zip_path).exists() {
        return Err(format!("未找到已下载的更新包: {}", zip_path));
    }

    let current_pid = std::process::id();
    let log_path = std::env::temp_dir().join("mhw-radar-update.log");
    let cmd_path = std::env::temp_dir().join("mhw-radar-update.cmd");
    let extract_dir = std::env::temp_dir().join("mhw-radar-update-extract");

    let cmd_content = format!(
        "@echo off\r\n\
         setlocal EnableExtensions\r\n\
         set \"APP_DIR={}\"\r\n\
         set \"ZIP_PATH={}\"\r\n\
         set \"LOG_PATH={}\"\r\n\
         set \"EXTRACT_DIR={}\"\r\n\
         set \"PANEL_PID={}\"\r\n\
         \r\n\
         echo [%date% %time%] MHW Radar updater started > \"%LOG_PATH%\"\r\n\
         echo APP_DIR=%APP_DIR% >> \"%LOG_PATH%\"\r\n\
         echo ZIP_PATH=%ZIP_PATH% >> \"%LOG_PATH%\"\r\n\
         echo PANEL_PID=%PANEL_PID% >> \"%LOG_PATH%\"\r\n\
         \r\n\
         REM First wait for the panel instance that spawned this updater.\r\n\
         for /l %%i in (1,1,90) do (\r\n\
           tasklist /fi \"PID eq %PANEL_PID%\" | find \"%PANEL_PID%\" >nul\r\n\
           if errorlevel 1 goto panel_exited\r\n\
           timeout /t 1 /nobreak >nul\r\n\
         )\r\n\
         echo [%date% %time%] Panel process did not exit in time, force killing PID %PANEL_PID%. >> \"%LOG_PATH%\"\r\n\
         taskkill /f /pid %PANEL_PID% /t >> \"%LOG_PATH%\" 2>>&1\r\n\
         timeout /t 1 /nobreak >nul\r\n\
         \r\n\
         :panel_exited\r\n\
         echo [%date% %time%] Panel exited. >> \"%LOG_PATH%\"\r\n\
         echo [%date% %time%] Closing all MHW Radar panel and engine processes before update. >> \"%LOG_PATH%\"\r\n\
         taskkill /f /im \"MHW Radar.exe\" /t >> \"%LOG_PATH%\" 2>>&1\r\n\
         taskkill /f /im mhw-radar.exe /t >> \"%LOG_PATH%\" 2>>&1\r\n\
         timeout /t 2 /nobreak >nul\r\n\
         \r\n\
         rmdir /s /q \"%EXTRACT_DIR%\" >> \"%LOG_PATH%\" 2>>&1\r\n\
         mkdir \"%EXTRACT_DIR%\" >> \"%LOG_PATH%\" 2>>&1\r\n\
         powershell.exe -NoProfile -ExecutionPolicy Bypass -Command \"& {{ $ErrorActionPreference = 'Stop'; Expand-Archive -LiteralPath $env:ZIP_PATH -DestinationPath $env:EXTRACT_DIR -Force }}\" >> \"%LOG_PATH%\" 2>>&1\r\n\
         if errorlevel 1 (\r\n\
           echo [%date% %time%] Expand-Archive failed. >> \"%LOG_PATH%\"\r\n\
           start \"\" notepad.exe \"%LOG_PATH%\"\r\n\
           exit /b 1\r\n\
         )\r\n\
         \r\n\
         if not exist \"%EXTRACT_DIR%\\MHW Radar.exe\" (\r\n\
           echo [%date% %time%] Extracted package missing MHW Radar.exe. >> \"%LOG_PATH%\"\r\n\
           start \"\" notepad.exe \"%LOG_PATH%\"\r\n\
           exit /b 1\r\n\
         )\r\n\
         \r\n\
         set \"COPY_OK=0\"\r\n\
         for /l %%i in (1,1,15) do (\r\n\
           echo [%date% %time%] Copy attempt %%i. >> \"%LOG_PATH%\"\r\n\
           taskkill /f /im \"MHW Radar.exe\" /t >> \"%LOG_PATH%\" 2>>&1\r\n\
           taskkill /f /im mhw-radar.exe /t >> \"%LOG_PATH%\" 2>>&1\r\n\
           timeout /t 1 /nobreak >nul\r\n\
           xcopy \"%EXTRACT_DIR%\\*\" \"%APP_DIR%\\\" /E /I /Y >> \"%LOG_PATH%\" 2>>&1\r\n\
           if not errorlevel 1 (\r\n\
             set \"COPY_OK=1\"\r\n\
             goto copy_done\r\n\
           )\r\n\
           echo [%date% %time%] Copy attempt %%i failed. >> \"%LOG_PATH%\"\r\n\
           timeout /t 1 /nobreak >nul\r\n\
         )\r\n\
         \r\n\
         :copy_done\r\n\
         if not \"%COPY_OK%\"==\"1\" (\r\n\
           echo [%date% %time%] File copy failed after retries. >> \"%LOG_PATH%\"\r\n\
           echo Please close all MHW Radar.exe and mhw-radar.exe processes, then try again. >> \"%LOG_PATH%\"\r\n\
           start \"\" notepad.exe \"%LOG_PATH%\"\r\n\
           exit /b 1\r\n\
         )\r\n\
         \r\n\
         rmdir /s /q \"%EXTRACT_DIR%\" >> \"%LOG_PATH%\" 2>>&1\r\n\
         del \"%ZIP_PATH%\" >> \"%LOG_PATH%\" 2>>&1\r\n\
         echo [%date% %time%] Starting updated app. >> \"%LOG_PATH%\"\r\n\
         start \"\" /d \"%APP_DIR%\" \"%APP_DIR%\\MHW Radar.exe\"\r\n\
         \r\n\
         del \"%~f0\" >nul 2>nul\r\n",
        cmd_set_value(&app_dir),
        cmd_set_value(&zip_path),
        cmd_set_value(&log_path.to_string_lossy()),
        cmd_set_value(&extract_dir.to_string_lossy()),
        current_pid,
    );

    std::fs::write(&cmd_path, cmd_content).map_err(|e| format!("无法写入更新脚本: {}", e))?;

    let mut cmd = Command::new(&cmd_path);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    cmd.spawn()
        .map_err(|e| format!("无法启动更新脚本: {}", e))?;

    eprintln!(
        "[updater] updater spawned: {}, log: {}",
        cmd_path.to_string_lossy(),
        log_path.to_string_lossy()
    );

    Ok(())
}