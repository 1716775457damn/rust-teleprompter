// -*- coding: utf-8 -*-
use eframe::egui;
use std::time::Instant;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Sisyphus Professional Rust Teleprompter")
            .with_inner_size([1024.0, 768.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Sisyphus Rust Teleprompter",
        options,
        Box::new(|cc| {
            setup_custom_fonts(&cc.egui_ctx);
            configure_dark_theme(&cc.egui_ctx);
            Ok(Box::new(TeleprompterApp::default()))
        }),
    )
}

// Dynamically load native Chinese system fonts on Windows and macOS to fix the square block [] rendering issue.
fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    
    let paths = [
        // Windows Microsoft YaHei
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\msyh.ttf",
        // macOS PingFang
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
        "/System/Library/Fonts/STHeiti Medium.ttc",
        "/Library/Fonts/Microsoft/Microsoft YaHei.ttf",
        // Linux fallback paths
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
    ];

    let mut font_data = None;
    for path in paths {
        if let Ok(data) = std::fs::read(path) {
            font_data = Some(data);
            break;
        }
    }

    if let Some(data) = font_data {
        fonts.font_data.insert(
            "system_chinese".to_owned(),
            egui::FontData::from_owned(data),
        );
        
        fonts.families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(0, "system_chinese".to_owned());
            
        fonts.families
            .get_mut(&egui::FontFamily::Monospace)
            .unwrap()
            .insert(0, "system_chinese".to_owned());
    }
    
    ctx.set_fonts(fonts);
}

// Configures a highly polished, modern dark theme with elegant cyan accents
fn configure_dark_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(0, 151, 167); // Cyan
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(0, 188, 212); // Light Cyan
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(33, 33, 33);
    visuals.selection.bg_fill = egui::Color32::from_rgb(0, 188, 212);
    visuals.window_rounding = 8.0.into();
    ctx.set_visuals(visuals);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppMode {
    Edit,
    Prompter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColorPreset {
    WhiteOnBlack,
    YellowOnBlack,
    GreenOnBlack,
    CyanOnBlack,
    BlackOnWhite,
}

struct TeleprompterApp {
    text: String,
    font_size: f32,
    scroll_speed: f32, // Pixels per second
    scroll_y: f32,
    is_playing: bool,
    is_mirrored: bool,
    show_guide: bool,
    guide_y_pct: f32, // Position of reading guide line (0.1 to 0.9)
    color_preset: ColorPreset,
    text_color: egui::Color32,
    bg_color: egui::Color32,
    mode: AppMode,
    last_update: Instant,
    last_action_time: Instant, // For auto-hiding prompt info
    
    // Optimizations:
    text_width_pct: f32, // Margins / width control (0.4 to 0.95 of screen)
    countdown_secs: f32, // Preparation countdown (e.g. 3.0s)
    show_edge_fade: bool, // Top and bottom gradient fades
}

impl Default for TeleprompterApp {
    fn default() -> Self {
        Self {
            text: DEFAULT_TEXT.to_string(),
            font_size: 48.0,
            scroll_speed: 60.0,
            scroll_y: 0.0,
            is_playing: false,
            is_mirrored: false,
            show_guide: true,
            guide_y_pct: 0.33,
            color_preset: ColorPreset::WhiteOnBlack,
            text_color: egui::Color32::WHITE,
            bg_color: egui::Color32::BLACK,
            mode: AppMode::Edit,
            last_update: Instant::now(),
            last_action_time: Instant::now(),
            text_width_pct: 0.8,
            countdown_secs: 0.0,
            show_edge_fade: true,
        }
    }
}

impl TeleprompterApp {
    fn apply_color_preset(&mut self) {
        match self.color_preset {
            ColorPreset::WhiteOnBlack => {
                self.text_color = egui::Color32::WHITE;
                self.bg_color = egui::Color32::BLACK;
            }
            ColorPreset::YellowOnBlack => {
                self.text_color = egui::Color32::from_rgb(255, 235, 59); // Bright material yellow
                self.bg_color = egui::Color32::BLACK;
            }
            ColorPreset::GreenOnBlack => {
                self.text_color = egui::Color32::from_rgb(76, 175, 80); // Terminal green
                self.bg_color = egui::Color32::BLACK;
            }
            ColorPreset::CyanOnBlack => {
                self.text_color = egui::Color32::from_rgb(0, 188, 212); // Cyan
                self.bg_color = egui::Color32::BLACK;
            }
            ColorPreset::BlackOnWhite => {
                self.text_color = egui::Color32::BLACK;
                self.bg_color = egui::Color32::WHITE;
            }
        }
    }

    fn record_action(&mut self) {
        self.last_action_time = Instant::now();
    }
    
    // Draw visual gradient fade-out rectangles at the top/bottom of the prompter screen
    fn draw_fade_gradient(&self, painter: &egui::Painter, rect: egui::Rect, color: egui::Color32, top_to_bottom: bool) {
        let mut mesh = egui::Mesh::default();
        let c_solid = color;
        let c_transparent = egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 0);
        
        let (c_top, c_bottom) = if top_to_bottom {
            (c_solid, c_transparent)
        } else {
            (c_transparent, c_solid)
        };
        
        mesh.vertices.push(egui::epaint::Vertex { pos: rect.left_top(), uv: egui::Pos2::ZERO, color: c_top });
        mesh.vertices.push(egui::epaint::Vertex { pos: rect.right_top(), uv: egui::Pos2::ZERO, color: c_top });
        mesh.vertices.push(egui::epaint::Vertex { pos: rect.left_bottom(), uv: egui::Pos2::ZERO, color: c_bottom });
        mesh.vertices.push(egui::epaint::Vertex { pos: rect.right_bottom(), uv: egui::Pos2::ZERO, color: c_bottom });
        
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(1, 3, 2);
        
        painter.add(egui::Shape::mesh(mesh));
    }
}

