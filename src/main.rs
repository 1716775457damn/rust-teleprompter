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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LanguageFilter {
    All,
    ChineseOnly,
    EnglishOnly,
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
    text_width_pct: f32, // Margins / width control (0.4 to 0.95 of screen)
    countdown_secs: f32, // Preparation countdown (e.g. 3.0s)
    show_edge_fade: bool, // Top and bottom gradient fades
    line_spacing: f32, // Line height multiplier (1.0 to 2.5)
    sections: Vec<(String, f32)>, // Store header names and their scroll Y offsets
    max_scroll: f32, // Store dynamically calculated max scroll limit
    language_filter: LanguageFilter,
    enable_focus_mode: bool, // Dim unread text blocks to enhance focus
    
    // Presenter HUD Upgrades:
    elapsed_secs: f32, // Stopwatch elapsed time
    show_hud: bool, // Toggle Presenter Guide HUD (default true)
}

fn load_initial_text() -> String {
    if let Ok(content) = std::fs::read_to_string("script.txt") {
        if !content.trim().is_empty() {
            return content;
        }
    }
    if let Ok(content) = std::fs::read_to_string("F:\\rust-teleprompter\\script.txt") {
        if !content.trim().is_empty() {
            return content;
        }
    }
    DEFAULT_TEXT.to_string()
}

fn save_text(text: &str) {
    let _ = std::fs::write("script.txt", text);
    let _ = std::fs::write("F:\\rust-teleprompter\\script.txt", text);
}

