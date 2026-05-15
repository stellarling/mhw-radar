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

点击悬浮窗左下角的齿轮图标，或在任务栏托盘图标上选择「打开面板」，即可打开设置面板：

- **基础工具**：开关各项显示内容（血量、距离、角度、招式名等）
- **狩猎日志**：按回合查看历史战斗记录，支持导出文本
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

### 开发模式

完整启动（编译 engine + Tauri 面板 + Vite 开发服务器）：

```powershell
npm run dev
```

仅启动前端（不改 Rust 代码时使用，跳过编译）：

```powershell
npm run dev:frontend
```

### 构建发布

```powershell
# 编译 + 打包为 zip
npm run package

# 产物输出到 dist/MHW-Radar-v<version>.zip
```

发布步骤：

1. 更新 `panel-ui/src-tauri/tauri.conf.json` 中的版本号
2. 执行 `npm run package`
3. 创建 git tag 并推送至 GitHub
4. 在 GitHub Releases 页面上传生成的 zip 文件

## 项目结构

```text
├── engine/                          # Rust 数据读取引擎 (mhw-radar.exe)
│   └── src/
│       ├── main.rs                  # 入口，初始化悬浮窗 + IPC 服务器
│       ├── memory.rs                # 底层内存 I/O（Windows API）
│       ├── reader.rs                # 游戏进程管理、数据读取、角度/血量/计时解析
│       ├── overlay.rs               # egui 透明悬浮窗渲染
│       ├── ipc.rs                   # HTTP API 服务器 (127.0.0.1:17320)
│       ├── log.rs                   # 回合制日志存储
│       ├── types.rs                 # 共享数据类型
│       ├── game_data.rs             # 招式/怪物/任务名称查表（编译时嵌入 JSON）
│       └── data/                    # JSON 数据文件
│           ├── action_names.json
│           ├── monster_names.json
│           ├── quest_names.json
│           └── monster_ai_addresses.json
│
├── panel-ui/                        # Tauri 设置面板 (MHW Radar.exe)
│   ├── src/                         # React + TypeScript 前端
│   │   ├── App.tsx                  # 主面板 UI
│   │   ├── components/              # UI 组件
│   │   └── constants.ts             # 常量与配置
│   └── src-tauri/                   # Tauri Rust 后端
│       └── src/main.rs              # 进程管理、IPC 绑定、更新逻辑
│
├── scripts/package.mjs              # 打包脚本
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
- 回合制日志系统：每次新任务自动开新回合，支持 100 轮 × 2000 条记录
- 覆盖式窗口不捕获鼠标/键盘焦点，不影响游戏操作

## 免责声明

本工具为开源免费软件，仅供学习交流使用，禁止用于商业用途。

所有游戏相关数据、美术素材及商标版权均归属 CAPCOM CO., LTD.
《怪物猎人：世界》《怪物猎人：世界·冰原》© CAPCOM CO., LTD. ALL RIGHTS RESERVED.

本工具仅提供信息呈现，不修改任何游戏文件或内存数据，所有解析均基于客户端只读方式，不影响游戏原有逻辑与网络通信。使用本工具所产生的任何后果由使用者自行承担。