impl eframe::App for TeleprompterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_update).as_secs_f32();
        self.last_update = now;

        // Auto-scrolling in Prompter mode (with countdown pause)
        if self.mode == AppMode::Prompter {
            if self.countdown_secs > 0.0 {
                self.countdown_secs = (self.countdown_secs - dt).max(0.0);
                if self.countdown_secs == 0.0 {
                    self.is_playing = true;
                }
                ctx.request_repaint();
            } else if self.is_playing {
                self.scroll_y += self.scroll_speed * dt;
                ctx.request_repaint();
            }
        }

        // Window style
        let frame_style = if self.mode == AppMode::Prompter {
            egui::Frame::none().fill(self.bg_color)
        } else {
            egui::Frame::none().fill(ctx.style().visuals.window_fill())
        };
        
        egui::CentralPanel::default().frame(frame_style).show(ctx, |ui| {
            match self.mode {
                AppMode::Edit => self.show_edit_ui(ui),
                AppMode::Prompter => self.show_prompter_ui(ui),
            }
        });
    }
}

impl TeleprompterApp {
    fn show_edit_ui(&mut self, ui: &mut egui::Ui) {
        // Reset background back to a comfortable UI gray for editing
        ui.style_mut().visuals.override_text_color = None;

        ui.vertical(|ui| {
            // Header bar
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.heading("🚀 Sisyphus Professional Rust Teleprompter");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⚡ Start Prompter (Spacebar)").clicked() {
                        self.mode = AppMode::Prompter;
                        self.scroll_y = 0.0;
                        self.countdown_secs = 3.0; // 3 seconds count down
                        self.is_playing = false;
                        self.record_action();
                        self.last_update = Instant::now();
                    }
                });
            });
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Two-column layout: Left (Controls), Right (Text input)
            ui.columns(2, |columns| {
                // Column 0: Controls
                columns[0].vertical(|ui| {
                    ui.group(|ui| {
                        ui.heading("🎛️ Settings");
                        ui.add_space(10.0);

                        ui.horizontal(|ui| {
                            ui.label("Font Size:");
                            ui.add(egui::Slider::new(&mut self.font_size, 16.0..=120.0).suffix(" px"));
                        });
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            ui.label("Scroll Speed:");
                            ui.add(egui::Slider::new(&mut self.scroll_speed, 10.0..=500.0).suffix(" px/s"));
                        });
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            ui.label("Text Column Width:");
                            ui.add(egui::Slider::new(&mut self.text_width_pct, 0.4..=0.95).text("Width %"));
                        });
                        ui.add_space(6.0);

                        ui.checkbox(&mut self.is_mirrored, "🪞 Mirror Text (Horizontal Flip for Glass)");
                        ui.checkbox(&mut self.show_guide, "🎯 Show Reading Guide Line");
                        
                        if self.show_guide {
                            ui.horizontal(|ui| {
                                ui.label("Guide Position:");
                                ui.add(egui::Slider::new(&mut self.guide_y_pct, 0.1..=0.9).text("Height %"));
                            });
                        }
                        ui.add_space(6.0);

                        ui.checkbox(&mut self.show_edge_fade, "🎬 Enable Cinema Edge Fade-Out");
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            ui.label("Color Preset:");
                            let prev_preset = self.color_preset;
                            egui::ComboBox::from_id_source("color_preset_combo")
                                .selected_text(match self.color_preset {
                                    ColorPreset::WhiteOnBlack => "White text on Black",
                                    ColorPreset::YellowOnBlack => "Yellow text on Black",
                                    ColorPreset::GreenOnBlack => "Green text on Black",
                                    ColorPreset::CyanOnBlack => "Cyan text on Black",
                                    ColorPreset::BlackOnWhite => "Black text on White",
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.color_preset, ColorPreset::WhiteOnBlack, "White text on Black");
                                    ui.selectable_value(&mut self.color_preset, ColorPreset::YellowOnBlack, "Yellow text on Black");
                                    ui.selectable_value(&mut self.color_preset, ColorPreset::GreenOnBlack, "Green text on Black");
                                    ui.selectable_value(&mut self.color_preset, ColorPreset::CyanOnBlack, "Cyan text on Black");
                                    ui.selectable_value(&mut self.color_preset, ColorPreset::BlackOnWhite, "Black text on White");
                                });
                            if self.color_preset != prev_preset {
                                self.apply_color_preset();
                            }
                        });
                    });

                    ui.add_space(15.0);
                    ui.group(|ui| {
                        ui.heading("⌨️ Shortcut Keys (Prompter Mode)");
                        ui.add_space(6.0);
                        ui.label("• Spacebar: Play / Pause scrolling");
                        ui.label("• Esc: Exit to Edit Mode");
                        ui.label("• Up / Down Arrow: Speed up / slow down (+/- 5)");
                        ui.label("• Left / Right Arrow: Scroll backward / forward");
                        ui.label("• Mouse Wheel: Scroll manually (paused) / adjust speed (playing)");
                        ui.label("• R Key: Reset scroll to top");
                        ui.label("• M Key: Toggle Mirroring");
                        ui.label("• G Key: Toggle Guide line");
                        ui.label("• Enter: Skip Countdown immediately");
                    });
                });

                // Column 1: Script Editor
                columns[1].vertical(|ui| {
                    ui.label("📝 Enter Presentation Script:");
                    ui.add_space(5.0);
                    let text_edit = egui::TextEdit::multiline(&mut self.text)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(ui.available_width())
                        .desired_rows(24);
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.add(text_edit);
                    });
                });
            });
        });
    }

    fn show_prompter_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx();
        let rect = ui.max_rect();
        let width = rect.width();
        let height = rect.height();
        
        // Listen to global inputs
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            if self.countdown_secs > 0.0 {
                self.countdown_secs = 0.0;
                self.is_playing = true;
            } else {
                self.is_playing = !self.is_playing;
            }
            self.record_action();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            if self.countdown_secs > 0.0 {
                self.countdown_secs = 0.0;
                self.is_playing = true;
                self.record_action();
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.mode = AppMode::Edit;
            self.is_playing = false;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            self.scroll_speed = (self.scroll_speed + 5.0).min(500.0);
            self.record_action();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            self.scroll_speed = (self.scroll_speed - 5.0).max(5.0);
            self.record_action();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::PageUp)) {
            self.scroll_y = (self.scroll_y - 150.0).max(0.0);
            self.record_action();
            ctx.request_repaint();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::PageDown)) {
            self.scroll_y += 150.0;
            self.record_action();
            ctx.request_repaint();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::R)) {
            self.scroll_y = 0.0;
            self.is_playing = false;
            self.countdown_secs = 0.0;
            self.record_action();
            ctx.request_repaint();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::M)) {
            self.is_mirrored = !self.is_mirrored;
            self.record_action();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::G)) {
            self.show_guide = !self.show_guide;
            self.record_action();
        }

        // Mouse Wheel interaction
        let scroll_delta = ctx.input(|i| i.smooth_scroll_delta);
        if scroll_delta.y != 0.0 {
            if self.is_playing {
                // Adjust speed when playing
                self.scroll_speed = (self.scroll_speed + scroll_delta.y * 0.5).clamp(5.0, 500.0);
            } else if self.countdown_secs <= 0.0 {
                // Scroll text manually when paused
                self.scroll_y = (self.scroll_y - scroll_delta.y * 1.5).max(0.0);
            }
            self.record_action();
            ctx.request_repaint();
        }

        // 1. Text wrapping & layout
        let font_id = egui::FontId::new(self.font_size, egui::FontFamily::Proportional);
        // Calculate dynamic padding based on text width slider
        let text_area_width = width * self.text_width_pct;
        let padding = (width - text_area_width) / 2.0;
        let wrapping_width = text_area_width;
        
        let galley = ui.fonts(|f| f.layout(self.text.clone(), font_id, self.text_color, wrapping_width));
        let galley_height = galley.rect.height();

        // Reading line Y position
        let guide_y = height * self.guide_y_pct;
        let start_y = guide_y;

        // Position Y calculation
        let draw_y = start_y - self.scroll_y;

        // Keep scroll within logical bounds
        let max_scroll = galley_height + 200.0;
        if self.scroll_y > max_scroll {
            self.scroll_y = max_scroll;
            self.is_playing = false;
        }

        // Draw the text
        let text_pos = egui::Pos2::new(rect.min.x + padding, rect.min.y + draw_y);
        let shape = egui::Shape::galley(text_pos, galley, self.text_color);

        if self.is_mirrored {
            // Apply horizontal vertex-level mirror across the window's middle X
            let center_x = rect.center().x;
            let clipped_shape = egui::epaint::ClippedShape {
                clip_rect: ui.clip_rect(),
                shape,
            };
            let primitives = ctx.tessellate(vec![clipped_shape], ctx.pixels_per_point());
            for primitive in primitives {
                if let egui::epaint::Primitive::Mesh(mut mesh) = primitive.primitive {
                    for vertex in &mut mesh.vertices {
                        vertex.pos.x = center_x - (vertex.pos.x - center_x);
                    }
                    ui.painter().add(egui::Shape::mesh(mesh));
                }
            }
        } else {
            ui.painter().add(shape);
        }

        // 2. Draw Reading Guide Line (semitransparent horizontal guide)
        if self.show_guide {
            let guide_color = egui::Color32::from_rgba_unmultiplied(239, 83, 80, 75); // Subtle red line
            let stroke = egui::Stroke::new(2.0, guide_color);
            ui.painter().line_segment(
                [
                    egui::Pos2::new(rect.min.x + 15.0, rect.min.y + guide_y),
                    egui::Pos2::new(rect.max.x - 15.0, rect.min.y + guide_y),
                ],
                stroke,
            );
            
            // Side arrow indicators pointing inwards
            let arrow_color = egui::Color32::from_rgb(239, 83, 80);
            
            let mut left_arrow = vec![
                egui::Pos2::new(rect.min.x + 15.0, rect.min.y + guide_y - 8.0),
                egui::Pos2::new(rect.min.x + 15.0, rect.min.y + guide_y + 8.0),
                egui::Pos2::new(rect.min.x + 30.0, rect.min.y + guide_y),
            ];
            
            let mut right_arrow = vec![
                egui::Pos2::new(rect.max.x - 15.0, rect.min.y + guide_y - 8.0),
                egui::Pos2::new(rect.max.x - 15.0, rect.min.y + guide_y + 8.0),
                egui::Pos2::new(rect.max.x - 30.0, rect.min.y + guide_y),
            ];

            if self.is_mirrored {
                // If mirrored, flip the visual guides too so they remain aligned with physical view
                let center_x = rect.center().x;
                for p in &mut left_arrow {
                    p.x = center_x - (p.x - center_x);
                }
                for p in &mut right_arrow {
                    p.x = center_x - (p.x - center_x);
                }
            }

            ui.painter().add(egui::Shape::convex_polygon(left_arrow, arrow_color, egui::Stroke::NONE));
            ui.painter().add(egui::Shape::convex_polygon(right_arrow, arrow_color, egui::Stroke::NONE));
        }

        // 3. Draw Cinema Edge Gradient Fades
        if self.show_edge_fade {
            let fade_height = 120.0;
            // Top fade rect
            let top_rect = egui::Rect::from_min_max(
                rect.left_top(),
                egui::Pos2::new(rect.max.x, rect.min.y + fade_height),
            );
            self.draw_fade_gradient(ui.painter(), top_rect, self.bg_color, true);

            // Bottom fade rect
            let bottom_rect = egui::Rect::from_min_max(
                egui::Pos2::new(rect.min.x, rect.max.y - fade_height),
                rect.right_bottom(),
            );
            self.draw_fade_gradient(ui.painter(), bottom_rect, self.bg_color, false);
        }

        // 4. Draw Countdown Overlay
        if self.countdown_secs > 0.0 {
            let overlay_bg = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180);
            let display_number = self.countdown_secs.ceil() as i32;
            
            egui::Area::new(egui::Id::new("countdown_area"))
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(overlay_bg)
                        .inner_margin(egui::Margin::same(20.0))
                        .rounding(12.0)
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.add_space(10.0);
                                ui.label(
                                    egui::RichText::new(format!("{}", display_number))
                                        .size(100.0)
                                        .strong()
                                        .color(egui::Color32::from_rgb(0, 188, 212)),
                                );
                                ui.label(
                                    egui::RichText::new("Starting soon... Press ENTER to skip")
                                        .size(16.0)
                                        .color(egui::Color32::LIGHT_GRAY),
                                );
                                ui.add_space(10.0);
                            });
                        });
                });
        }

        // 5. Control overlay (disappears after 2s of playing with no actions)
        let show_overlay = !self.is_playing || Instant::now().duration_since(self.last_action_time).as_secs_f32() < 2.0;
        if show_overlay && self.countdown_secs <= 0.0 {
            let overlay_bg = egui::Color32::from_rgba_unmultiplied(33, 33, 33, 200);
            let text_color = egui::Color32::WHITE;
            
            egui::Area::new(egui::Id::new("overlay_area"))
                .anchor(egui::Align2::RIGHT_BOTTOM, egui::Vec2::new(-15.0, -15.0))
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(overlay_bg)
                        .inner_margin(egui::Margin::same(8.0))
                        .rounding(4.0)
                        .show(ui, |ui| {
                            ui.style_mut().visuals.override_text_color = Some(text_color);
                            ui.horizontal(|ui| {
                                let state_str = if self.is_playing { "▶ PLAYING" } else { "⏸ PAUSED" };
                                ui.label(format!(
                                    "[{}] Speed: {:.0} px/s | Width: {:.0}% | Mirror: {} | ESC: Exit",
                                    state_str,
                                    self.scroll_speed,
                                    self.text_width_pct * 100.0,
                                    if self.is_mirrored { "ON" } else { "OFF" }
                                ));
                            });
                        });
                });
        }
    }
}