impl Default for TeleprompterApp {
    fn default() -> Self {
        Self {
            text: load_initial_text(),
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
            line_spacing: 1.4,
            sections: Vec::new(),
            max_scroll: 1000.0,
            language_filter: LanguageFilter::All,
            enable_focus_mode: true,
            elapsed_secs: 0.0,
            show_hud: true,
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

    fn jump_to_section(&mut self, idx: usize) {
        if idx < self.sections.len() {
            self.scroll_y = self.sections[idx].1;
            self.record_action();
        }
    }

    // Tessellates and draws a shape, mirroring it horizontally if `is_mirrored` is enabled
    fn paint_shape(&self, ctx: &egui::Context, painter: &egui::Painter, clip_rect: egui::Rect, shape: egui::Shape, center_x: f32) {
        if self.is_mirrored {
            let clipped_shape = egui::epaint::ClippedShape {
                clip_rect,
                shape,
            };
            let primitives = ctx.tessellate(vec![clipped_shape], ctx.pixels_per_point());
            for primitive in primitives {
                if let egui::epaint::Primitive::Mesh(mut mesh) = primitive.primitive {
                    for vertex in &mut mesh.vertices {
                        vertex.pos.x = center_x - (vertex.pos.x - center_x);
                    }
                    painter.add(egui::Shape::mesh(mesh));
                }
            }
        } else {
            painter.add(shape);
        }
    }
}

// Custom Markdown-style bold segment parser (`**text**` -> highlighting)
fn parse_formatted_line(
    text: &str,
    font_size: f32,
    base_color: egui::Color32,
    accent_color: egui::Color32,
    wrapping_width: f32,
    is_header: bool,
    is_meta: bool,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = wrapping_width;
    
    let final_font_size = if is_header {
        font_size * 1.1
    } else if is_meta {
        font_size * 0.8
    } else {
        font_size
    };

    let final_base_color = if is_header {
        egui::Color32::from_rgb(0, 188, 212) // Cyan for headers
    } else if is_meta {
        egui::Color32::from_rgb(120, 144, 156) // Muted blue-grey for metadata/tags
    } else {
        base_color
    };

    let parts: Vec<&str> = text.split("**").collect();
    let mut is_bold = false;
    for part in parts {
        let color = if is_bold { accent_color } else { final_base_color };
        
        let text_format = egui::TextFormat {
            font_id: egui::FontId::new(final_font_size, egui::FontFamily::Proportional),
            color,
            background: egui::Color32::TRANSPARENT,
            italics: false,
            underline: if is_bold { egui::Stroke::new(2.0, accent_color) } else { egui::Stroke::NONE },
            strikethrough: egui::Stroke::NONE,
            valign: egui::Align::BOTTOM,
            ..Default::default()
        };
        
        job.append(part, 0.0, text_format);
        is_bold = !is_bold;
    }
    job
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
                self.scroll_y = (self.scroll_y + self.scroll_speed * dt).min(self.max_scroll);
                self.elapsed_secs += dt;
                if self.scroll_y >= self.max_scroll {
                    self.is_playing = false;
                }
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
                        self.elapsed_secs = 0.0;
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
                            ui.label("Line Height / Spacing:");
                            ui.add(egui::Slider::new(&mut self.line_spacing, 1.0..=2.5).suffix(" x"));
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
                        ui.checkbox(&mut self.enable_focus_mode, "👁️ Enable Active Line Focus Mode");
                        ui.checkbox(&mut self.show_hud, "📊 Enable Presenter Side HUD Panels");
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            ui.label("Language Block Filter:");
                            egui::ComboBox::from_id_source("lang_filter_combo")
                                .selected_text(match self.language_filter {
                                    LanguageFilter::All => "Show All (CN & EN)",
                                    LanguageFilter::ChineseOnly => "Chinese Only (中文)",
                                    LanguageFilter::EnglishOnly => "English Only",
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.language_filter, LanguageFilter::All, "Show All (CN & EN)");
                                    ui.selectable_value(&mut self.language_filter, LanguageFilter::ChineseOnly, "Chinese Only (中文)");
                                    ui.selectable_value(&mut self.language_filter, LanguageFilter::EnglishOnly, "English Only");
                                });
                        });
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
                        ui.label("• Keys 1-9: Jump directly to mapped Page Sections!");
                        ui.label("• L Key: Toggle Language Filters (All / CN / EN)");
                        ui.label("• H Key: Toggle Presenter Guide HUD");
                        ui.label("• Minus (-) / Equals (=): Move reading guide line up/down");
                        ui.label("• R Key: Reset scroll to top & timer");
                        ui.label("• M Key: Toggle Mirroring");
                        ui.label("• G Key: Toggle Guide line");
                        ui.label("• Enter: Skip Countdown immediately");
                    });
                });

                // Column 1: Script Editor (Auto-saves changes)
                columns[1].vertical(|ui| {
                    ui.label("📝 Enter Presentation Script (Autosaved):");
                    ui.add_space(5.0);
                    let previous_text = self.text.clone();
                    let text_edit = egui::TextEdit::multiline(&mut self.text)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(ui.available_width())
                        .desired_rows(24);
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.add(text_edit);
                    });
                    if self.text != previous_text {
                        save_text(&self.text);
                    }
                });
            });
        });
    }

    fn show_prompter_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx();
        let rect = ui.max_rect();
        let width = rect.width();
        let height = rect.height();
        let center_x = rect.center().x;
        
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
            self.scroll_y = (self.scroll_y + 150.0).min(self.max_scroll);
            self.record_action();
            ctx.request_repaint();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::R)) {
            self.scroll_y = 0.0;
            self.is_playing = false;
            self.countdown_secs = 0.0;
            self.elapsed_secs = 0.0;
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
        if ctx.input(|i| i.key_pressed(egui::Key::H)) {
            self.show_hud = !self.show_hud;
            self.record_action();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::L)) {
            self.language_filter = match self.language_filter {
                LanguageFilter::All => LanguageFilter::ChineseOnly,
                LanguageFilter::ChineseOnly => LanguageFilter::EnglishOnly,
                LanguageFilter::EnglishOnly => LanguageFilter::All,
            };
            self.record_action();
            ctx.request_repaint();
        }

        // Reading Guide Line Y Adjustments via keyboard shortcuts (-) and (=)
        if ctx.input(|i| i.key_pressed(egui::Key::Minus)) {
            self.guide_y_pct = (self.guide_y_pct - 0.02).max(0.1);
            self.record_action();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Equals)) {
            self.guide_y_pct = (self.guide_y_pct + 0.02).min(0.9);
            self.record_action();
        }

        // Quick Jumps (Keys 1-9)
        if ctx.input(|i| i.key_pressed(egui::Key::Num1)) { self.jump_to_section(0); }
        if ctx.input(|i| i.key_pressed(egui::Key::Num2)) { self.jump_to_section(1); }
        if ctx.input(|i| i.key_pressed(egui::Key::Num3)) { self.jump_to_section(2); }
        if ctx.input(|i| i.key_pressed(egui::Key::Num4)) { self.jump_to_section(3); }
        if ctx.input(|i| i.key_pressed(egui::Key::Num5)) { self.jump_to_section(4); }
        if ctx.input(|i| i.key_pressed(egui::Key::Num6)) { self.jump_to_section(5); }
        if ctx.input(|i| i.key_pressed(egui::Key::Num7)) { self.jump_to_section(6); }
        if ctx.input(|i| i.key_pressed(egui::Key::Num8)) { self.jump_to_section(7); }
        if ctx.input(|i| i.key_pressed(egui::Key::Num9)) { self.jump_to_section(8); }

        // Mouse Wheel interaction
        let scroll_delta = ctx.input(|i| i.smooth_scroll_delta);
        if scroll_delta.y != 0.0 {
            if self.is_playing {
                // Adjust speed when playing
                self.scroll_speed = (self.scroll_speed + scroll_delta.y * 0.5).clamp(5.0, 500.0);
            } else if self.countdown_secs <= 0.0 {
                // Scroll text manually when paused
                self.scroll_y = (self.scroll_y - scroll_delta.y * 1.5).clamp(0.0, self.max_scroll);
            }
            self.record_action();
            ctx.request_repaint();
        }

        // Calculate layout properties
        let text_area_width = width * self.text_width_pct;
        let padding = (width - text_area_width) / 2.0;
        let wrapping_width = text_area_width;

        // Reading line Y position
        let guide_y = height * self.guide_y_pct;
        let start_y = guide_y;

        // Paragraph-based custom layout loop with syntax highlighting and jump section calculations
        let mut current_y = 0.0;
        let mut temp_sections = Vec::new();
        
        // Tracks stateful language blocks in parsing
        let mut current_block_lang = LanguageFilter::All;
        let bold_highlight_color = egui::Color32::from_rgb(255, 179, 0); // Amber/orange for bold text emphasis

        for line in self.text.lines() {
            let trimmed = line.trim();
            
            // Detect block language tags
            if trimmed.starts_with("[中文]") || trimmed.starts_with("【中文】") {
                current_block_lang = LanguageFilter::ChineseOnly;
            } else if trimmed.starts_with("[English]") || trimmed.starts_with("[英文]") {
                current_block_lang = LanguageFilter::EnglishOnly;
            } else if trimmed.starts_with("===") {
                current_block_lang = LanguageFilter::All; // Headings are always displayed
            }
            
            // Apply language filter
            if self.language_filter != LanguageFilter::All && current_block_lang != LanguageFilter::All {
                if current_block_lang != self.language_filter {
                    continue; // Skip rendering & layout calculation for filtered language
                }
            }

            let is_header = trimmed.starts_with("===");
            let is_meta = trimmed.starts_with("[") || trimmed.starts_with("【");
            
            let extra_space = if is_header {
                let clean_name = trimmed.replace("===", "").trim().to_string();
                temp_sections.push((clean_name, current_y));
                self.font_size * 0.8
            } else if is_meta {
                self.font_size * 0.2
            } else {
                0.0
            };

            // Layout the line/paragraph using the correct format and bold parser
            let mut job = parse_formatted_line(
                trimmed,
                self.font_size,
                self.text_color,
                bold_highlight_color,
                wrapping_width,
                is_header,
                is_meta,
            );
            
            let galley = ui.fonts(|f| f.layout_job(job.clone()));
            let galley_height = galley.rect.height();

            // Calculate precise Y drawing position relative to the scroll state
            let draw_pos_y = start_y + current_y - self.scroll_y;

            // Frustum Culling: Draw only if the text is physically visible on the screen
            if draw_pos_y + galley_height > 0.0 && draw_pos_y < height {
                // Focus Mode Opacity Calculation: dim elements far from the guide line
                if self.enable_focus_mode && !is_header {
                    let distance_from_guide = (draw_pos_y - guide_y).abs();
                    let max_distance = 150.0;
                    let opacity = if distance_from_guide < max_distance {
                        let t = distance_from_guide / max_distance;
                        1.0 - t * 0.65 // Dim down to 35% opacity
                    } else {
                        0.35
                    };
                    for section in &mut job.sections {
                        section.format.color = section.format.color.linear_multiply(opacity);
                    }
                }
                
                // Re-lay out with updated opacity values
                let final_galley = ui.fonts(|f| f.layout_job(job));
                let text_pos = egui::Pos2::new(rect.min.x + padding, rect.min.y + draw_pos_y);
                let shape = egui::Shape::galley(text_pos, final_galley, self.text_color);
                self.paint_shape(ctx, ui.painter(), ui.clip_rect(), shape, center_x);
            }

            // Increment scroll offsets
            current_y += (galley_height * self.line_spacing) + extra_space;
        }

        // Cache calculated offsets
        self.sections = temp_sections;
        self.max_scroll = (current_y - height + guide_y + 100.0).max(0.0);

        // Calculate active section index
        let mut active_sec_idx = None;
        for (idx, &(_, offset_y)) in self.sections.iter().enumerate() {
            if self.scroll_y >= offset_y - 30.0 {
                active_sec_idx = Some(idx);
            }
        }

        // 2. Draw Reading Guide Line (semitransparent horizontal guide)
        if self.show_guide {
            let guide_color = egui::Color32::from_rgba_unmultiplied(239, 83, 80, 75); // Subtle red line
            let stroke = egui::Stroke::new(2.0, guide_color);
            self.paint_shape(
                ctx,
                ui.painter(),
                ui.clip_rect(),
                egui::Shape::line_segment(
                    [
                        egui::Pos2::new(rect.min.x + 15.0, rect.min.y + guide_y),
                        egui::Pos2::new(rect.max.x - 15.0, rect.min.y + guide_y),
                    ],
                    stroke,
                ),
                center_x,
            );
            
            // Side arrow indicators pointing inwards
            let arrow_color = egui::Color32::from_rgb(239, 83, 80);
            
            let left_arrow = vec![
                egui::Pos2::new(rect.min.x + 15.0, rect.min.y + guide_y - 8.0),
                egui::Pos2::new(rect.min.x + 15.0, rect.min.y + guide_y + 8.0),
                egui::Pos2::new(rect.min.x + 30.0, rect.min.y + guide_y),
            ];
            
            let right_arrow = vec![
                egui::Pos2::new(rect.max.x - 15.0, rect.min.y + guide_y - 8.0),
                egui::Pos2::new(rect.max.x - 15.0, rect.min.y + guide_y + 8.0),
                egui::Pos2::new(rect.max.x - 30.0, rect.min.y + guide_y),
            ];

            self.paint_shape(ctx, ui.painter(), ui.clip_rect(), egui::Shape::convex_polygon(left_arrow, arrow_color, egui::Stroke::NONE), center_x);
            self.paint_shape(ctx, ui.painter(), ui.clip_rect(), egui::Shape::convex_polygon(right_arrow, arrow_color, egui::Stroke::NONE), center_x);
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

        // 4. Draw Presenter Guide HUD (Left/Right panels in margins)
        let has_room_for_hud = padding >= 150.0;
        if self.show_hud && has_room_for_hud && self.countdown_secs <= 0.0 {
            let hud_text_color = if self.bg_color == egui::Color32::WHITE {
                egui::Color32::DARK_GRAY
            } else {
                egui::Color32::from_rgb(176, 190, 197) // Clean secondary grey
            };
            let cyan_accent = egui::Color32::from_rgb(0, 188, 212);

            // LEFT PANEL: Stopwatch Timer & Mini Table of Contents
            let left_panel_x = rect.min.x + 20.0;
            
            // Draw Stopwatch String
            let mins = (self.elapsed_secs / 60.0) as i32;
            let secs = (self.elapsed_secs % 60.0) as i32;
            let timer_str = format!("⏱  {:02}:{:02}", mins, secs);
            let font_timer = egui::FontId::new(22.0, egui::FontFamily::Proportional);
            let timer_galley = ui.fonts(|f| f.layout(timer_str, font_timer, cyan_accent, f32::INFINITY));
            self.paint_shape(
                ctx,
                ui.painter(),
                ui.clip_rect(),
                egui::Shape::galley(egui::Pos2::new(left_panel_x, rect.min.y + 50.0), timer_galley, cyan_accent),
                center_x,
            );

            // Draw Section List (ToC)
            let toc_start_y = rect.min.y + 100.0;
            let step_y = 24.0;
            let max_visible_sections = ((height - 180.0) / step_y) as usize;

            for (idx, (name, _)) in self.sections.iter().enumerate().take(max_visible_sections) {
                let is_active = Some(idx) == active_sec_idx;
                let color = if is_active { cyan_accent } else { hud_text_color };
                let prefix = if is_active { "● " } else { "○ " };
                
                // Truncate name if it exceeds the margin width safely
                let max_chars = ((padding - 40.0) / 10.0).max(10.0) as usize;
                let truncated_name: String = name.chars().take(max_chars).collect();
                let display_name = format!("{}{}", prefix, truncated_name);
                
                let font_sec = if is_active {
                    egui::FontId::new(13.0, egui::FontFamily::Proportional)
                } else {
                    egui::FontId::new(12.0, egui::FontFamily::Proportional)
                };

                let pos = egui::Pos2::new(left_panel_x, toc_start_y + idx as f32 * step_y);
                let sec_galley = ui.fonts(|f| f.layout(display_name, font_sec, color, f32::INFINITY));
                self.paint_shape(ctx, ui.painter(), ui.clip_rect(), egui::Shape::galley(pos, sec_galley, color), center_x);
            }

            // RIGHT PANEL: Dynamic Pace Estimator & Next Up Preview
            let right_panel_x = rect.max.x - padding + 25.0;
            let right_panel_y = rect.min.y + 50.0;

            // 1. Pace Estimator (CPM / WPM)
            let total_chars = self.text.chars().count();
            let cpm = if self.max_scroll > 0.0 && total_chars > 0 {
                let chars_per_pixel = total_chars as f32 / self.max_scroll;
                let chars_per_sec = self.scroll_speed * chars_per_pixel;
                (chars_per_sec * 60.0) as i32
            } else {
                0
            };
            let wpm = cpm / 5;
            let pace_str = format!("⚡ Pace: {} CPM\n          {} WPM", cpm, wpm);
            let font_pace = egui::FontId::new(13.0, egui::FontFamily::Proportional);
            let pace_galley = ui.fonts(|f| f.layout(pace_str, font_pace, hud_text_color, padding - 45.0));
            self.paint_shape(
                ctx,
                ui.painter(),
                ui.clip_rect(),
                egui::Shape::galley(egui::Pos2::new(right_panel_x, right_panel_y), pace_galley, hud_text_color),
                center_x,
            );

            // 2. Next Up Preview Card
            if let Some(active_idx) = active_sec_idx {
                if active_idx + 1 < self.sections.len() {
                    let next_name = &self.sections[active_idx + 1].0;
                    
                    let max_chars = ((padding - 45.0) / 10.0).max(10.0) as usize;
                    let truncated_next: String = next_name.chars().take(max_chars).collect();
                    let next_str = format!("⏭  Next Up:\n{}", truncated_next);
                    
                    let font_next = egui::FontId::new(13.0, egui::FontFamily::Proportional);
                    let next_galley = ui.fonts(|f| f.layout(next_str, font_next, cyan_accent, padding - 45.0));
                    self.paint_shape(
                        ctx,
                        ui.painter(),
                        ui.clip_rect(),
                        egui::Shape::galley(egui::Pos2::new(right_panel_x, right_panel_y + 60.0), next_galley, cyan_accent),
                        center_x,
                    );
                }
            }
        }

        // 5. Draw Scrolling Progress Bar (Cyan thin line at the top)
        if self.max_scroll > 0.0 {
            let progress = (self.scroll_y / self.max_scroll).clamp(0.0, 1.0);
            let progress_width = width * progress;
            let bar_rect = egui::Rect::from_min_max(
                rect.left_top(),
                egui::Pos2::new(rect.min.x + progress_width, rect.min.y + 4.0),
            );
            self.paint_shape(
                ctx,
                ui.painter(),
                ui.clip_rect(),
                egui::Shape::rect_filled(bar_rect, 0.0, egui::Color32::from_rgb(0, 188, 212)),
                center_x,
            );
        }

        // 6. Draw Countdown Overlay
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

        // 7. Control overlay (disappears after 2s of playing with no actions)
        let show_overlay = !self.is_playing || Instant::now().duration_since(self.last_action_time).as_secs_f32() < 2.0;
        if show_overlay && self.countdown_secs <= 0.0 {
            let overlay_bg = egui::Color32::from_rgba_unmultiplied(33, 33, 33, 200);
            let text_color = egui::Color32::WHITE;
            
            // Estimate Remaining Time Calculation
            let remaining_time_str = if self.scroll_speed > 0.0 {
                let remaining_secs = ((self.max_scroll - self.scroll_y) / self.scroll_speed).max(0.0);
                let mins = (remaining_secs / 60.0) as i32;
                let secs = (remaining_secs % 60.0) as i32;
                format!("{:02}:{:02}", mins, secs)
            } else {
                "--:--".to_string()
            };

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
                                let lang_str = match self.language_filter {
                                    LanguageFilter::All => "All",
                                    LanguageFilter::ChineseOnly => "CN Only",
                                    LanguageFilter::EnglishOnly => "EN Only",
                                };
                                ui.label(format!(
                                    "[{}] Rem: {} | Speed: {:.0} px/s | Width: {:.0}% | Lang: {} | Focus: {} | ESC: Exit",
                                    state_str,
                                    remaining_time_str,
                                    self.scroll_speed,
                                    self.text_width_pct * 100.0,
                                    lang_str,
                                    if self.enable_focus_mode { "ON" } else { "OFF" }
                                ));
                            });
                        });
                });
        }
    }
}

