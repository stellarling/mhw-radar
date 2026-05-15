# MHW Radar

《怪物猎人：世界》实时狩猎辅助工具。双进程架构，通过读取游戏内存数据，在透明悬浮窗中实时显示怪物血量、位置、动作等信息，并配套完整的狩猎日志系统，用于战斗复盘与出招研究。

---

## 功能

- **实时悬浮窗**：怪物名称、血量（绝对值/百分比）、水平距离、高度落差、相对角度
- **动作识别**：怪物动作 ID 及中文招式名，招式变化检测与闪烁提示
- **黑龙下压**：显示黑龙下压值
- **狩猎日志**：回合制日志系统，自动记录每次攻击与怪物动作变更，支持分页查看与导出
- **UI 设置**：逐项开关显示内容，悬浮窗透明度与文字透明度独立调节
- **快捷键**：`Ctrl+Shift+U` 切换悬浮窗显示/隐藏
- **自动更新**：启动时检测新版本，一键下载更新

## 使用方法

### 快速开始

1. 启动游戏 `MonsterHunterWorld.exe`
2. 从 [Releases](https://github.com/stellarling/mhw-radar/releases) 下载最新压缩包并解压
3. 双击运行 `MHW Radar.exe`
4. 程序会自动在后台启动数据读取引擎

> 如果双击无效，请以管理员身份运行（右键 → 以管理员身份运行）。读取游戏内存可能需要管理员权限。

### 面板设置

半透明悬浮窗默认开启，如不需要可以快捷键关闭，打开设置面板后可以看到：

- **基础工具**：开关各项显示内容（血量、距离、角度、招式名等）
- **狩猎日志**：按场次查看历史战斗记录，支持导出文本
- **软件更新**：检查更新并自动升级

### 快捷键

| 快捷键         | 功能                |
| -------------- | ------------------- |
| `Ctrl+Shift+U` | 切换悬浮窗显示/隐藏 |

## 构建

### 前置要求

- [Rust](https://rustup.rs/)（stable）
- [Node.js](https://nodejs.org/) 20+
- 目标平台：`x86_64-pc-windows-msvc`（仅 Windows）

### 开发

```powershell
# 完整启动（编译 engine + Tauri 面板 + Vite）
npm run dev

# 仅启动前端（不改 Rust 时使用）
npm run dev:frontend
```

### 打包

```powershell
npm run package
# 产物：dist/MHW-Radar-v<version>.zip
```

## 项目结构

```text
├── engine/           # Rust 数据读取引擎 + egui 悬浮窗 (mhw-radar.exe)
├── panel-ui/         # Tauri 设置面板 (MHW Radar.exe)
├── scripts/          # 打包脚本
└── README.md
```

## 技术说明

### 双进程架构

程序由两个独立进程组成：

- **mhw-radar.exe**（引擎）：Rust 编写，负责内存读取与 egui 悬浮窗渲染。可独立运行（不带面板），但设置与日志需通过 HTTP API 访问。
- **MHW Radar.exe**（面板）：Tauri v2 外壳，启动时 spawn 引擎进程为子进程，关闭时自动清理。提供完整的设置界面与日志查看器。

两者通过 `127.0.0.1:17320` 的 REST API 通信。

### 关键依赖

| 依赖                   | 用途                               |
| ---------------------- | ---------------------------------- |
| `eframe` / `egui`      | 即时模式 GUI，实现游戏内透明悬浮窗 |
| `process-memory`       | Windows 跨进程内存读取             |
| `winapi`               | 全局热键注册                       |
| `serde` / `serde_json` | 编译时 JSON 数据解析               |
| Tauri v2               | 系统级窗口外壳与进程管理           |
| React 19               | 面板 UI 框架                       |
| Vite 6                 | 前端构建工具                       |

### 架构特点

- 数据采集与 UI 渲染分离：后台线程独占内存 I/O，主线程仅轮询共享缓存
- 招式名称查询使用 `HashMap`，编译时 `include_str!` 嵌入 JSON，启动时一次解析，运行时零 I/O
- 回合制日志系统：每次新任务自动开新回合，目前支持 100 轮 × 2000 条记录
- 覆盖式窗口不捕获鼠标/键盘焦点，不影响游戏操作

## 许可协议

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## 免责声明

本工具为开源免费软件，仅供学习交流使用。

所有游戏相关数据、美术素材及商标版权均归属 CAPCOM CO., LTD.
《怪物猎人：世界》《怪物猎人：世界·冰原》© CAPCOM CO., LTD. ALL RIGHTS RESERVED.

本工具仅提供信息呈现，不修改任何游戏文件或内存数据，所有解析均基于客户端只读方式，不影响游戏原有逻辑与网络通信。使用本工具所产生的任何后果由使用者自行承担。