const DEFAULT_TEXT: &str = r#"=== PAGE 1: Opening & Hardware Product Definition ===
[中文]
各位评委老师，早上好。我是陶鑫旺。正如我作品集第一页所示，我将自己定位为机器人系统研发工程师与工业AI全栈实践者。我始终致力于打破代码与物理世界之间的边界。

我的第一个核心成果在于硬件产品定义。在李泽湘教授指导的InnoX创业营期间——您可以在图1中看到我们的路演合影——我主导了智能硬件Lumina从0到1的开发。针对年轻人因高功能焦虑引起的‘决策瘫痪’，我们创新地设计了‘配重转子动态阻尼扭矩反馈’的物理机制，将心理学机制具象化地实现在物理世界中。

[English]
Good morning, respected judges. I am Xinwang Tao. As shown on the first page of my portfolio, I position myself as a Robotics System R&D Engineer and an Industrial AI Full-Stack Practitioner. I am always driven to break the boundaries between code and the physical world.

My first core achievement lies in hardware product definition. During the InnoX Camp mentored by Prof. Zexiang Li—you can see our roadshow group photo in Fig. 1—I led the 0-to-1 development of the smart hardware, Lumina. Targeting the 'decision paralysis' caused by high-functioning anxiety among young people, we innovatively designed a physical mechanism with 'weighted rotor dynamic damping torque feedback,' reifying psychological mechanisms into the physical world.

