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
    GWL_EXSTYLE, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, WS_EX_NOACTIVATE,
};

use crate::reader::format_quest_time;
use crate::types::{RadarData, Settings};

// ── 圆形雷达图 ──────────────────────────────────────────────

/// 在圆上取点：0°=上, 90°=右, 180°=下, 270°=左。
fn point_on_circle(center_x: f32, center_y: f32, radius: f32, angle_deg: f32) -> egui::Pos2 {
    let rad = angle_deg.to_radians();
    egui::pos2(
        center_x + radius * rad.sin(),
        center_y - radius * rad.cos(),
    )
}

/// 将水平距离映射到雷达图半径，含非线性压缩。
fn map_distance_to_radius(dist: f32, ring_900_radius: f32, ring_1500_radius: f32, max_radius: f32) -> f32 {
    let d = dist.max(0.0);
    if d <= 900.0 {
        return (d / 900.0) * ring_900_radius;
    }
    if d <= 1500.0 {
        let t = (d - 900.0) / 600.0;
        return ring_900_radius + t * (ring_1500_radius - ring_900_radius);
    }
    let overflow = d - 1500.0;
    let extra = (overflow / 1500.0).clamp(0.0, 1.0);
    let compressed = extra.sqrt();
    (ring_1500_radius + compressed * (max_radius - ring_1500_radius)).min(max_radius)
}

/// 距离强度等级：4 档。
fn distance_level(dist_h: f32) -> usize {
    if dist_h < 900.0 {
        4
    } else if dist_h < 1500.0 {
        3
    } else if dist_h < 2500.0 {
        2
    } else {
        1
    }
}

/// 距离色块配色：越近越暖。
fn distance_color(level: usize) -> egui::Color32 {
    match level {
        4 => egui::Color32::from_rgb(255, 110, 70),  // 近
        3 => egui::Color32::from_rgb(245, 190, 80),  // 中
        2 => egui::Color32::from_rgb(80, 190, 220),  // 远
        _ => egui::Color32::from_rgb(90, 130, 170),  // 极远
    }
}

/// 绘制距离强度平行四边形色块。
fn draw_distance_blocks(
    painter: &egui::Painter,
    x: f32,
    y: f32,
    dist_h: f32,
    text_opacity: f32,
) {
    let level = distance_level(dist_h);
    let alpha = (text_opacity.clamp(0.0, 1.0) * 255.0) as u8;

    let active_base = distance_color(level);
    let active = egui::Color32::from_rgba_premultiplied(
        active_base.r(),
        active_base.g(),
        active_base.b(),
        alpha,
    );

    let inactive = egui::Color32::from_rgba_premultiplied(
        80, 86, 92,
        (alpha as f32 * 0.35) as u8,
    );

    let block_w = 14.0;
    let block_h = 8.0;
    let gap = 4.0;
    let skew = 4.0;

    for i in 0..4 {
        let bx = x + i as f32 * (block_w + gap);
        let color = if i < level { active } else { inactive };

        let points = vec![
            egui::pos2(bx + skew, y),
            egui::pos2(bx + block_w + skew, y),
            egui::pos2(bx + block_w, y + block_h),
            egui::pos2(bx, y + block_h),
        ];

        painter.add(egui::Shape::convex_polygon(
            points,
            color,
            egui::Stroke::new(
                1.0,
                egui::Color32::from_rgba_premultiplied(255, 255, 255, (alpha as f32 * 0.20) as u8),
            ),
        ));
    }
}

/// 高度落差符号化。
fn vertical_hint(dist_v: f32) -> &'static str {
    if dist_v > 100.0 { "↑" } else if dist_v < -100.0 { "↓" } else { "≈" }
}

