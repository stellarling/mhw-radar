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

use sha2::{Digest, Sha256};

const RADAR_EXE_NAME: &str = "mhw-radar.exe";
const UPDATER_EXE_NAME: &str = "mhw-radar-updater.exe";
const PANEL_EXE_NAME: &str = "MHW Radar.exe";

const MAX_UPDATE_ZIP_BYTES: u64 = 300 * 1024 * 1024;

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

fn spawn_radar_fast() -> Result<Child, String> {
    let path = radar_exe_path();

    if !path.exists() {
        return Err(format!("mhw-radar.exe not found at: {:?}", path));
    }

    let app_dir = app_root_dir();

    let mut cmd = Command::new(&path);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .current_dir(&app_dir)
        .env("MHW_RADAR_APP_DIR", &app_dir);

    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let child = cmd.spawn().map_err(|e| format!("failed to start mhw-radar.exe: {}", e))?;
    Ok(child)
}

fn spawn_radar() {
    let path = radar_exe_path();

    if !path.exists() {
        eprintln!("[launcher] mhw-radar.exe not found at: {:?}", path);
        return;
    }

    // 检查是否已通过 child handle 跟踪（面板的子进程）
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

    // 快速路径：直接启动，不做全局 taskkill
    match spawn_radar_fast() {
        Ok(child) => {
            let pid = child.id();
            if let Ok(mut guard) = RADAR_PROCESS.lock() {
                *guard = Some(child);
            }
            eprintln!("[launcher] mhw-radar.exe started, pid={}", pid);
        }
        Err(e) => {
            eprintln!("[launcher] first attempt failed: {}", e);
            eprintln!("[launcher] cleaning stale processes and retrying...");
            kill_radar_by_image_name();

            match spawn_radar_fast() {
                Ok(child) => {
                    let pid = child.id();
                    if let Ok(mut guard) = RADAR_PROCESS.lock() {
                        *guard = Some(child);
                    }
                    eprintln!("[launcher] mhw-radar.exe started after cleanup, pid={}", pid);
                }
                Err(e2) => {
                    eprintln!("[launcher] failed to start mhw-radar.exe after cleanup: {}", e2);
                }
            }
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
    let startup_begin = std::time::Instant::now();

    let _single_instance_guard = match acquire_single_instance() {
        Some(guard) => guard,
        None => {
            activate_existing_panel_window();
            return;
        }
    };

    eprintln!("[startup] single instance check: {:?}", startup_begin.elapsed());

    spawn_radar();

    eprintln!("[startup] engine spawn done: {:?}", startup_begin.elapsed());

    let run_result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            kill_engine,
            get_version,
            get_app_dir,
            get_temp_dir,
            fetch_update_text,
            download_update,
            spawn_updater,
            open_external_url,
        ])
        .setup(move |_app| {
            eprintln!("[startup] Tauri setup (window/WebView init): {:?}", startup_begin.elapsed());
            Ok(())
        })
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

fn is_allowed_external_host(host: &str) -> bool {
    matches!(
        host,
        "github.com" | "www.github.com" | "mhdatalab.com" | "www.mhdatalab.com"
    )
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

    let parsed = reqwest::Url::parse(&normalized)
        .map_err(|e| format!("链接无效: {}", e))?;

    if parsed.scheme() != "https" {
        return Err("只允许打开 HTTPS 链接".to_string());
    }

    let host = parsed.host_str().unwrap_or("");
    if !is_allowed_external_host(host) {
        return Err(format!("不允许打开该外部链接: {}", host));
    }

    Ok(normalized)
}

/// 使用系统默认浏览器打开允许的外部链接。
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

fn is_allowed_update_host(host: &str) -> bool {
    matches!(
        host,
        "github.com"
            | "objects.githubusercontent.com"
            | "github-releases.githubusercontent.com"
            | "release-assets.githubusercontent.com"
    )
}

/// 校验初始下载 URL：必须是 mhw-radar release 的 HTTPS 地址。
fn validate_initial_update_url(url: &str, expected_asset_name: Option<&str>) -> Result<reqwest::Url, String> {
    let parsed = reqwest::Url::parse(url).map_err(|e| format!("更新包地址无效: {}", e))?;

    if parsed.scheme() != "https" {
        return Err("更新包必须使用 HTTPS 地址".to_string());
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| "更新包地址缺少域名".to_string())?;

    if host != "github.com" {
        return Err(format!("更新包初始下载地址必须是 github.com: {}", host));
    }

    let path = parsed.path();
    if !path.starts_with("/stellarling/mhw-radar/releases/download/") {
        return Err("更新包地址不是 mhw-radar 的 Release 下载地址".to_string());
    }

    if let Some(expected_name) = expected_asset_name {
        if !path.ends_with(expected_name) {
            return Err(format!(
                "更新包地址文件名不匹配: 期望 {}",
                expected_name
            ));
        }
    }

    Ok(parsed)
}

/// 校验 redirect 后的 URL host（允许 GitHub CDN）。
fn validate_redirect_update_url(url: &reqwest::Url) -> Result<(), String> {
    let host = url
        .host_str()
        .ok_or_else(|| "跳转地址缺少域名".to_string())?;

    if !is_allowed_update_host(host) {
        return Err(format!("跳转到非可信域名: {}", host));
    }
    Ok(())
}

/// 校验下载目标路径：必须在 %TEMP%/mhw-radar-update-* 下，文件名是 .zip。
fn validate_update_dest(dest: &std::path::Path) -> Result<(), String> {
    let temp = std::env::temp_dir();
    let canonical_temp = temp.canonicalize().ok().unwrap_or_else(|| temp.clone());

    // 父目录 canonicalize
    let parent = dest.parent().ok_or("更新包目标路径无父目录")?;
    let canonical_parent = parent
        .canonicalize()
        .map_err(|_| format!("更新包目标父目录不存在: {}", parent.display()))?;

    if !canonical_parent.starts_with(&canonical_temp) {
        return Err("更新包必须在临时目录下".to_string());
    }

    let parent_name = canonical_parent
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    if !parent_name.starts_with("mhw-radar-update-") {
        return Err("更新包父目录名必须以 mhw-radar-update- 开头".to_string());
    }

    if dest.extension().and_then(|e| e.to_str()) != Some("zip") {
        return Err("更新包必须为 .zip 文件".to_string());
    }

    if safe_name(dest).contains("..") {
        return Err("更新包路径不允许 ..".to_string());
    }

    Ok(())
}

fn safe_name(p: &std::path::Path) -> String {
    p.to_string_lossy().replace('\\', "/")
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

fn sha256_file(path: &std::path::Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| format!("无法打开文件计算 SHA-256: {}", e))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| format!("读取文件计算 SHA-256 失败: {}", e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// 校验 expected_sha256 是否为合法的 64 位十六进制 SHA-256。
/// 不允许空字符串——Tauri command 是安全边界，不能依赖前端传合法值。
fn validate_expected_sha256(value: &str) -> Result<String, String> {
    let v = value.trim().to_ascii_lowercase();
    if v.len() != 64 || !v.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("expected_sha256 必须是 64 位十六进制 SHA-256".to_string());
    }
    Ok(v)
}

/// 使用 Rust HTTP 客户端获取更新相关的小文件内容（如 .sha256）。
/// 避免 WebView fetch() 因跨域/CDN 重定向导致 Failed to fetch。
#[tauri::command]
async fn fetch_update_text(url: String) -> Result<String, String> {
    let parsed = reqwest::Url::parse(&url)
        .map_err(|e| format!("校验文件地址无效: {}", e))?;

    if parsed.scheme() != "https" {
        return Err("校验文件必须使用 HTTPS 地址".to_string());
    }

    let host = parsed.host_str().unwrap_or("");
    if host != "github.com" {
        return Err(format!("校验文件地址不是 github.com: {}", host));
    }

    let client = reqwest::Client::builder()
        .user_agent(format!("MHW-Radar-Updater/{}", env!("CARGO_PKG_VERSION")))
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| format!("创建下载客户端失败: {}", e))?;

    let res = client
        .get(parsed)
        .send()
        .await
        .map_err(|e| format!("下载校验文件失败: {}", e))?;

    let status = res.status();
    if !status.is_success() {
        return Err(format!("下载校验文件失败: HTTP {}", status));
    }

    res.text()
        .await
        .map_err(|e| format!("读取校验文件失败: {}", e))
}

/// 使用 Rust HTTP 客户端直接下载 GitHub Release 附件。
#[tauri::command]
async fn download_update(
    window: Window,
    url: String,
    dest: String,
    expected_sha256: String,
) -> Result<DownloadReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        download_update_blocking(window, url, dest, expected_sha256)
    })
    .await
    .map_err(|e| format!("下载线程异常: {}", e))?
}