=== PAGE 2: Commercial Updates & Achievement 2: Industrial AI ===
[中文]
请翻到第2页。在产品落地阶段，如顶部图2所示，通过持续、高强度的MVP快速迭代，我们成功打磨出了一款高度精致的产品Demo。

更令人兴奋的是，该项目目前正加速迈向深度商业化。我们团队最近迎来了两位实力强劲的新伙伴，目前正全力进军国际市场流量。我们正积极建立海外种子用户社区，并筹备在Kickstarter上发起众筹活动。这一从产品定义、Demo实现到全球化拓展的完整历程，使我荣获了‘优秀产品经理’称号。

除了敏捷硬件开发，我的第二个核心成果是前线工业级AI的全栈部署。请看中间的图3；这是我构建 of 工业AI Agent决策流架构。面对复杂、非标的SMT车间，我开发了一套高效的视频处理管线，清洗了1289个异常视频片段，作为视觉大模型（VLM）微调的基础。基于LangGraph状态机和ReAct逻辑，该闭环系统现已部署于实际生产线，展现出替代约30%重度人工视觉检测岗位的巨大潜力。

[English]
Please turn to PAGE 2. In the product implementation phase, as shown in Fig. 2 at the top, through continuous, high-intensity MVP rapid iterations, we have successfully polished a highly refined product Demo.

