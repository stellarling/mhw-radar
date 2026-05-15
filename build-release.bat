@echo off
cd /d "%~dp0engine"
cargo build --release %*
if exist target\release\mhw_radar.pdb (
    move /Y target\release\mhw_radar.pdb target\release\mhw-radar.pdb >nul
)
