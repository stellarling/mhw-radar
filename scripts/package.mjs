// 打包脚本：在 npm run build 之后执行，输出 dist/MHW-Radar-<version>.zip
// 用法: node scripts/package.mjs

import { readFileSync, existsSync, copyFileSync, mkdirSync, writeFileSync } from "fs";
import { join, dirname } from "path";
import { execSync } from "child_process";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, "..");

// 读取版本号（以 Cargo.toml 为准）
const cargoToml = readFileSync(join(ROOT, "panel-ui", "src-tauri", "Cargo.toml"), "utf-8");
const version = cargoToml.match(/^version\s*=\s*"([^"]+)"/m)?.[1] ?? "0.0.0";

// 确保已构建
if (!existsSync(join(ROOT, "engine", "target", "release", "mhw-radar.exe"))) {
  console.log("Building mhw-radar.exe...");
  execSync("cargo build --release", { cwd: join(ROOT, "engine"), stdio: "inherit" });
}

// 定位 Tauri 面板 exe
// tauri build 的产物位于 bundle 目录下
const bundleDirs = [
  join(ROOT, "panel-ui", "src-tauri", "target", "release", "bundle", "nsis"),
  join(ROOT, "panel-ui", "src-tauri", "target", "release", "bundle", "msi"),
];

let panelExePath = null;
for (const dir of bundleDirs) {
  const candidate = join(dir, "MHW Radar.exe");
  if (existsSync(candidate)) {
    panelExePath = candidate;
    break;
  }
}

// 如果 bundle 没找到，试试 target/release/ 下的原始 exe（便携模式）
if (!panelExePath) {
  const raw = join(ROOT, "panel-ui", "src-tauri", "target", "release", "mhw-radar-panel.exe");
  if (existsSync(raw)) {
    panelExePath = raw;
  }
}

if (!panelExePath) {
  console.error("未找到 Tauri 面板 exe，请先运行 npm run build");
  process.exit(1);
}

// 创建分发目录
const distDir = join(ROOT, "dist", `MHW-Radar-v${version}`);
mkdirSync(distDir, { recursive: true });

// 复制面板 exe
copyFileSync(panelExePath, join(distDir, "MHW Radar.exe"));
console.log(`  → MHW Radar.exe`);

// 复制 mhw-radar.exe 到 resources/bin/ 子目录
const binDir = join(distDir, "resources", "bin");
mkdirSync(binDir, { recursive: true });
copyFileSync(join(ROOT, "engine", "target", "release", "mhw-radar.exe"), join(binDir, "mhw-radar.exe"));
console.log(`  → resources/bin/mhw-radar.exe`);

// 写入使用说明
const readme = `\
================================
  MHW Radar v${version} - 使用说明
================================

【如何启动】
直接双击文件夹根目录下的 MHW Radar.exe 即可。
程序会自动在后台启动引擎。

【注意事项】
1. 如果发现双击无效，请以管理员身份运行（右键 → 以管理员身份运行）
   ─ 读取游戏内存可能需要管理员权限
2. 杀毒软件可能误报
   ─ 本工具仅读取内存，不修改任何游戏数据
   ─ 如被杀毒软件拦截，请添加信任/排除项
3. 目前这一版本只在窗口化全屏模式下生效，后续会更新优化
4. 快捷键：Ctrl+Shift+U = 切换悬浮窗显示/隐藏

【文件说明】
MHW Radar.exe         主程序面板
resources/bin/mhw-radar.exe   数据读取引擎

【免责声明】
本工具为开源免费软件，仅供学习交流使用。
© CAPCOM CO., LTD. ALL RIGHTS RESERVED.
`;
writeFileSync(join(distDir, "使用说明.txt"), readme, "utf-8");
console.log(`  → 使用说明.txt`);

// 创建 zip（用 PowerShell 内置 Compress-Archive）
console.log("\nCreating zip...");
execSync(
  `powershell -NoProfile -Command "& { Compress-Archive -Path '${distDir}\\*' -DestinationPath '${join(ROOT, "dist", `MHW-Radar-v${version}.zip`)}' -Force }"`,
  { cwd: ROOT, stdio: "inherit" }
);

console.log(`\n✅ dist/MHW-Radar-v${version}.zip`);