Even more excitingly, this project is now accelerating into deep commercialization. We recently onboarded two strong partners to our team and are now fully targeting international market traffic. We are actively building our overseas seed user community and preparing to launch a crowdfunding campaign on Kickstarter. This complete journey from product definition and Demo realization to global expansion earned me the 'Outstanding Product Manager' title.

Beyond agile hardware development, my second core achievement is the full-stack deployment of frontline industrial AI. Please look at Fig. 3 in the middle; this is the industrial AI Agent decision-making flow architecture I built. Facing the complex, non-standard SMT workshops, I developed a highly efficient video processing pipeline, cleaning 1,289 error-related video segments as a Vision Large Model (VLM) fine-tuning foundation. Based on the LangGraph state machine and ReAct logic, this closed-loop system is now deployed on actual production lines, demonstrating the immense potential to replace approximately 30% of heavy manual visual inspection roles.

=== PAGE 3: Achievement 3: Systems Engineering & Commercialization ===
[中文]
请翻到第3页。我的第三个核心成果是大型系统的商业化运营。作为科技初创企业‘云影智巡’的CTO，我全面领导了机器人技术（包括教育无人机和仿生扑翼微型飞行器）的全栈研发。

图4在左侧展示了我们多代仿生扑翼无人机硬核的迭代过程。图5在右侧梳理了支持该产品矩阵的跨学科技术栈。基于ArduPilot和PX4等底层架构，我们成功攻克了罗盘校准和复杂PID调参等工程难题。

