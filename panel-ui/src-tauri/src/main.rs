// Tauri 窗口外壳 —— 管理 mhw-radar.exe 生命周期 + 提供面板窗口

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const RADAR_EXE_NAME: &str = "mhw-radar.exe";

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

static RADAR_PROCESS: Mutex<Option<Child>> = Mutex::new(None);
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

/// 获取 mhw-radar.exe 路径（dev 模式下相对于源码目录）
fn radar_exe_path() -> std::path::PathBuf {
    // env!("CARGO_MANIFEST_DIR") = panel-ui/src-tauri/
    // 回退两级到项目根目录 → engine/target/release/mhw-radar.exe
    let dev_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
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
            .args(["/IM", RADAR_EXE_NAME, "/F"])
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
        .stderr(Stdio::null());

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

            eprintln!("[launcher] mhw-radar.exe started, pid={}", pid);
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
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
        .to_string_lossy()
        .to_string()
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

fn ps_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn cmd_set_value(value: &str) -> String {
    value.replace('"', "\"\"")
}

/// 通过 PowerShell 从 GitHub 下载更新包。
#[tauri::command]
fn download_update(url: String, dest: String) -> Result<(), String> {
    if !url.starts_with("https://github.com/") && !url.starts_with("https://objects.githubusercontent.com/") {
        return Err("更新包下载地址不是可信的 GitHub 地址".to_string());
    }

    if let Some(parent) = std::path::Path::new(&dest).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("无法创建临时目录: {}", e))?;
    }

    let _ = std::fs::remove_file(&dest);

    let script = format!(
        "$ErrorActionPreference = 'Stop'; \
         [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; \
         Invoke-WebRequest -Uri {} -OutFile {} -UseBasicParsing; \
         if (!(Test-Path -LiteralPath {})) {{ throw '更新包下载后不存在' }}; \
         if ((Get-Item -LiteralPath {}).Length -le 0) {{ throw '更新包为空文件' }}",
        ps_single_quote(&url),
        ps_single_quote(&dest),
        ps_single_quote(&dest),
        ps_single_quote(&dest),
    );

    let mut cmd = Command::new("powershell.exe");
    cmd.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
        .stdin(Stdio::null())
        .stdout(Stdio::null());

    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("无法启动 PowerShell 下载更新包: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            return Err("下载失败，请检查网络连接或 GitHub Release 附件名称".to_string());
        }
        return Err(format!("下载失败: {}", stderr));
    }

    let size = std::fs::metadata(&dest)
        .map_err(|e| format!("无法读取更新包: {}", e))?
        .len();

    if size == 0 {
        return Err("下载失败：更新包为空文件".to_string());
    }

    Ok(())
}

/// 生成更新 cmd 并静默启动，然后面板退出。
///
/// `app_dir` — MHW Radar.exe 所在目录
/// `zip_path` — 已下载到本地的更新包完整路径
#[tauri::command]
fn spawn_updater(app_dir: String, zip_path: String) -> Result<(), String> {
    let app_exe = std::path::Path::new(&app_dir).join("MHW Radar.exe");
    if !app_exe.exists() {
        return Err(format!("未找到主程序: {}", app_exe.to_string_lossy()));
    }

    if !std::path::Path::new(&zip_path).exists() {
        return Err(format!("未找到已下载的更新包: {}", zip_path));
    }

    let log_path = std::env::temp_dir().join("mhw-radar-update.log");
    let cmd_path = std::env::temp_dir().join("mhw-radar-update.cmd");

    let cmd_content = format!(
        "@echo off\r\n\
         setlocal EnableExtensions\r\n\
         set \"APP_DIR={}\"\r\n\
         set \"ZIP_PATH={}\"\r\n\
         set \"LOG_PATH={}\"\r\n\
         \r\n\
         echo [%date% %time%] MHW Radar updater started > \"%LOG_PATH%\"\r\n\
         echo APP_DIR=%APP_DIR% >> \"%LOG_PATH%\"\r\n\
         echo ZIP_PATH=%ZIP_PATH% >> \"%LOG_PATH%\"\r\n\
         \r\n\
         REM Wait for the current panel process to exit before replacing MHW Radar.exe.\r\n\
         for /l %%i in (1,1,60) do (\r\n\
           tasklist /fi \"imagename eq MHW Radar.exe\" | find /i \"MHW Radar.exe\" >nul\r\n\
           if errorlevel 1 goto panel_exited\r\n\
           timeout /t 1 /nobreak >nul\r\n\
         )\r\n\
         echo Panel process did not exit in time. Continue anyway. >> \"%LOG_PATH%\"\r\n\
         \r\n\
         :panel_exited\r\n\
         echo [%date% %time%] Panel exited. >> \"%LOG_PATH%\"\r\n\
         taskkill /f /im mhw-radar.exe >> \"%LOG_PATH%\" 2>>&1\r\n\
         \r\n\
         powershell.exe -NoProfile -ExecutionPolicy Bypass -Command \"& {{ $ErrorActionPreference = 'Stop'; Expand-Archive -LiteralPath $env:ZIP_PATH -DestinationPath $env:APP_DIR -Force }}\" >> \"%LOG_PATH%\" 2>>&1\r\n\
         if errorlevel 1 (\r\n\
           echo [%date% %time%] Expand-Archive failed. >> \"%LOG_PATH%\"\r\n\
           start \"\" notepad.exe \"%LOG_PATH%\"\r\n\
           exit /b 1\r\n\
         )\r\n\
         \r\n\
         del \"%ZIP_PATH%\" >> \"%LOG_PATH%\" 2>>&1\r\n\
         echo [%date% %time%] Starting updated app. >> \"%LOG_PATH%\"\r\n\
         start \"\" /d \"%APP_DIR%\" \"%APP_DIR%\\MHW Radar.exe\"\r\n\
         \r\n\
         del \"%~f0\" >nul 2>nul\r\n",
        cmd_set_value(&app_dir),
        cmd_set_value(&zip_path),
        cmd_set_value(&log_path.to_string_lossy()),
    );

    std::fs::write(&cmd_path, cmd_content)
        .map_err(|e| format!("无法写入更新脚本: {}", e))?;

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