/// 真正执行阻塞下载的函数。只能在 blocking 线程中调用。
fn download_update_blocking(
    window: Window,
    url: String,
    dest: String,
    expected_sha256: String,
) -> Result<DownloadReport, String> {
    let dest_path = std::path::PathBuf::from(&dest);

    // 先确保目标父目录存在（后续 canonicalize 需要目录存在）
    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("无法创建临时目录: {}", e))?;
    }

    // 校验目标路径
    validate_update_dest(&dest_path)?;

    // 强制校验 expected_sha256，不允许空值绕过
    let expected_sha256 = validate_expected_sha256(&expected_sha256)?;

    // 校验初始 URL
    let asset_name = dest_path
        .file_name()
        .and_then(|n| n.to_str());
    let parsed_url = validate_initial_update_url(&url, asset_name)?;

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

        // 校验 redirect URL host
        validate_redirect_update_url(response.url())?;

        if !response.status().is_success() {
            last_err = format!("GitHub 返回 HTTP {}", response.status());
            continue;
        }

        // Content-Length 检查
        if let Some(cl) = response.content_length() {
            if cl > MAX_UPDATE_ZIP_BYTES {
                let _ = std::fs::remove_file(&part_path);
                return Err(format!(
                    "更新包过大: {} bytes (上限 {} bytes)",
                    cl, MAX_UPDATE_ZIP_BYTES
                ));
            }
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

            // 超出大小上限，立即中止
            if downloaded > MAX_UPDATE_ZIP_BYTES {
                drop(file);
                let _ = std::fs::remove_file(&part_path);
                return Err(format!(
                    "更新包过大: 已下载 {} bytes (上限 {} bytes)",
                    downloaded, MAX_UPDATE_ZIP_BYTES
                ));
            }

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

        // Content-Length 完整性校验
        if let Some(cl) = total {
            if size != cl {
                let _ = std::fs::remove_file(&part_path);
                return Err(format!(
                    "下载不完整: 大小 {} bytes, 期望 {} bytes",
                    size, cl
                ));
            }
        }

        std::fs::rename(&part_path, &dest_path).map_err(|e| {
            format!(
                "无法保存更新包到 {}: {}",
                dest_path.to_string_lossy(),
                e
            )
        })?;

        // SHA-256 校验（强制，之前已 validate）
        emit_download_progress(&window, size, Some(size), "正在校验更新包完整性...");
        let actual = sha256_file(&dest_path)?;
        if actual != expected_sha256 {
            let _ = std::fs::remove_file(&dest_path);
            return Err(format!(
                "更新包校验失败: expected {}, got {}",
                expected_sha256, actual
            ));
        }

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

/// 清理 %TEMP%/mhw-radar-updater-run-* 目录，删除失败不阻断。
fn cleanup_old_updater_run_dirs() {
    let temp = std::env::temp_dir();
    if let Ok(entries) = std::fs::read_dir(&temp) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with("mhw-radar-updater-run-") {
                let _ = std::fs::remove_dir_all(&path);
            }
        }
    }
}

