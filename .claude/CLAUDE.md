# MHW Radar Project

## Architecture
双进程架构，代码目录与进程一一对应：
- **`engine/`** — Rust 数据读取 + egui 游戏内透明悬浮窗（overlay），产出 `mhw-radar.exe`
- **`panel-ui/`** — React + TypeScript 设置/日志面板（Tauri），产出 `MHW Radar.exe`
- 两者通过 HTTP API (`127.0.0.1:17320`) 通信

## Dev Command
- **`npm run dev`**（根目录）— 完整启动：编译 engine + Tauri 面板 + Vite
- **`npm run dev:frontend`**（根目录）— 仅启动前端 Vite（不改 Rust 时用，跳过编译）
- **`cargo build --release --manifest-path engine/Cargo.toml`** — 仅编译 Rust 引擎
- **`cd engine && cargo build --release`** — 同上，简写
- 注意：只改前端代码时无需重启 dev，Vite 自动热更新；改 Rust 代码才需要重启
- 如果用户已在运行 `npm run dev`，助手验证时只需 `cargo build --release --manifest-path engine/Cargo.toml` + `npx tsc --noEmit`，无需重复启动 dev

## Build & Package
- **`npm run package`**（根目录）— 完整打包：编译 → 生成 zip → 输出到 `dist/MHW-Radar-v<version>.zip`
  - zip 内容：`MHW Radar.exe`（用户双击这个）+ `resources/bin/mhw-radar.exe`（内部依赖）
- **`npm run build`**（根目录）— 仅编译（生成 Tauri MSI 安装包）
- **`cargo build --release --manifest-path engine/Cargo.toml`** — 仅编译 mhw-radar.exe，产物在 `engine/target/release/mhw-radar.exe`
- 目标：x86_64-pc-windows-msvc

## Module Layout

### 后端 (`engine/`)
- `engine/src/memory.rs` — 低级内存 I/O (Vector3, read_memory, resolve_pointer)
- `engine/src/reader.rs` — 游戏进程管理、数据读取、角度计算、怪物血量、任务计时、招式变化检测
- `engine/src/overlay.rs` — egui 透明悬浮窗渲染
- `engine/src/main.rs` — 入口，初始化 eframe + 启动 IPC 服务器
- `engine/src/ipc.rs` — HTTP API 服务器 (std::net::TcpListener)，为 Tauri 面板提供 REST 接口
- `engine/src/log.rs` — 日志存储与分级、任务状态机
- `engine/src/types.rs` — 共享数据类型 (Settings, RadarData, PanelStatus, MonsterHp)
- `engine/src/game_data.rs` — 招式/怪物/任务名称查表 (编译时嵌入 JSON)

### 前端 (Tauri 面板)
- `panel-ui/` — Tauri v2 + React 19 + Vite + TypeScript
- `panel-ui/src/App.tsx` — 主面板 UI（设置开关/滑条、日志查看器、状态显示）
- `panel-ui/src-tauri/src/main.rs` — Tauri 后端，管理 mhw-radar.exe 生命周期
- `panel-ui/src-tauri/tauri.conf.json` — Tauri 配置

## Process Lifecycle
- Tauri 面板启动时 spawn mhw-radar.exe 为子进程
- 面板关闭时 kill 子进程
- mhw-radar.exe 也可独立运行（不带面板），但日志和设置需通过 API 访问

## Code Style
- Only comment when the `WHY` is non-obvious
- Offset constants describe the semantic field they point to (not the tool that discovered them)
