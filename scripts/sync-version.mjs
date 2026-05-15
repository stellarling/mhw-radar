// 构建前同步版本号：读取 panel-ui/src-tauri/Cargo.toml，写入 tauri.conf.json
import { readFileSync, writeFileSync } from "fs";
import { join, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, "..");
const tauriDir = join(ROOT, "panel-ui", "src-tauri");

const cargoToml = readFileSync(join(tauriDir, "Cargo.toml"), "utf-8");
const version = cargoToml.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
if (!version) {
  console.error("无法从 Cargo.toml 读取版本号");
  process.exit(1);
}

const confPath = join(tauriDir, "tauri.conf.json");
const conf = JSON.parse(readFileSync(confPath, "utf-8"));
conf.version = version;
writeFileSync(confPath, JSON.stringify(conf, null, 2) + "\n", "utf-8");
console.log(`  → tauri.conf.json 版本号已同步为 ${version}`);
