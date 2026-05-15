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
/// 如果你希望允许用户同时独立运行 engine，可删除 spawn_radar() 中对本函数的调用。
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
        .invoke_handler(tauri::generate_handler![
            kill_engine,
            get_version,
            get_app_dir,
            get_temp_dir,
            download_update,
            spawn_updater,
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

/// 通过 PowerShell 从 GitHub 下载更新包
#[tauri::command]
fn download_update(url: String, dest: String) -> Result<(), String> {
    let status = Command::new("powershell")
        .args(["-NoProfile", "-Command", &format!(
            "[Net.ServicePointManager]::SecurityProtocol = \
             [Net.SecurityProtocolType]::Tls12; \
             Invoke-WebRequest -Uri '{}' -OutFile '{}'",
            url, dest
        )])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("无法启动 PowerShell: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err("下载失败，请检查网络连接后重试".to_string())
    }
}

/// 生成更新 bat 并静默启动，然后面板退出
///
/// `app_dir` — MHW Radar.exe 所在目录
/// `zip_path` — 已下载到本地的更新包完整路径
#[tauri::command]
fn spawn_updater(app_dir: String, zip_path: String) -> Result<(), String> {
    let bat_content = format!(
        "@echo off\r\n\
         setlocal\r\n\
         \r\n\
         REM Wait for MHW Radar.exe to fully exit\r\n\
         ping 127.0.0.1 -n 4 > nul\r\n\
         \r\n\
         REM Kill engine if still alive\r\n\
         taskkill /f /im mhw-radar.exe 2>nul\r\n\
         \r\n\
         REM Unzip new version over application directory\r\n\
         powershell -NoProfile -Command \"& {{ Expand-Archive -Path '{}' -DestinationPath '{}' -Force }}\" 2>nul\r\n\
         \r\n\
         REM Clean up zip\r\n\
         del \"{}\" 2>nul\r\n\
         \r\n\
         REM Start new version\r\n\
         start \"\" \"{}\\MHW Radar.exe\"\r\n\
         \r\n\
         REM Self destruct\r\n\
         del \"%~f0\"\r\n",
        zip_path, app_dir, zip_path, app_dir
    );

    let bat_path = std::env::temp_dir().join("mhw-radar-update.bat");
    std::fs::write(&bat_path, bat_content).map_err(|e| e.to_string())?;

    let mut cmd = Command::new(&bat_path);
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());

    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    cmd.spawn().map_err(|e| e.to_string())?;

    eprintln!("[updater] bat spawned, exiting panel");
    Ok(())
}