[English]
Please turn to PAGE 3. My third core achievement is the commercial operation of large-scale systems. As the CTO of the tech startup 'Yunying Zhixun', I comprehensively led the full-stack R&D of robotics, including educational drones and bionic flapping-wing UAVs.

Fig. 4 on the left displays the hardcore iterative process of our multi-generation bionic flapping-wing UAVs. Fig. 5 on the right maps out the cross-disciplinary tech stack supporting this product matrix. Based on underlying architectures like ArduPilot and PX4, we successfully resolved engineering challenges such as compass calibration and complex PID tuning.

=== PAGE 4: Commercial Validation & IP Moat ===
[中文]
第4页展示了我们的商业验证与技术护城河。图6来自于我们在‘互联网+’大赛中荣获国家级银奖的颁奖现场。图7是我们公司在湖南股权交易所成功挂牌的证书。

为了构建技术壁垒，作为核心发明人，我累计获得了10余项发明专利与软件著作权。此外，底部的图9证明了我已获得鸿蒙（HarmonyOS）高级应用开发者认证，展现出全面的生态构建能力。

[English]
PAGE 4 showcases our commercial validation and technical moats. Fig. 6 is from the site where we won the National Silver Award in the 'Internet+' Competition. Fig. 7 is the certificate of our company's successful listing on the Hunan Equity Exchange.

