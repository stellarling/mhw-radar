// mhw-radar-updater — 独立更新器
//
// 由 Tauri 面板 spawn_updater 启动，完成等待主程序退出、解压、覆盖、回滚、重启。
// 不依赖 Tauri / GUI，纯 CLI 程序。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

// ── 常量 ────────────────────────────────────────────────────

const ENGINE_NAME: &str = "mhw-radar.exe";
const UPDATER_NAME: &str = "mhw-radar-updater.exe";
const PANEL_NAME: &str = "MHW Radar.exe";
const LOG_NAME: &str = "mhw-radar-update.log";
const LOCK_NAME: &str = "mhw-radar-update.lock";

const POLL_INTERVAL_MS: u64 = 1000;
const WAIT_TIMEOUT_SECS: u64 = 90;
const COPY_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 500;

const MAX_ZIP_ENTRIES: usize = 2000;
const MAX_UNCOMPRESSED_BYTES: u64 = 600 * 1024 * 1024;
const MAX_SINGLE_FILE_BYTES: u64 = 300 * 1024 * 1024;

/// 更新包中遇到这些顶层目录时跳过，保护用户数据不被覆盖。
const USER_DATA_DIRS: &[&str] = &["logs", "log", "exports", "screenshots"];

// ── CLI Args ─────────────────────────────────────────────────

struct Args {
    app_dir: PathBuf,
    zip_path: PathBuf,
    parent_pid: u32,
    restart: PathBuf,
    self_temp_dir: Option<PathBuf>,
}

fn parse_args() -> Result<Args, String> {
    let raw: Vec<String> = std::env::args().collect();

    let mut app_dir: Option<PathBuf> = None;
    let mut zip_path: Option<PathBuf> = None;
    let mut parent_pid: Option<u32> = None;
    let mut restart: Option<PathBuf> = None;
    let mut self_temp_dir: Option<PathBuf> = None;

    let mut i = 1;
    while i < raw.len() {
        match raw[i].as_str() {
            "--app-dir" => {
                i += 1;
                app_dir = Some(PathBuf::from(raw.get(i).ok_or("--app-dir 缺少值")?));
            }
            "--zip" => {
                i += 1;
                zip_path = Some(PathBuf::from(raw.get(i).ok_or("--zip 缺少值")?));
            }
            "--parent-pid" => {
                i += 1;
                let s = raw.get(i).ok_or("--parent-pid 缺少值")?;
                parent_pid = Some(s.parse().map_err(|_| format!("parent-pid 不是有效整数: {}", s))?);
            }
            "--restart" => {
                i += 1;
                restart = Some(PathBuf::from(raw.get(i).ok_or("--restart 缺少值")?));
            }
            "--self-temp-dir" => {
                i += 1;
                self_temp_dir = Some(PathBuf::from(raw.get(i).ok_or("--self-temp-dir 缺少值")?));
            }
            "--log" => {
                if i + 1 < raw.len() && !raw[i + 1].starts_with("--") {
                    i += 1;
                }
            }
            other => return Err(format!("未知参数: {}", other)),
        }
        i += 1;
    }

    let app_dir = app_dir.ok_or("缺少 --app-dir")?;
    let zip_path = zip_path.ok_or("缺少 --zip")?;
    let parent_pid = parent_pid.ok_or("缺少 --parent-pid")?;
    let restart = restart.ok_or("缺少 --restart")?;

    if !app_dir.is_dir() {
        return Err(format!("--app-dir 不是有效目录: {}", app_dir.display()));
    }
    if !zip_path.is_file() {
        return Err(format!("--zip 不是有效文件: {}", zip_path.display()));
    }
    if !restart.is_file() {
        let resolved = if restart.is_absolute() {
            restart.clone()
        } else {
            app_dir.join(&restart)
        };
        if !resolved.is_file() {
            return Err(format!("--restart 不存在: {} (尝试: {})", restart.display(), resolved.display()));
        }
    }

    Ok(Args { app_dir, zip_path, parent_pid, restart, self_temp_dir })
}

// ── Logger ───────────────────────────────────────────────────

struct Logger {
    temp: std::fs::File,
    app_log: Option<std::fs::File>,
}

impl Logger {
    fn new(app_dir: &Path) -> Result<Self, String> {
        let temp_path = std::env::temp_dir().join(LOG_NAME);
        let temp = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&temp_path)
            .map_err(|e| format!("无法创建日志 {}: {}", temp_path.display(), e))?;