const DEFAULT_TEXT: &str = r#"=== PAGE 1: Opening & Hardware Product Definition ===
[中文]
各位评委老师，早上好。我是陶鑫旺。正如我作品集第一页所示，我将自己定位为**机器人系统研发工程师**与**工业AI全栈实践者**。我始终致力于打破代码与物理世界之间的边界。

我的第一个核心成果在于**硬件产品定义**。在李泽湘教授指导的InnoX创业营期间——您可以在图1中看到我们的路演合影——我主导了智能硬件Lumina从0到1的开发。针对年轻人因高功能焦虑引起的‘决策瘫痪’，我们创新地设计了**‘配重转子动态阻尼扭矩反馈’**的物理机制，将心理学机制具象化地实现在物理世界中。

[English]
Good morning, respected judges. I am Xinwang Tao. As shown on the first page of my portfolio, I position myself as a **Robotics System R&D Engineer** and an **Industrial AI Full-Stack Practitioner**. I am always driven to break the boundaries between code and the physical world.

My first core achievement lies in **hardware product definition**. During the InnoX Camp mentored by Prof. Zexiang Li—you can see our roadshow group photo in Fig. 1—I led the 0-to-1 development of the smart hardware, Lumina. Targeting the 'decision paralysis' caused by high-functioning anxiety among young people, we innovatively designed a physical mechanism with **'weighted rotor dynamic damping torque feedback,'** reifying psychological mechanisms into the physical world.