/// 启动独立更新器 `mhw-radar-updater.exe` 并退出面板。
///
/// 先校验路径合法性，再将 updater 复制到临时目录后启动（使 updater 可覆盖自身）。
#[tauri::command]
fn spawn_updater(app_dir: String, zip_path: String) -> Result<(), String> {
    let app_dir = std::path::PathBuf::from(&app_dir);
    let actual_app_dir = app_root_dir();

    // 安全：禁止前端传任意 app_dir
    if canonicalize_lossy(&app_dir) != canonicalize_lossy(&actual_app_dir) {
        return Err("app_dir 与当前程序目录不一致".to_string());
    }

    let app_exe = app_dir.join(PANEL_EXE_NAME);
    let updater_exe = app_dir.join("resources").join("bin").join(UPDATER_EXE_NAME);

    if !app_exe.is_file() {
        return Err(format!("未找到主程序: {}", app_exe.to_string_lossy()));
    }
    if !updater_exe.is_file() {
        return Err(format!("未找到更新器: {}", updater_exe.to_string_lossy()));
    }

    let zip_path = std::path::PathBuf::from(&zip_path);
    validate_update_zip_path(&zip_path)?;

    // 清理残留的旧 updater 临时目录（删除失败不阻断）
    cleanup_old_updater_run_dirs();

    // 把 updater 复制到临时目录，使 updater 可覆盖 app 目录中自身
    let run_dir = std::env::temp_dir().join(format!(
        "mhw-radar-updater-run-{}-{}",
        std::process::id(),
        unix_millis()
    ));
    std::fs::create_dir_all(&run_dir)
        .map_err(|e| format!("无法创建更新器运行目录: {}", e))?;

    let temp_updater = run_dir.join(UPDATER_EXE_NAME);
    std::fs::copy(&updater_exe, &temp_updater)
        .map_err(|e| format!("无法复制更新器到临时目录: {}", e))?;

    let current_pid = std::process::id();

    let mut cmd = Command::new(&temp_updater);
    cmd.arg("--app-dir")
        .arg(&app_dir)
        .arg("--zip")
        .arg(&zip_path)
        .arg("--parent-pid")
        .arg(current_pid.to_string())
        .arg("--restart")
        .arg(&app_exe)
        .arg("--self-temp-dir")
        .arg(&run_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    cmd.spawn()
        .map_err(|e| format!("无法启动更新器: {}", e))?;

    eprintln!(
        "[updater] updater temp copy spawned: {} (original: {})",
        temp_updater.to_string_lossy(),
        updater_exe.to_string_lossy()
    );
    Ok(())
}

/// 规范化路径字符串，用于路径比较。
fn canonicalize_lossy(p: &std::path::Path) -> String {
    p.canonicalize()
        .unwrap_or_else(|_| p.to_path_buf())
        .to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_lowercase()
}

fn unix_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// 校验更新包路径：必须位于 %TEMP%/mhw-radar-update-* 下，且为 .zip。
fn validate_update_zip_path(zip_path: &std::path::Path) -> Result<(), String> {
    let temp = std::env::temp_dir();
    let canonical_temp = temp.canonicalize().ok().unwrap_or_else(|| temp.clone());

    let parent = zip_path.parent().ok_or("更新包路径无父目录")?;
    let canonical_parent = parent
        .canonicalize()
        .map_err(|_| format!("更新包父目录不存在: {}", parent.display()))?;

    if !canonical_parent.starts_with(&canonical_temp) {
        return Err("更新包必须在临时目录下".to_string());
    }

    let parent_name = canonical_parent
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    if !parent_name.starts_with("mhw-radar-update-") {
        return Err("更新包父目录名必须以 mhw-radar-update- 开头".to_string());
    }

    if zip_path.extension().and_then(|e| e.to_str()) != Some("zip") {
        return Err("更新包必须为 .zip 文件".to_string());
    }

    if !zip_path.is_file() {
        return Err(format!("更新包文件不存在: {}", zip_path.display()));
    }

    // 不允许 ..
    if zip_path.to_string_lossy().replace('\\', "/").contains("..") {
        return Err("更新包路径不允许 ..".to_string());
    }

    Ok(())
}