        let app_log_path = app_dir.join("logs").join(LOG_NAME);
        let app_log = if std::fs::create_dir_all(app_dir.join("logs")).is_ok() {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&app_log_path)
                .ok()
        } else {
            None
        };

        let mut logger = Logger { temp, app_log };
        logger.log(&format!("=== MHW Radar Updater 启动 === PID={}", std::process::id()));
        logger.log(&format!("app_dir:  {}", app_dir.display()));
        Ok(logger)
    }

    fn log(&mut self, msg: &str) {
        let line = format!("[{}] {}\r\n", timestamp(), msg);
        let _ = self.temp.write_all(line.as_bytes());
        let _ = self.temp.flush();
        if let Some(ref mut f) = self.app_log {
            let _ = f.write_all(line.as_bytes());
            let _ = f.flush();
        }
    }
}

fn timestamp() -> String {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let z = d / 86400 + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = y + if month <= 2 { 1 } else { 0 } as u64;

    let rem = d % 86400;
    let hour = rem / 3600;
    let min = (rem % 3600) / 60;
    let sec = rem % 60;

    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", year, month, day, hour, min, sec)
}

// ── 安装锁（LockGuard Drop 时自动释放） ─────────────────────

struct LockGuard {
    path: PathBuf,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn acquire_lock(app_dir: &Path, logger: &mut Logger) -> Result<LockGuard, String> {
    let lock_dir = app_dir.join("logs");
    std::fs::create_dir_all(&lock_dir)
        .map_err(|e| format!("无法创建日志目录用于安装锁: {}", e))?;

    let lock_path = lock_dir.join(LOCK_NAME);

    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path)
    {
        Ok(mut file) => {
            let pid = std::process::id();
            writeln!(file, "{}", pid)
                .map_err(|e| format!("无法写入安装锁 PID: {}", e))?;
            file.sync_all().ok();
            logger.log(&format!("安装锁已创建，PID={}", pid));
            Ok(LockGuard { path: lock_path })
        }
        Err(e) => {
            // 空文件或无法解析 PID 不视为 stale，防止并发覆盖
            let stale = std::fs::read_to_string(&lock_path)
                .ok()
                .and_then(|s| s.trim().parse::<u32>().ok())
                .map(|pid| !process_exists(pid))
                .unwrap_or(false);

            if stale {
                logger.log("检测到僵尸安装锁，移除后重试");
                let _ = std::fs::remove_file(&lock_path);
                return acquire_lock(app_dir, logger);
            }

            Err(format!("安装锁已存在，另一更新器可能正在运行: {}", e))
        }
    }
}

// ── 等待主程序退出 ──────────────────────────────────────────

