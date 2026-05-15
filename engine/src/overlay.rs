//! 悬浮窗 UI 状态 + 渲染
//!
//! RadarApp 管理悬浮窗的 UI 状态（显示/隐藏、窗口样式、拖拽），
//! 实现 eframe::App 完成每帧渲染。
//! 数据本身由后台 reader 线程提供，通过共享 RadarData 获取。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use winapi::shared::windef::RECT;
use winapi::um::winuser::{
    FindWindowW, GetWindowLongPtrW, GetWindowRect, SetWindowLongPtrW, SetWindowPos,
    GWL_EXSTYLE, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, WS_EX_NOACTIVATE, WS_EX_TRANSPARENT,
};

use crate::reader::format_quest_time;
use crate::types::{RadarData, Settings};

/// 游戏状态管理器（仅 UI 状态，数据由后台线程提供）
pub struct RadarApp {
    pub shared_data: Arc<Mutex<RadarData>>,
    pub settings: Arc<Mutex<Settings>>,
    pub hotkey_signal: Arc<AtomicBool>,
    pub hidden: bool,
    pub saved_window_pos: Option<(i32, i32)>,
    pub last_inner_height: f32,
    pub window_styles_set: bool,
    pub mouse_passthrough_on: bool,
}

impl RadarApp {
    /// 创建带有全局热键信号和共享数据的 RadarApp 实例
    pub fn with_hotkey_signal(
        signal: Arc<AtomicBool>,
        shared_data: Arc<Mutex<RadarData>>,
        settings: Arc<Mutex<Settings>>,
    ) -> Self {
        Self {
            hotkey_signal: signal,
            shared_data,
            settings,
            hidden: false,
            saved_window_pos: None,
            last_inner_height: 0.0,
            window_styles_set: false,
            mouse_passthrough_on: false,
        }
    }

    /// 从共享缓存读取最新雷达数据（不阻塞，即取即用）
    pub fn refresh(&self) -> RadarData {
        self.shared_data
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default()
    }
}