To build technical barriers, as shown in Fig. 8 in the middle, I accumulated over 10 invention patents and software copyrights as the core inventor. Additionally, Fig. 9 at the bottom proves that I obtained the HarmonyOS Advanced Application Developer Certification, demonstrating a comprehensive ecosystem-building capability.

=== PAGE 5: Achievement 4: Academic Research & Theory ===
[中文]
工程痛点反向驱动了我对学术界的深耕。请看包含我第四个成果的第5页。

左下角的图12是我在JCR Q1区期刊《Biomimetics》上发表的仿生无人机空间连杆机构研究，在此项研究中，我们克服了复杂湍流下的控制难题。右下角的图13是发表在自动驾驶顶级期刊《IEEE TVT》上的预测局部多重注意力（PLMA）模型。作为第二作者，我独立设计了核心进化网络拓扑，并在真实数据集上完成了高精度的风险评估验证。

[English]
Engineering pain points have inversely driven my deep dive into academia. Please look at PAGE 5, covering my fourth achievement.

Fig. 12 on the bottom left is my research on a bionic UAV spatial linkage mechanism published in the JCR Q1 journal, Biomimetics, where we conquered control challenges under complex turbulence. Fig. 13 on the bottom right is the Predictive Local Multiple Attention (PLMA) model published in the top-tier autonomous driving journal, IEEE TVT. As the second author, I independently designed the core evolutionary network topology and completed high-precision risk assessment validation on real-world datasets.

=== PAGE 6: Full-Stack Skill Tree & Vision ===
[中文]
最后，请看第6页。顶部区域总结了我的全栈技能树。从熟练使用Cursor等AI工具构建自动化工作流，到将边缘计算底层彻底从C++重构为性能更高的Rust语言并部署在ESP32-S3微控制器上——我正不断且敏捷地迭代我的技术边界。

我坚信，顶尖的技术创新是数字计算与人类感官体验的深度共鸣。我非常渴望加入贵团队，与顶尖的跨学科头脑进行思想碰撞，共同定义机器人与自主系统（RAS）的下一个颠覆性范式！谢谢大家。

[English]
Finally, please look at PAGE 6. The top section summarizes my full-stack skill tree. From proficiently using AI tools like Cursor to build automated workflows, to completely refactoring the edge computing base from C++ to the higher-performance Rust language and deploying it on ESP32-S3 microcontrollers—I am constantly and agilely iterating my technical boundaries.

I firmly believe that top-tier technological innovation is a deep resonance between digital computing and human sensory experience. I am eager to join your team, brainstorm with top interdisciplinary minds, and co-define the next disruptive paradigm in Robotics and Autonomous Systems (RAS)! Thank you.
"#;