fn wait_for_parent_exit(pid: u32, logger: &mut Logger) -> Result<(), String> {
    logger.log(&format!("等待主程序 (PID={}) 退出...", pid));

    for _ in 0..WAIT_TIMEOUT_SECS {
        if !process_exists(pid) {
            logger.log("主程序已正常退出");
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
    }

    logger.log(&format!("主程序在 {} 秒内未退出，尝试强制终止", WAIT_TIMEOUT_SECS));
    let output = Command::new("taskkill")
        .args(["/F", "/PID", &pid.to_string(), "/T"])
        .output()
        .map_err(|e| format!("无法强制终止主程序: {}", e))?;

    if output.status.success() {
        logger.log(&format!("主程序已强制终止 (PID={})", pid));
        std::thread::sleep(Duration::from_millis(1500));
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        logger.log(&format!("taskkill 输出: {}", stderr));
    }
    Ok(())
}

fn process_exists(pid: u32) -> bool {
    Command::new("tasklist")
        .args(["/FI", &format!("PID eq {}", pid), "/NH"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
        .unwrap_or(false)
}

// ── 关闭 engine ─────────────────────────────────────────────

fn close_engine(logger: &mut Logger) {
    logger.log("关闭 engine 进程...");
    match Command::new("taskkill")
        .args(["/F", "/IM", ENGINE_NAME, "/T"])
        .output()
    {
        Ok(o) if o.status.success() => logger.log(&format!("{} 已关闭", ENGINE_NAME)),
        Ok(o) => logger.log(&format!("关闭 {}: {}", ENGINE_NAME, String::from_utf8_lossy(&o.stderr))),
        Err(_) => logger.log("taskkill 调用失败，可能无 engine 进程在运行"),
    }
    std::thread::sleep(Duration::from_millis(1000));
}

// ── Staging ─────────────────────────────────────────────────

fn create_staging_dir(logger: &mut Logger) -> Result<PathBuf, String> {
    let pid = std::process::id();
    let staging = std::env::temp_dir().join(format!("mhw-radar-update-staging-{}", pid));

    if staging.exists() {
        std::fs::remove_dir_all(&staging)
            .map_err(|e| format!("无法清理旧 staging 目录: {}", e))?;
    }
    std::fs::create_dir_all(&staging)
        .map_err(|e| format!("无法创建 staging 目录: {}", e))?;

    logger.log(&format!("staging: {}", staging.display()));
    Ok(staging)
}

// ── 解压（防 Zip Slip + ZIP bomb）──────────────────────────

fn extract_zip_safe(zip_path: &Path, staging: &Path, logger: &mut Logger) -> Result<(), String> {
    logger.log(&format!("解压更新包 {} ...", zip_path.display()));

    let file = std::fs::File::open(zip_path)
        .map_err(|e| format!("无法打开更新包: {}", e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("更新包格式无效: {}", e))?;

    if archive.len() > MAX_ZIP_ENTRIES {
        return Err(format!(
            "更新包条目数 {} 超过上限 {}", archive.len(), MAX_ZIP_ENTRIES
        ));
    }

    let mut total_uncompressed: u64 = 0;
    let mut total_copied: u64 = 0;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)
            .map_err(|e| format!("读取条目 {} 失败: {}", i, e))?;

        let raw_name = entry.name().to_string();
        let safe_name = raw_name.replace('\\', "/");

        // Zip Slip 防护
        if safe_name.split('/').any(|c| c == "..") {
            return Err(format!("Zip Slip 攻击检测: {}", raw_name));
        }
        if Path::new(&raw_name).is_absolute() {
            return Err(format!("拒绝绝对路径条目: {}", raw_name));
        }

        // 大小校验
        let entry_size = entry.size();
        if entry_size > MAX_SINGLE_FILE_BYTES {
            return Err(format!(
                "单文件过大: {} ({} bytes > {} 上限)", raw_name, entry_size, MAX_SINGLE_FILE_BYTES
            ));
        }
        total_uncompressed += entry_size;
        if total_uncompressed > MAX_UNCOMPRESSED_BYTES {
            return Err(format!(
                "总解压大小 {} bytes 超过上限 {} bytes", total_uncompressed, MAX_UNCOMPRESSED_BYTES
            ));
        }

        let dst = staging.join(&safe_name);

        if entry.is_dir() {
            std::fs::create_dir_all(&dst)
                .map_err(|e| format!("无法创建目录 {}: {}", dst.display(), e))?;
        } else {
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("无法创建父目录 {}: {}", parent.display(), e))?;
            }
            let mut out = std::fs::File::create(&dst)
                .map_err(|e| format!("无法创建文件 {}: {}", dst.display(), e))?;
            let copied = std::io::copy(&mut entry, &mut out)
                .map_err(|e| format!("解压文件 {} 失败: {}", dst.display(), e))?;
            total_copied += copied;
        }
    }

    if total_copied > MAX_UNCOMPRESSED_BYTES {
        return Err(format!(
            "实际解压 {} bytes 超过上限 {} bytes", total_copied, MAX_UNCOMPRESSED_BYTES
        ));
    }

    logger.log(&format!("解压完成: {} 个条目, {} bytes", archive.len(), total_copied));
    Ok(())
}

// ── 查找包根目录 ────────────────────────────────────────────

fn find_package_root(staging: &Path, logger: &mut Logger) -> Result<PathBuf, String> {
    if staging.join(PANEL_NAME).exists() {
        logger.log(&format!("包结构: 扁平 ({} 在根目录)", PANEL_NAME));
        return Ok(staging.to_path_buf());
    }

    for entry in std::fs::read_dir(staging).map_err(|e| format!("读取 staging 失败: {}", e))? {
        let entry = entry.map_err(|e| format!("{}", e))?;
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            if entry.path().join(PANEL_NAME).exists() {
                logger.log(&format!("包结构: 嵌套子目录 ({})", entry.file_name().to_string_lossy()));
                return Ok(entry.path());
            }
        }
    }

    Err(format!("更新包中未找到 {}", PANEL_NAME))
}

// ── 校验（严格模式下 updater 必须存在）──────────────────────

fn require_file(path: &Path, label: &str) -> Result<(), String> {
    if path.is_file() {
        Ok(())
    } else {
        Err(format!("更新包缺少必需文件: {}", label))
    }
}

fn validate_package_root(package_root: &Path) -> Result<(), String> {
    require_file(&package_root.join(PANEL_NAME), PANEL_NAME)?;
    require_file(
        &package_root.join("resources").join("bin").join(ENGINE_NAME),
        "resources/bin/mhw-radar.exe",
    )?;
    require_file(
        &package_root.join("resources").join("bin").join(UPDATER_NAME),
        "resources/bin/mhw-radar-updater.exe",
    )?;
    Ok(())
}