impl eframe::App for RadarApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Color32::TRANSPARENT.to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(std::time::Duration::from_secs_f32(0.1));

        // 首次运行 / 恢复时设置窗口样式：禁止抢焦点 + 鼠标穿透
        unsafe {
            let title: Vec<u16> = "MHW Radar\0".encode_utf16().collect();
            let hwnd = FindWindowW(std::ptr::null_mut(), title.as_ptr());

            if !hwnd.is_null() && !self.window_styles_set {
                let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;

                SetWindowLongPtrW(
                    hwnd,
                    GWL_EXSTYLE,
                    (current | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT) as isize,
                );

                self.window_styles_set = true;
                self.mouse_passthrough_on = true;
            }
        }

        // 检测全局热键 Ctrl+Shift+U
        if self.hotkey_signal.swap(false, Ordering::SeqCst) {
            self.hidden = !self.hidden;

            unsafe {
                let title: Vec<u16> = "MHW Radar\0".encode_utf16().collect();
                let hwnd = FindWindowW(std::ptr::null_mut(), title.as_ptr());

                if !hwnd.is_null() {
                    if self.hidden {
                        let mut rect = std::mem::zeroed::<RECT>();
                        GetWindowRect(hwnd, &mut rect);

                        self.saved_window_pos = Some((rect.left, rect.top));

                        SetWindowPos(
                            hwnd,
                            std::ptr::null_mut(),
                            -32000,
                            -32000,
                            0,
                            0,
                            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                        );
                    } else {
                        let (x, y) = self.saved_window_pos.unwrap_or((100, 100));

                        SetWindowPos(
                            hwnd,
                            std::ptr::null_mut(),
                            x,
                            y,
                            0,
                            0,
                            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                        );
                    }
                }
            }
        }

        // 读取最新数据
        let data = self.refresh();
        if self.hidden {
            return;
        }

        let settings = self.settings.lock().unwrap().clone();

        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                let win_w = 280.0;

                let rect_height = if !data.connected {
                    60.0
                } else if data.has_monster {
                    calc_overlay_height(&settings, data.monster_id)
                } else {
                    60.0
                };

                let alpha = (settings.window_opacity * 255.0) as u8;
                let text_alpha = settings.text_opacity;

                let tc = |c: egui::Color32| -> egui::Color32 {
                    let a = (255.0 * text_alpha) as u8;

                    egui::Color32::from_rgba_premultiplied(
                        (c.r() as u16 * a as u16 / 255) as u8,
                        (c.g() as u16 * a as u16 / 255) as u8,
                        (c.b() as u16 * a as u16 / 255) as u8,
                        a,
                    )
                };

                let overlay_rect = egui::Rect::from_min_size(
                    egui::pos2(0.0, 0.0),
                    egui::vec2(win_w, rect_height),
                );

                // 背景
                ui.painter()
                    .rect_filled(overlay_rect, 6.0, egui::Color32::from_black_alpha(alpha));

                // 拖拽
                let drag_resp = ui.allocate_rect(overlay_rect, egui::Sense::drag());

                if drag_resp.dragged() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }

                if !data.connected {
                    ui.painter().text(
                        egui::pos2(10.0, 14.0),
                        egui::Align2::LEFT_CENTER,
                        "未检测到游戏进程",
                        egui::FontId::proportional(16.0),
                        tc(egui::Color32::GRAY),
                    );
                } else if data.has_monster {
                    let painter = ui.painter();
                    let mut y = 4.0;

                    if settings.show_time {
                        if let Some(ms) = data.quest_elapsed_ms {
                            painter.text(
                                egui::pos2(10.0, y),
                                egui::Align2::LEFT_TOP,
                                format!("时间: {}", format_quest_time(ms)),
                                egui::FontId::proportional(16.0),
                                tc(egui::Color32::WHITE),
                            );

                            y += 22.0;
                        }
                    }

                    if settings.show_monster_name {
                        let name = data.monster_name.unwrap_or("未知怪物");

                        painter.text(
                            egui::pos2(10.0, y),
                            egui::Align2::LEFT_TOP,
                            format!("目标: {}", name),
                            egui::FontId::proportional(16.0),
                            tc(egui::Color32::WHITE),
                        );

                        y += 22.0;
                    }

                    if settings.show_hp {
                        if let Some(hp) = data.monster_hp {
                            let pct = hp.current / hp.max * 100.0;

                            painter.text(
                                egui::pos2(10.0, y),
                                egui::Align2::LEFT_TOP,
                                format!("血量: {:.0} ({:.1}%)", hp.current, pct),
                                egui::FontId::proportional(16.0),
                                tc(egui::Color32::GREEN),
                            );

                            y += 22.0;
                        }
                    }

                    if settings.show_counterattack && data.monster_id == 101 {
                        if let Some(val) = data.counterattack_value {
                            let label = if data.counterattack_scaled {
                                format!("下压值(换算): {:.0}", val)
                            } else {
                                format!("下压值: {:.0}", val)
                            };

                            painter.text(
                                egui::pos2(10.0, y),
                                egui::Align2::LEFT_TOP,
                                label,
                                egui::FontId::proportional(16.0),
                                tc(egui::Color32::YELLOW),
                            );

                            y += 22.0;
                        }
                    }

                    if settings.show_dist_h {
                        painter.text(
                            egui::pos2(10.0, y),
                            egui::Align2::LEFT_TOP,
                            format!("水平距离: {:.0}", data.dist_h),
                            egui::FontId::proportional(16.0),
                            tc(egui::Color32::from_rgb(255, 165, 0)),
                        );

                        y += 22.0;
                    }

                    if settings.show_dist_v {
                        painter.text(
                            egui::pos2(10.0, y),
                            egui::Align2::LEFT_TOP,
                            format!("高度落差: {:.0}", data.dist_v),
                            egui::FontId::proportional(16.0),
                            tc(egui::Color32::from_rgb(255, 165, 0)),
                        );

                        y += 22.0;
                    }

                    if settings.show_angle {
                        painter.text(
                            egui::pos2(10.0, y),
                            egui::Align2::LEFT_TOP,
                            format!("角度: {:.1}°", data.angle),
                            egui::FontId::proportional(16.0),
                            tc(egui::Color32::from_rgb(255, 165, 0)),
                        );

                        y += 22.0;
                    }

                    if settings.show_action_id {
                        painter.text(
                            egui::pos2(10.0, y),
                            egui::Align2::LEFT_TOP,
                            format!("怪物动作ID：{}", data.action_id),
                            egui::FontId::proportional(16.0),
                            tc(egui::Color32::from_rgb(180, 180, 255)),
                        );

                        y += 22.0;
                    }

                    if settings.show_action_name {
                        let name = data
                            .action_name
                            .or(data.action_name_en.as_deref())
                            .unwrap_or("未知");

                        painter.text(
                            egui::pos2(10.0, y),
                            egui::Align2::LEFT_TOP,
                            format!("实际招式: {}", name),
                            egui::FontId::proportional(16.0),
                            tc(egui::Color32::from_rgb(144, 238, 144)),
                        );

                        y += 22.0;
                    }

                    painter.text(
                        egui::pos2(10.0, y),
                        egui::Align2::LEFT_TOP,
                        "隐藏/显示: Ctrl+Shift+U",
                        egui::FontId::proportional(13.0),
                        tc(egui::Color32::LIGHT_GRAY),
                    );
                } else {
                    ui.painter().text(
                        egui::pos2(10.0, 14.0),
                        egui::Align2::LEFT_CENTER,
                        "未检测到怪物",
                        egui::FontId::proportional(16.0),
                        tc(egui::Color32::GRAY),
                    );
                }

                if data.flashing {
                    ui.painter()
                        .rect_filled(overlay_rect, 6.0, egui::Color32::from_white_alpha(70));
                }
            });

        // 自动调整高度
        let new_height = if !data.connected {
            60.0
        } else if data.has_monster {
            calc_overlay_height(&settings, data.monster_id)
        } else {
            60.0
        };

        if new_height != self.last_inner_height {
            self.last_inner_height = new_height;

            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                280.0,
                new_height,
            )));
        }
    }
}

/// 计算悬浮窗高度（根据启用的显示项和怪物类型）
fn calc_overlay_height(settings: &Settings, monster_id: i32) -> f32 {
    let mut h = 4.0;

    if settings.show_time {
        h += 22.0;
    }

    if settings.show_monster_name {
        h += 22.0;
    }

    if settings.show_hp {
        h += 22.0;
    }

    if settings.show_counterattack && monster_id == 101 {
        h += 22.0;
    }

    if settings.show_dist_h {
        h += 22.0;
    }

    if settings.show_dist_v {
        h += 22.0;
    }

    if settings.show_angle {
        h += 22.0;
    }

    if settings.show_action_id {
        h += 22.0;
    }

    if settings.show_action_name {
        h += 22.0;
    }

    h += 22.0; // 快捷键提示

    h
}