=== PAGE 2: Commercial Updates & Achievement 2: Industrial AI ===
[中文]
请翻到第2页。在产品落地阶段，如顶部图2所示，通过持续、高强度的MVP快速迭代，我们成功打磨出了一款高度精致的产品Demo。

**更令人兴奋的是，该项目目前正加速迈向深度商业化。我们团队最近迎来了两位实力强劲的新伙伴，目前正全力进军国际市场流量。我们正积极建立海外种子用户社区，并筹备在Kickstarter上发起众筹活动。** 这一从产品定义、Demo实现到全球化拓展的完整历程，使我荣获了‘优秀产品经理’称号。

除了敏捷硬件开发，我的第二个核心成果是**前线工业级AI的全栈部署**。请看中间的图3；这是我构建的工业AI Agent决策流架构。面对复杂、非标的SMT车间，我开发了一套高效的视频处理管线，清洗了1289个异常视频片段，作为视觉大模型（VLM）微调的基础。基于LangGraph状态机和ReAct逻辑，该闭环系统现已部署于实际生产线，展现出替代约30%重度人工视觉检测岗位的巨大潜力。

[English]
Please turn to PAGE 2. In the product implementation phase, as shown in Fig. 2 at the top, through continuous, high-intensity MVP rapid iterations, we have successfully polished a highly refined product Demo.

