// 打包脚本：在 npm run build 之后执行，输出 dist/MHW-Radar-<version>.zip
// 用法: node scripts/package.mjs

import { readFileSync, existsSync, copyFileSync, mkdirSync } from "fs";
import { join, dirname } from "path";
import { execSync } from "child_process";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, "..");

// 读取版本号
const tauriConf = JSON.parse(
  readFileSync(join(ROOT, "panel-ui", "src-tauri", "tauri.conf.json"), "utf-8")
);
const version = tauriConf.version;

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

// 复制 mhw-radar.exe 到 resources/bin/ 子目录（不污染用户视野）
const binDir = join(distDir, "resources", "bin");
mkdirSync(binDir, { recursive: true });
copyFileSync(join(ROOT, "engine", "target", "release", "mhw-radar.exe"), join(binDir, "mhw-radar.exe"));
console.log(`  → resources/bin/mhw-radar.exe`);

// 创建 zip（用 PowerShell 内置 Compress-Archive）
console.log("\nCreating zip...");
execSync(
  `powershell -NoProfile -Command "& { Compress-Archive -Path '${distDir}\\*' -DestinationPath '${join(ROOT, "dist", `MHW-Radar-v${version}.zip`)}' -Force }"`,
  { cwd: ROOT, stdio: "inherit" }
);

console.log(`\n✅ dist/MHW-Radar-v${version}.zip`);