// ── 备份（核心文件失败则中止）──────────────────────────────

fn backup_core_file(src: &Path, app_dir: &Path, backup: &Path, logger: &mut Logger) -> Result<(), String> {
    if !src.is_file() {
        return Ok(());
    }
    let rel = src.strip_prefix(app_dir).unwrap_or(src);
    let dst = backup.join(rel);
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("无法创建备份目录 {}: {}", parent.display(), e))?;
    }
    std::fs::copy(src, &dst)
        .map_err(|e| format!("备份失败 ({}): {}", rel.display(), e))?;
    logger.log(&format!("备份: {}", rel.display()));
    Ok(())
}

fn create_backup(app_dir: &Path, logger: &mut Logger) -> Result<PathBuf, String> {
    let pid = std::process::id();
    let backup = std::env::temp_dir().join(format!("mhw-radar-backup-{}", pid));

    if backup.exists() {
        std::fs::remove_dir_all(&backup).ok();
    }
    std::fs::create_dir_all(&backup)
        .map_err(|e| format!("无法创建备份目录: {}", e))?;

    logger.log(&format!("备份目录: {}", backup.display()));

    let core_files = [
        app_dir.join(PANEL_NAME),
        app_dir.join("resources").join("bin").join(ENGINE_NAME),
        app_dir.join("resources").join("bin").join(UPDATER_NAME),
    ];

    for src in &core_files {
        backup_core_file(src, app_dir, &backup, logger)?;
    }

    Ok(backup)
}

// ── 复制文件 ────────────────────────────────────────────────

fn copy_tree_with_retries(src: &Path, dst: &Path, logger: &mut Logger) -> Result<(), String> {
    if src.is_dir() {
        std::fs::create_dir_all(dst)
            .map_err(|e| format!("创建目录失败 {}: {}", dst.display(), e))?;

        for entry in std::fs::read_dir(src).map_err(|e| format!("读取目录失败 {}: {}", src.display(), e))? {
            let entry = entry.map_err(|e| format!("{}", e))?;
            copy_tree_with_retries(&entry.path(), &dst.join(entry.file_name()), logger)?;
        }
    } else if src.is_file() {
        for attempt in 1..=COPY_RETRIES {
            match std::fs::copy(src, dst) {
                Ok(_) => {
                    logger.log(&format!("复制: {} → {}", src.display(), dst.display()));
                    return Ok(());
                }
                Err(e) => {
                    if attempt < COPY_RETRIES {
                        logger.log(&format!("复制重试 {}/{} ({}): {}", attempt, COPY_RETRIES, src.display(), e));
                        std::thread::sleep(Duration::from_millis(RETRY_DELAY_MS));
                    } else {
                        return Err(format!("复制失败 ({}): {}", src.display(), e));
                    }
                }
            }
        }
    }

    Ok(())
}

fn copy_update_files(package_root: &Path, app_dir: &Path, logger: &mut Logger) -> Result<(), String> {
    logger.log("开始复制更新文件（merge 策略：只覆盖同名文件，不删除目标目录中的额外文件）...");

    for entry in std::fs::read_dir(package_root)
        .map_err(|e| format!("无法读取包目录: {}", e))?
    {
        let entry = entry.map_err(|e| format!("{}", e))?;
        let name = entry.file_name();
        let name_lower = name.to_string_lossy().to_ascii_lowercase();

        // 跳过用户数据目录：保护用户日志、截图和导出文件不被更新包覆盖
        if USER_DATA_DIRS.contains(&name_lower.as_str()) {
            logger.log(&format!("跳过用户数据目录: {}", name_lower));
            continue;
        }

        copy_tree_with_retries(&entry.path(), &app_dir.join(&name), logger)?;
    }

    logger.log("更新文件复制完成（merge 完成，用户文件不受影响）");
    Ok(())
}

// ── 回滚（递归） ─────────────────────────────────────────────

fn rollback(backup: &Path, app_dir: &Path, logger: &mut Logger) -> Result<(), String> {
    logger.log("开始回滚...");

    if !backup.exists() {
        return Err("备份目录不存在，无法回滚".to_string());
    }

    copy_tree_with_retries(backup, app_dir, logger)?;
    logger.log("回滚完成");
    Ok(())
}

// ── 重启 ─────────────────────────────────────────────────────