**Even more excitingly, this project is now accelerating into deep commercialization. We recently onboarded two strong partners to our team and are now fully targeting international market traffic. We are actively building our overseas seed user community and preparing to launch a crowdfunding campaign on Kickstarter.** This complete journey from product definition and Demo realization to global expansion earned me the 'Outstanding Product Manager' title.

Beyond agile hardware development, my second core achievement is the **full-stack deployment of frontline industrial AI**. Please look at Fig. 3 in the middle; this is the industrial AI Agent decision-making flow architecture I built. Facing the complex, non-standard SMT workshops, I developed a highly efficient video processing pipeline, cleaning 1,289 error-related video segments as a Vision Large Model (VLM) fine-tuning foundation. Based on the LangGraph state machine and ReAct logic, this closed-loop system is now deployed on actual production lines, demonstrating the immense potential to replace approximately 30% of heavy manual visual inspection roles.

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

为了构建技术壁垒，作为核心发明人，我累计获得了**10余项发明专利与软件著作权**。此外，底部的图9证明了我已获得鸿蒙（HarmonyOS）高级应用开发者认证，展现出全面的生态构建能力。

[English]
PAGE 4 showcases our commercial validation and technical moats. Fig. 6 is from the site where we won the National Silver Award in the 'Internet+' Competition. Fig. 7 is the certificate of our company's successful listing on the Hunan Equity Exchange.

To build technical barriers, as shown in Fig. 8 in the middle, I accumulated **over 10 invention patents and software copyrights** as the core inventor. Additionally, Fig. 9 at the bottom proves that I obtained the HarmonyOS Advanced Application Developer Certification, demonstrating a comprehensive ecosystem-building capability.

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

我坚信，顶尖的技术创新是**数字计算与人类感官体验的深度共鸣**。我非常渴望加入贵团队，与顶尖的跨学科头脑进行思想碰撞，共同定义机器人与自主系统（RAS）的下一个颠覆性范式！谢谢大家。

[English]
Finally, please look at PAGE 6. The top section summarizes my full-stack skill tree. From proficiently using AI tools like Cursor to build automated workflows, to completely refactoring the edge computing base from C++ to the higher-performance Rust language and deploying it on ESP32-S3 microcontrollers—I am constantly and agilely iterating my technical boundaries.

I firmly believe that top-tier technological innovation is a deep resonance between digital computing and human sensory experience. I am eager to join your team, brainstorm with top interdisciplinary minds, and co-define the next disruptive paradigm in Robotics and Autonomous Systems (RAS)! Thank you.
"#;