/// 绘制圆形雷达图 —— 替代原来的角度数字和距离/高度文本。
///
/// - 怪物固定在中心，玩家以点表示
/// - 水平距离通过玩家点距中心的半径表达
/// - 正前方/正后方 60° 分割射线
/// - 高度落差仅以符号弱化提示
fn draw_circular_radar(
    painter: &egui::Painter,
    x: f32,
    y: f32,
    size: f32,
    dist_h: f32,
    dist_v: f32,
    angle: f32,
    text_opacity: f32,
) {
    if !dist_h.is_finite() || !dist_v.is_finite() || !angle.is_finite() {
        return;
    }

    let alpha = (text_opacity.clamp(0.0, 1.0) * 255.0) as u8;

    let bg_color = egui::Color32::from_rgba_premultiplied(20, 24, 28, (alpha as f32 * 0.30) as u8);
    let line_color = egui::Color32::from_rgba_premultiplied(160, 180, 190, (alpha as f32 * 0.65) as u8);
    let sector_color = egui::Color32::from_rgba_premultiplied(120, 140, 155, (alpha as f32 * 0.45) as u8);
    let monster_color = egui::Color32::from_rgba_premultiplied(255, 180, 80, alpha);
    let player_color = egui::Color32::from_rgba_premultiplied(110, 220, 255, alpha);
    let text_color = egui::Color32::from_rgba_premultiplied(220, 220, 220, alpha);

    let center_x = x + size * 0.5;
    let center_y = y + size * 0.5;
    let padding = 6.0;
    let max_radius = size * 0.5 - padding;

    let ring_900_radius = max_radius * 0.50;
    let ring_1500_radius = max_radius * 0.85;
    let center = egui::pos2(center_x, center_y);

    // 背景圆
    painter.circle_filled(center, max_radius + 3.0, bg_color);

    // 距离圈：900 内圈 + 1500 外圈（不画边界环）
    painter.circle_stroke(center, ring_900_radius, egui::Stroke::new(1.0, line_color));
    painter.circle_stroke(center, ring_1500_radius, egui::Stroke::new(1.0, line_color));

    // 正前方 / 正后方 60° 边界射线
    for angle_deg in [30.0, 330.0, 150.0, 210.0] {
        let end = point_on_circle(center_x, center_y, max_radius, angle_deg);
        painter.line_segment([center, end], egui::Stroke::new(1.0, sector_color));
    }

    // 中心怪物点
    painter.circle_filled(center, 4.0, monster_color);

    // 玩家点
    let radius = map_distance_to_radius(dist_h, ring_900_radius, ring_1500_radius, max_radius);
    let player_pos = point_on_circle(center_x, center_y, radius, angle);

    painter.circle_filled(player_pos, 4.0, player_color);
    painter.circle_stroke(
        player_pos,
        4.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(255, 255, 255, (alpha as f32 * 0.35) as u8)),
    );

    // 前方标签
    painter.text(
        egui::pos2(center_x, y - 2.0),
        egui::Align2::CENTER_BOTTOM,
        "前",
        egui::FontId::proportional(11.0),
        text_color,
    );

    // 距离圈标签（写在环线上）
    painter.text(
        egui::pos2(center_x + ring_900_radius, center_y),
        egui::Align2::CENTER_CENTER,
        "900",
        egui::FontId::proportional(10.0),
        text_color,
    );
    painter.text(
        egui::pos2(center_x + ring_1500_radius, center_y),
        egui::Align2::CENTER_CENTER,
        "1500",
        egui::FontId::proportional(10.0),
        text_color,
    );

    // 雷达下方：距离强度色块 + 距离等级文字 + 高度符号
    let hint = vertical_hint(dist_v);
    let bottom_y = y + size + 8.0;

    let dist_lbl = match distance_level(dist_h) {
        4 => "极近",
        3 => "中等",
        2 => "很远",
        _ => "极远",
    };

    // 全部左顶格排布：距离  ▰▰▰▰  极远        高度 ↑
    let mut cx = x + 6.0;

    painter.text(
        egui::pos2(cx, bottom_y),
        egui::Align2::LEFT_TOP,
        "距离",
        egui::FontId::proportional(12.0),
        text_color,
    );
    cx += 28.0;

    draw_distance_blocks(painter, cx, bottom_y + 5.0, dist_h, text_opacity);
    cx += 68.0 + 10.0; // 4 块总宽 + 间距

    painter.text(
        egui::pos2(cx, bottom_y),
        egui::Align2::LEFT_TOP,
        dist_lbl,
        egui::FontId::proportional(12.0),
        text_color,
    );
    cx += 42.0;

    let height_color = egui::Color32::from_rgba_premultiplied(130, 210, 230, alpha);
    painter.text(
        egui::pos2(cx, bottom_y),
        egui::Align2::LEFT_TOP,
        format!("高度 {}", hint),
        egui::FontId::proportional(12.0),
        height_color,
    );
}

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
                    (current | WS_EX_NOACTIVATE) as isize,
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
                let win_w = 240.0;

                let rect_height = if !data.connected {
                    82.0
                } else if data.has_monster {
                    calc_overlay_height(&settings, data.monster_id)
                } else {
                    82.0
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
                        let suffix = if data.monsters.len() > 1 { "(改锁定切换)" } else { "" };

                        painter.text(
                            egui::pos2(10.0, y),
                            egui::Align2::LEFT_TOP,
                            format!("目标: {}{}", name, suffix),
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
                            format!("当前招式: {}", name),
                            egui::FontId::proportional(16.0),
                            tc(egui::Color32::from_rgb(144, 238, 144)),
                        );

                        y += 22.0;
                    }

                    // 圆形雷达图（放在最底部，快捷键提示上方）
                    if settings.show_radar {
                        y += 20.0; // 上方间距

                        draw_circular_radar(
                            painter,
                            10.0,
                            y,
                            160.0,
                            data.dist_h,
                            data.dist_v,
                            data.angle,
                            text_alpha,
                        );

                        y += 192.0; // 雷达占用 180 + 下方间距 12
                    }

                } else {
                    ui.painter().text(
                        egui::pos2(10.0, 14.0),
                        egui::Align2::LEFT_CENTER,
                        "未检测到怪物",
                        egui::FontId::proportional(16.0),
                        tc(egui::Color32::GRAY),
                    );
                }

                // 快捷键提示（始终显示，右下角）
                ui.painter().text(
                    egui::pos2(win_w - 10.0, rect_height - 22.0),
                    egui::Align2::RIGHT_TOP,
                    "隐藏/显示: Ctrl+Shift+U",
                    egui::FontId::proportional(13.0),
                    tc(egui::Color32::LIGHT_GRAY),
                );


            });

        // 自动调整高度
        let new_height = if !data.connected {
            82.0
        } else if data.has_monster {
            calc_overlay_height(&settings, data.monster_id)
        } else {
            82.0
        };

        if new_height != self.last_inner_height {
            self.last_inner_height = new_height;

            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                240.0,
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

    if settings.show_radar {
        h += 212.0; // 20px 上方间距 + 160px 雷达 + 20px 提示文字 + 12px 下方间距
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