fn restart_app(restart: &Path, logger: &mut Logger) {
    logger.log(&format!("启动更新后的程序: {}", restart.display()));

    match Command::new(restart)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(child) => logger.log(&format!("新程序已启动, PID={}", child.id())),
        Err(e) => logger.log(&format!("启动新程序失败: {}", e)),
    }
}

// ── 清理（成功/失败策略不同） ─────────────────────────────

fn cleanup_after_update(
    staging: &Path,
    backup: Option<&Path>,
    zip_path: &Path,
    self_temp_dir: Option<&Path>,
    logger: &mut Logger,
    success: bool,
) {
    // staging 总是可删
    if staging.exists() {
        std::fs::remove_dir_all(staging).ok();
        logger.log("已清理 staging 目录");
    }

    // ZIP: 成功删，失败保留以便人工排查
    if success && zip_path.exists() {
        std::fs::remove_file(zip_path).ok();
        logger.log("已清理更新包 ZIP");
    } else if !success {
        logger.log(&format!("更新失败，保留更新包: {}", zip_path.display()));
    }

    // backup: 成功删；失败时保留以便人工恢复
    if let Some(bk) = backup {
        if bk.exists() {
            if success {
                std::fs::remove_dir_all(bk).ok();
                logger.log("已清理 backup 目录");
            } else {
                logger.log(&format!("更新失败，保留备份目录以便人工恢复: {}", bk.display()));
            }
        }
    }

    // self_temp_dir: 尝试清理，失败不算错误
    if let Some(dir) = self_temp_dir {
        if dir.exists() {
            std::fs::remove_dir_all(dir).ok();
            logger.log("已清理自身 temp 目录");
        }
    }

    // LockGuard Drop 自动删除锁文件，不需要手动释放
    logger.log("清理完成");
}

// ── 主流程 ───────────────────────────────────────────────────

/// 执行实际的更新步骤（等待退出、解压、校验、备份、复制、重启）。
/// 不负责清理——由 `run` 统一 cleanup。
fn run_update_steps(
    args: &Args,
    staging: &Path,
    backup: &mut Option<PathBuf>,
    logger: &mut Logger,
) -> Result<(), String> {
    wait_for_parent_exit(args.parent_pid, logger)?;
    close_engine(logger);

    extract_zip_safe(&args.zip_path, staging, logger)?;

    let package_root = find_package_root(staging, logger)?;
    validate_package_root(&package_root)?;
    logger.log("包结构校验通过");

    let bk = create_backup(&args.app_dir, logger)?;
    *backup = Some(bk);

    if let Err(e) = copy_update_files(&package_root, &args.app_dir, logger) {
        logger.log(&format!("复制失败: {}", e));
        // 回滚：成功则尝试重启旧版本，失败则只记录
        if let Some(ref b) = *backup {
            match rollback(b, &args.app_dir, logger) {
                Ok(()) => {
                    logger.log("回滚成功，尝试启动旧版本");
                    if args.restart.is_file() {
                        restart_app(&args.restart, logger);
                    }
                }
                Err(re) => {
                    logger.log(&format!("回滚失败，不自动重启: {}", re));
                }
            }
        }
        return Err(e);
    }

    restart_app(&args.restart, logger);
    Ok(())
}

fn run(args: Args, logger: &mut Logger) -> Result<(), String> {
    logger.log(&format!("zip_path:     {}", args.zip_path.display()));
    logger.log(&format!("parent_pid:   {}", args.parent_pid));
    logger.log(&format!("restart:      {}", args.restart.display()));
    if let Some(ref td) = args.self_temp_dir {
        logger.log(&format!("self_temp_dir: {}", td.display()));
    }

    // 安装锁（作用域结束自动释放）
    let _lock = acquire_lock(&args.app_dir, logger)?;

    let staging = create_staging_dir(logger)?;
    let mut backup: Option<PathBuf> = None;

    let result = run_update_steps(&args, &staging, &mut backup, logger);

    cleanup_after_update(
        &staging,
        backup.as_deref(),
        &args.zip_path,
        args.self_temp_dir.as_deref(),
        logger,
        result.is_ok(),
    );

    result
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("参数错误: {}", e);
            std::process::exit(1);
        }
    };

    let mut logger = match Logger::new(&args.app_dir) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("日志初始化失败: {}", e);
            std::process::exit(1);
        }
    };

    match run(args, &mut logger) {
        Ok(()) => {
            logger.log("=== MHW Radar Updater 完成 ===");
            std::process::exit(0);
        }
        Err(e) => {
            logger.log(&format!("=== MHW Radar Updater 失败: {} ===", e));
            std::process::exit(1);
        }
    }
}
