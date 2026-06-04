// -*- coding: utf-8 -*-
use eframe::egui;
use std::time::Instant;
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Sisyphus Professional Rust Teleprompter")
            .with_inner_size([1024.0, 768.0])
            .with_transparent(true), // Enable OS transparent window capabilities
        ..Default::default()
    };
    
    // Spawn Web Remote Control server in the background
    let shared_state = Arc::new(Mutex::new(SharedRemoteState::default()));
    let state_clone = shared_state.clone();
    
    thread::spawn(move || {
        let listener = match TcpListener::bind("0.0.0.0:9090") {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Failed to bind TcpListener on port 9090: {}", e);
                return;
            }
        };
        for stream in listener.incoming() {
            if let Ok(stream) = stream {
                let state_inner = state_clone.clone();
                thread::spawn(move || {
                    handle_client(stream, state_inner);
                });
            }
        }
    });

    eframe::run_native(
        "Sisyphus Rust Teleprompter",
        options,
        Box::new(move |cc| {
            setup_custom_fonts(&cc.egui_ctx);
            configure_dark_theme(&cc.egui_ctx);
            Ok(Box::new(TeleprompterApp::new(shared_state)) as Box<dyn eframe::App>)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum AppMode {
    Edit,
    Prompter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum ColorPreset {
    WhiteOnBlack,
    YellowOnBlack,
    GreenOnBlack,
    CyanOnBlack,
    BlackOnWhite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum LanguageFilter {
    All,
    ChineseOnly,
    EnglishOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum UiLanguage {
    Chinese,
    English,
}

// Text Alignment Enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum TextAlignment {
    Left,
    Center,
    Right,
}

impl Default for TextAlignment {
    fn default() -> Self {
        TextAlignment::Left
    }
}

impl TextAlignment {
    fn to_egui_align(self) -> egui::Align {
        match self {
            TextAlignment::Left => egui::Align::Min,
            TextAlignment::Center => egui::Align::Center,
            TextAlignment::Right => egui::Align::Max,
        }
    }
}

// Section info structure
#[derive(Debug, Clone)]
struct SectionInfo {
    name: String,
    offset_y: f32,
    speed: Option<f32>,
    cues: Vec<String>,
}

// Shared State for Web Remote Controller
struct SharedRemoteState {
    is_playing: bool,
    scroll_speed: f32,
    scroll_y: f32,
    max_scroll: f32,
    elapsed_secs: f32,
    remaining_secs: f32,
    active_section_idx: Option<usize>,
    sections: Vec<String>,
    command_queue: Vec<String>,
}

impl Default for SharedRemoteState {
    fn default() -> Self {
        Self {
            is_playing: false,
            scroll_speed: 60.0,
            scroll_y: 0.0,
            max_scroll: 0.0,
            elapsed_secs: 0.0,
            remaining_secs: 0.0,
            active_section_idx: None,
            sections: Vec::new(),
            command_queue: Vec::new(),
        }
    }
}

// Configuration persistence structure
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct AppConfig {
    font_size: f32,
    scroll_speed: f32,
    line_spacing: f32,
    text_width_pct: f32,
    is_mirrored: bool,
    show_guide: bool,
    guide_y_pct: f32,
    show_edge_fade: bool,
    enable_focus_mode: bool,
    show_hud: bool,
    color_preset: ColorPreset,
    ui_language: UiLanguage,
    always_on_top: bool,
    enable_timer_limit: bool,
    timer_limit_minutes: f32,
    target_duration_minutes: f32,
    window_opacity: f32,
    countdown_duration_secs: f32,
    enable_slide_alerts: bool,
    #[serde(default)]
    transparent_background: bool,
    #[serde(default)]
    mouse_passthrough: bool,
    #[serde(default)]
    text_align: TextAlignment,
    #[serde(default = "default_true")]
    enable_web_remote: bool, // Toggle server integration
}

fn default_true() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            font_size: 48.0,
            scroll_speed: 60.0,
            line_spacing: 1.4,
            text_width_pct: 0.8,
            is_mirrored: false,
            show_guide: true,
            guide_y_pct: 0.33,
            show_edge_fade: true,
            enable_focus_mode: true,
            show_hud: true,
            color_preset: ColorPreset::WhiteOnBlack,
            ui_language: UiLanguage::Chinese,
            always_on_top: false,
            enable_timer_limit: true,
            timer_limit_minutes: 4.5,
            target_duration_minutes: 4.5,
            window_opacity: 1.0,
            countdown_duration_secs: 3.0,
            enable_slide_alerts: true,
            transparent_background: false,
            mouse_passthrough: false,
            text_align: TextAlignment::Left,
            enable_web_remote: true,
        }
    }
}

fn load_config() -> AppConfig {
    let paths = ["config.json", "F:\\rust-teleprompter\\config.json"];
    for path in paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                return config;
            }
        }
    }
    AppConfig::default()
}

fn save_config(config: &AppConfig) {
    if let Ok(content) = serde_json::to_string_pretty(config) {
        let _ = std::fs::write("config.json", &content);
        let _ = std::fs::write("F:\\rust-teleprompter\\config.json", &content);
    }
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
    countdown_secs: f32, // Preparation countdown
    show_edge_fade: bool, // Top and bottom gradient fades
    line_spacing: f32, // Line height multiplier (1.0 to 2.5)
    sections: Vec<SectionInfo>, // Struct containing header, Y offset, parsed speed, and cues
    max_scroll: f32, // Store dynamically calculated max scroll limit
    language_filter: LanguageFilter,
    enable_focus_mode: bool, // Dim unread text blocks to enhance focus
    
    // Presenter HUD:
    elapsed_secs: f32, // Stopwatch elapsed time
    show_hud: bool, // Toggle Presenter Guide HUD (default true)
    
    // Localization & Window Features (v1.0.6):
    ui_language: UiLanguage,
    always_on_top: bool,
    prev_always_on_top: bool,
    enable_timer_limit: bool, // Count down instead of counting up
    timer_limit_minutes: f32, // Total countdown timer in minutes
    remaining_limit_secs: f32, // Countdown state variable in seconds

    // Iteration Upgrades (v1.0.8):
    target_duration_minutes: f32, // Input target for speed calibration
    target_scroll_y: f32, // Dampened target Y coordinate for smooth scrolling
    fullscreen: bool,
    prev_fullscreen: bool,

    // Iteration Upgrades (v1.0.9):
    window_opacity: f32, // Glass/overlay transparency (0.1 to 1.0)
    last_active_sec_idx: Option<usize>, // Track transition between sections to trigger autocompleted events

    // Iteration Upgrades (v1.1.0):
    countdown_duration_secs: f32, // User configurable countdown duration (1s to 10s)
    enable_slide_alerts: bool, // Toggle flashing slide prompts
    active_slide_alert: Option<String>, // Stores slide number to flash (e.g. "PAGE 3")

    // Iteration Upgrades (v1.1.1):
    transparent_background: bool, // Toggle completely transparent overlay mode

    // Iteration Upgrades (v1.1.2):
    mouse_passthrough: bool, // Toggle click-through ghost mode
    prev_mouse_passthrough: bool,
    text_align: TextAlignment, // Text alignment selector

    // Iteration Upgrades (v1.1.3):
    enable_web_remote: bool, // Toggle HTTP Remote controller
    shared_remote_state: Arc<Mutex<SharedRemoteState>>, // Cross-thread shared controller state
    local_ip: String, // Resolved local IP address for phone remote scanning
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

// Find active LAN IP address by connecting a temporary UDP socket to public DNS
fn get_local_ip() -> Option<String> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let local_addr = socket.local_addr().ok()?;
    Some(local_addr.ip().to_string())
}

impl TeleprompterApp {
    fn new(shared_state: Arc<Mutex<SharedRemoteState>>) -> Self {
        let config = load_config();
        let local_ip = get_local_ip().unwrap_or_else(|| "127.0.0.1".to_string());
        
        let mut app = Self {
            text: load_initial_text(),
            font_size: config.font_size,
            scroll_speed: config.scroll_speed,
            scroll_y: 0.0,
            is_playing: false,
            is_mirrored: config.is_mirrored,
            show_guide: config.show_guide,
            guide_y_pct: config.guide_y_pct,
            color_preset: config.color_preset,
            text_color: egui::Color32::WHITE,
            bg_color: egui::Color32::BLACK,
            mode: AppMode::Edit,
            last_update: Instant::now(),
            last_action_time: Instant::now(),
            text_width_pct: config.text_width_pct,
            countdown_secs: 0.0,
            show_edge_fade: config.show_edge_fade,
            line_spacing: config.line_spacing,
            sections: Vec::new(),
            max_scroll: 1000.0,
            language_filter: LanguageFilter::All,
            enable_focus_mode: config.enable_focus_mode,
            elapsed_secs: 0.0,
            show_hud: config.show_hud,
            
            ui_language: config.ui_language,
            always_on_top: config.always_on_top,
            prev_always_on_top: config.always_on_top,
            enable_timer_limit: config.enable_timer_limit,
            timer_limit_minutes: config.timer_limit_minutes,
            remaining_limit_secs: config.timer_limit_minutes * 60.0,

            target_duration_minutes: config.target_duration_minutes,
            target_scroll_y: 0.0,
            fullscreen: false,
            prev_fullscreen: false,

            window_opacity: config.window_opacity,
            last_active_sec_idx: None,

            countdown_duration_secs: config.countdown_duration_secs,
            enable_slide_alerts: config.enable_slide_alerts,
            active_slide_alert: None,
            transparent_background: config.transparent_background,

            mouse_passthrough: config.mouse_passthrough,
            prev_mouse_passthrough: config.mouse_passthrough,
            text_align: config.text_align,

            enable_web_remote: config.enable_web_remote,
            shared_remote_state: shared_state,
            local_ip,
        };
        app.apply_color_preset();
        app
    }
}

impl TeleprompterApp {
    // Translation dictionary mapping
    fn tr(&self, key: &str) -> &'static str {
        match self.ui_language {
            UiLanguage::Chinese => match key {
                "title" => "🚀 Sisyphus 专业级智能提词器",
                "start_prompter" => "⚡ 启动提词器 (空格键)",
                "settings" => "🎛️ 设置面板",
                "font_size" => "字体大小:",
                "scroll_speed" => "滚动速度:",
                "line_height" => "行距倍数:",
                "column_width" => "文本宽度:",
                "mirror" => "🪞 水平镜像反转 (提词镜反射专用)",
                "guide" => "🎯 显示黄金视线红辅助线",
                "guide_pos" => "辅助线高度:",
                "edge_fade" => "🎬 开启顶底边缘电影级淡出",
                "focus_mode" => "👁️ 开启视线聚焦淡化模式",
                "show_hud" => "📊 启用侧边 Presenter HUD 面板",
                "lang_filter" => "文本语言过滤:",
                "color_preset" => "配色主题:",
                "shortcuts" => "⌨️ 快捷键说明 (提词模式)",
                "sc_space" => "• 空格键：播放 / 暂停滚屏",
                "sc_esc" => "• Esc 键：退出提词，返回编辑区",
                "sc_arrows" => "• 上/下方向键：微调速度 (+/- 5)",
                "sc_scroll" => "• 鼠标滚轮：滚屏 (暂停) / 调速 (播放)",
                "sc_num" => "• 数字键 1-9：快速跳转对应 PPT 章节",
                "sc_l" => "• L 键：循环切换中英文语言过滤",
                "sc_h" => "• H 键：显示 / 隐藏侧边 HUD 面板",
                "sc_minus" => "• 减号 (-) / 等号 (=)：微调辅助线高度",
                "sc_r" => "• R 键：重置滚动位置与计时器",
                "sc_m" => "• M 键：切换物理镜像模式",
                "sc_g" => "• G 键：切换视线红辅助线",
                "editor_title" => "📝 演讲稿编辑区 (实时自动存盘):",
                "always_on_top" => "📌 窗口始终置顶 (Always on Top)",
                "timer_limit" => "⏱️ 倒计时限制 (分钟):",
                "timer_enable" => "启用限时倒计时",
                "timer_ended" => "⚠️ 时间已到！",
                "timer_title" => "计时器",
                "ui_lang" => "界面语言 (UI Language):",
                "clear_text" => "🗑️ 清空文本",
                "load_template" => "📋 载入模板",
                "target_duration" => "🎯 演讲目标时长 (分钟):",
                "calibrate_btn" => "🧮 根据目标时长自动对齐滚速",
                "fullscreen_hint" => "• F11 / F 键：切换全屏模式 (Fullscreen)",
                "window_opacity" => "🪟 玻璃悬浮窗不透明度 (Opacity):",
                "cue_title" => "💡 演示提示 (Presenter Cue):",
                "countdown_duration" => "⏲️ 准备倒计时时长 (秒):",
                "enable_slide_alerts" => "📢 开启幻灯片物理翻页强提醒",
                "slide_alert_banner" => "👉 请将 PPT 翻页至：",
                "stats_title" => "📊 演讲稿分析统计",
                "stats_chars" => "• 字符数(含标点):",
                "stats_words" => "• 英文单词数:",
                "stats_time" => "• 预计阅读时长(按当前速度):",
                "transparent_background" => "👻 开启背景完全透明 (仅文字可见)",
                "mouse_passthrough" => "👻 开启鼠标穿透 (忽略点击，可操作底层软件)",
                "text_align" => "文字对齐方式 (Alignment):",
                "align_left" => "左对齐",
                "align_center" => "居中对齐",
                "align_right" => "右对齐",
                "sc_passthrough_tip" => "💡 提示：开启鼠标穿透后，您可直接点击底层的PPT。如需退出，请在任务栏中点击本软件重新激活，再按 Esc 退出。",
                "web_remote" => "📱 开启手机网页远程控制 (Mobile Remote)",
                "remote_url" => "🔗 遥控器网址 (请用手机浏览器访问):",
                _ => "",
            },
            UiLanguage::English => match key {
                "title" => "🚀 Sisyphus Professional Smart Teleprompter",
                "start_prompter" => "⚡ Start Prompter (Space)",
                "settings" => "🎛️ Settings",
                "font_size" => "Font Size:",
                "scroll_speed" => "Scroll Speed:",
                "line_height" => "Line Height:",
                "column_width" => "Column Width:",
                "mirror" => "🪞 Mirror Text (Horizontal Flip for Glass)",
                "guide" => "🎯 Show Reading Guide Line",
                "guide_pos" => "Guide Position:",
                "edge_fade" => "🎬 Enable Cinema Edge Fade-Out",
                "focus_mode" => "👁️ Enable Active Line Focus Mode",
                "show_hud" => "📊 Enable Presenter Side HUD Panels",
                "lang_filter" => "Language Block Filter:",
                "color_preset" => "Color Preset:",
                "shortcuts" => "⌨️ Shortcut Keys (Prompter Mode)",
                "sc_space" => "• Spacebar: Play / Pause scrolling",
                "sc_esc" => "• Esc: Exit to Edit Mode",
                "sc_arrows" => "• Up / Down Arrow: Speed up / slow down (+/- 5)",
                "sc_scroll" => "• Mouse Wheel: Scroll manually (paused) / adjust speed (playing)",
                "sc_num" => "• Keys 1-9: Jump directly to mapped Page Sections",
                "sc_l" => "• L Key: Toggle Language Filters",
                "sc_h" => "• H Key: Toggle Presenter Guide HUD",
                "sc_minus" => "• Minus (-) / Equals (=): Move guide line up/down",
                "sc_r" => "• R Key: Reset scroll to top & timer",
                "sc_m" => "• M Key: Toggle Mirroring",
                "sc_g" => "• G Key: Toggle Guide line",
                "editor_title" => "📝 Enter Presentation Script (Autosaved):",
                "always_on_top" => "📌 Always on Top",
                "timer_limit" => "⏱️ Time Limit (Minutes):",
                "timer_enable" => "Enable Limit Countdown",
                "timer_ended" => "⚠️ TIME'S UP!",
                "timer_title" => "Timer",
                "ui_lang" => "UI Language:",
                "clear_text" => "🗑️ Clear Text",
                "load_template" => "📋 Load Template",
                "target_duration" => "🎯 Target Duration (Minutes):",
                "calibrate_btn" => "🧮 Align Scroll Speed to Target",
                "fullscreen_hint" => "• F11 / F Key: Toggle borderless Fullscreen",
                "window_opacity" => "🪟 Window Glass Opacity:",
                "cue_title" => "💡 Presenter Cue:",
                "countdown_duration" => "Countdown Duration (s):",
                "enable_slide_alerts" => "📢 Enable Slide Flip Alert Badges",
                "slide_alert_banner" => "👉 FLIP TO SLIDE:",
                "stats_title" => "📊 Script Analysis & Metrics",
                "stats_chars" => "• Total Characters:",
                "stats_words" => "• English Word Count:",
                "stats_time" => "• Est. Reading Time (at current speed):",
                "transparent_background" => "👻 Pure Transparent Background (Text Only)",
                "mouse_passthrough" => "👻 Enable Mouse Passthrough (Ignore clicks for overlay)",
                "text_align" => "Text Alignment:",
                "align_left" => "Left",
                "align_center" => "Center",
                "align_right" => "Right",
                "sc_passthrough_tip" => "💡 Note: When Passthrough is ON, click underlying apps. To exit, click this app's taskbar icon to refocus, then press Esc.",
                "web_remote" => "📱 Enable Web Mobile Remote Control",
                "remote_url" => "🔗 Remote URL (Open in phone browser):",
                _ => "",
            }
        }
    }

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
            self.target_scroll_y = self.sections[idx].offset_y;
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

    fn save_settings(&self) {
        let config = AppConfig {
            font_size: self.font_size,
            scroll_speed: self.scroll_speed,
            line_spacing: self.line_spacing,
            text_width_pct: self.text_width_pct,
            is_mirrored: self.is_mirrored,
            show_guide: self.show_guide,
            guide_y_pct: self.guide_y_pct,
            show_edge_fade: self.show_edge_fade,
            enable_focus_mode: self.enable_focus_mode,
            show_hud: self.show_hud,
            color_preset: self.color_preset,
            ui_language: self.ui_language,
            always_on_top: self.always_on_top,
            enable_timer_limit: self.enable_timer_limit,
            timer_limit_minutes: self.timer_limit_minutes,
            target_duration_minutes: self.target_duration_minutes,
            window_opacity: self.window_opacity,
            countdown_duration_secs: self.countdown_duration_secs,
            enable_slide_alerts: self.enable_slide_alerts,
            transparent_background: self.transparent_background,
            mouse_passthrough: self.mouse_passthrough,
            text_align: self.text_align,
            enable_web_remote: self.enable_web_remote,
        };
        save_config(&config);
    }

    // Dynamic Speed Calibration based on pixel height and target duration
    fn calibrate_speed_to_target(&mut self) {
        let total_seconds = self.target_duration_minutes * 60.0;
        if total_seconds > 0.0 && self.max_scroll > 0.0 {
            self.scroll_speed = (self.max_scroll / total_seconds).clamp(5.0, 500.0);
            self.save_settings();
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
    align: egui::Align, // Pass text alignment
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = wrapping_width;
    job.halign = align; // Apply alignment
    
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

impl TeleprompterApp {
    fn consume_web_remote_commands(&mut self, ctx: &egui::Context) {
        if !self.enable_web_remote {
            return;
        }
        
        // Extract commands from shared remote state by copying them first to release borrow on self
        let commands = {
            let mut s = match self.shared_remote_state.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };
            s.command_queue.drain(..).collect::<Vec<String>>()
        };
        
        // Apply commands queued by phone controller safely
        for cmd in commands {
            self.record_action();
            match cmd.as_str() {
                "toggle" => {
                    if self.countdown_secs > 0.0 {
                        self.countdown_secs = 0.0;
                        self.is_playing = true;
                    } else {
                        self.is_playing = !self.is_playing;
                    }
                }
                "play" => {
                    self.countdown_secs = 0.0;
                    self.is_playing = true;
                }
                "pause" => {
                    self.is_playing = false;
                }
                "speed_up" => {
                    self.scroll_speed = (self.scroll_speed + 10.0).min(500.0);
                }
                "speed_down" => {
                    self.scroll_speed = (self.scroll_speed - 10.0).max(5.0);
                }
                "page_up" => {
                    self.target_scroll_y = (self.target_scroll_y - 150.0).max(0.0);
                    ctx.request_repaint();
                }
                "page_down" => {
                    self.target_scroll_y = (self.target_scroll_y + 150.0).min(self.max_scroll);
                    ctx.request_repaint();
                }
                "reset" => {
                    self.target_scroll_y = 0.0;
                    self.scroll_y = 0.0;
                    self.is_playing = false;
                    self.countdown_secs = 0.0;
                    self.elapsed_secs = 0.0;
                    self.remaining_limit_secs = self.timer_limit_minutes * 60.0;
                    self.active_slide_alert = None;
                    ctx.request_repaint();
                }
                c if c.starts_with("jump:") => {
                    if let Ok(idx) = c["jump:".len()..].parse::<usize>() {
                        self.jump_to_section(idx);
                        ctx.request_repaint();
                    }
                }
                _ => {}
            }
        }

        // Push current GUI state to Web Remote State under a separate lock scope
        if let Ok(mut s) = self.shared_remote_state.lock() {
            s.is_playing = self.is_playing;
            s.scroll_speed = self.scroll_speed;
            s.scroll_y = self.scroll_y;
            s.max_scroll = self.max_scroll;
            s.elapsed_secs = self.elapsed_secs;
            s.remaining_secs = if self.enable_timer_limit { self.remaining_limit_secs } else { self.elapsed_secs };
            
            // Find active section index
            let mut active_idx = None;
            for (idx, sec) in self.sections.iter().enumerate() {
                if self.scroll_y >= sec.offset_y - 30.0 {
                    active_idx = Some(idx);
                }
            }
            s.active_section_idx = active_idx;
            s.sections = self.sections.iter().map(|s| s.name.clone()).collect();
        }
    }
}

impl eframe::App for TeleprompterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_update).as_secs_f32();
        self.last_update = now;

        // Viewport Always-on-top command dispatcher
        if self.always_on_top != self.prev_always_on_top {
            let level = if self.always_on_top {
                egui::WindowLevel::AlwaysOnTop
            } else {
                egui::WindowLevel::Normal
            };
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
            self.prev_always_on_top = self.always_on_top;
        }

        // Viewport Fullscreen command dispatcher
        if self.fullscreen != self.prev_fullscreen {
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fullscreen));
            self.prev_fullscreen = self.fullscreen;
        }

        // Mouse Passthrough / Ghost Mode command dispatcher (v1.1.2)
        let should_passthrough = self.mode == AppMode::Prompter && self.mouse_passthrough;
        if should_passthrough != self.prev_mouse_passthrough {
            ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(should_passthrough));
            self.prev_mouse_passthrough = should_passthrough;
        }

        // Auto-scrolling in Prompter mode (with countdown pause)
        if self.mode == AppMode::Prompter {
            if self.countdown_secs > 0.0 {
                self.countdown_secs = (self.countdown_secs - dt).max(0.0);
                if self.countdown_secs == 0.0 {
                    self.is_playing = true;
                }
                ctx.request_repaint();
            } else if self.is_playing {
                self.target_scroll_y = (self.target_scroll_y + self.scroll_speed * dt).min(self.max_scroll);
                self.elapsed_secs += dt;
                
                if self.enable_timer_limit {
                    self.remaining_limit_secs = (self.remaining_limit_secs - dt).max(0.0);
                }
                
                if self.scroll_y >= self.max_scroll {
                    self.is_playing = false;
                }
                ctx.request_repaint();
            }

            // Smooth scrolling interpolation (inertial lerp for cinema scrolling)
            if (self.scroll_y - self.target_scroll_y).abs() > 0.05 {
                let lerp_factor = (dt * 10.0).min(1.0); // 10.0 speed factor
                self.scroll_y += (self.target_scroll_y - self.scroll_y) * lerp_factor;
                ctx.request_repaint();
            } else {
                self.scroll_y = self.target_scroll_y;
            }
        }

        // Apply alpha transparency values to the background in Prompter mode
        let final_bg = if self.mode == AppMode::Prompter {
            if self.transparent_background {
                egui::Color32::TRANSPARENT // Pure transparent window (v1.1.1)
            } else {
                let alpha = (self.window_opacity * 255.0) as u8;
                egui::Color32::from_rgba_unmultiplied(
                    self.bg_color.r(),
                    self.bg_color.g(),
                    self.bg_color.b(),
                    alpha,
                )
            }
        } else {
            ctx.style().visuals.window_fill()
        };
        let frame_style = egui::Frame::none().fill(final_bg);
        
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

        // Take a snapshot of settings before UI actions to check for changes
        let old_font_size = self.font_size;
        let old_scroll_speed = self.scroll_speed;
        let old_line_spacing = self.line_spacing;
        let old_text_width_pct = self.text_width_pct;
        let old_is_mirrored = self.is_mirrored;
        let old_show_guide = self.show_guide;
        let old_guide_y_pct = self.guide_y_pct;
        let old_show_edge_fade = self.show_edge_fade;
        let old_enable_focus_mode = self.enable_focus_mode;
        let old_show_hud = self.show_hud;
        let old_color_preset = self.color_preset;
        let old_ui_language = self.ui_language;
        let old_always_on_top = self.always_on_top;
        let old_enable_timer_limit = self.enable_timer_limit;
        let old_timer_limit_minutes = self.timer_limit_minutes;
        let old_target_duration_minutes = self.target_duration_minutes;
        let old_window_opacity = self.window_opacity;
        let old_countdown_duration_secs = self.countdown_duration_secs;
        let old_enable_slide_alerts = self.enable_slide_alerts;
        let old_transparent_background = self.transparent_background;
        let old_mouse_passthrough = self.mouse_passthrough;
        let old_text_align = self.text_align;
        let old_enable_web_remote = self.enable_web_remote;

        // Evaluate all translation keys FIRST to prevent immutable-mutable borrow conflicts on self
        let tr_title = self.tr("title");
        let tr_start_prompter = self.tr("start_prompter");
        let tr_settings = self.tr("settings");
        let tr_ui_lang = self.tr("ui_lang");
        let tr_always_on_top = self.tr("always_on_top");
        let tr_timer_enable = self.tr("timer_enable");
        let tr_timer_limit = self.tr("timer_limit");
        let tr_font_size = self.tr("font_size");
        let tr_scroll_speed = self.tr("scroll_speed");
        let tr_line_height = self.tr("line_height");
        let tr_column_width = self.tr("column_width");
        let tr_mirror = self.tr("mirror");
        let tr_guide = self.tr("guide");
        let tr_guide_pos = self.tr("guide_pos");
        let tr_edge_fade = self.tr("edge_fade");
        let tr_focus_mode = self.tr("focus_mode");
        let tr_show_hud = self.tr("show_hud");
        let tr_lang_filter = self.tr("lang_filter");
        let tr_color_preset = self.tr("color_preset");
        let tr_shortcuts = self.tr("shortcuts");
        let tr_editor_title = self.tr("editor_title");
        
        let sc_space = self.tr("sc_space");
        let sc_esc = self.tr("sc_esc");
        let sc_arrows = self.tr("sc_arrows");
        let sc_scroll = self.tr("sc_scroll");
        let sc_num = self.tr("sc_num");
        let sc_l = self.tr("sc_l");
        let sc_h = self.tr("sc_h");
        let sc_minus = self.tr("sc_minus");
        let sc_r = self.tr("sc_r");
        let sc_m = self.tr("sc_m");
        let sc_g = self.tr("sc_g");

        // v1.0.8 to v1.1.3 translations
        let tr_target_duration = self.tr("target_duration");
        let tr_calibrate_btn = self.tr("calibrate_btn");
        let tr_fullscreen_hint = self.tr("fullscreen_hint");
        let tr_window_opacity = self.tr("window_opacity");
        let tr_countdown_duration = self.tr("countdown_duration");
        let tr_enable_slide_alerts = self.tr("enable_slide_alerts");
        let tr_stats_title = self.tr("stats_title");
        let tr_stats_chars = self.tr("stats_chars");
        let tr_stats_words = self.tr("stats_words");
        let tr_stats_time = self.tr("stats_time");
        let tr_transparent_background = self.tr("transparent_background");
        let tr_mouse_passthrough = self.tr("mouse_passthrough");
        let tr_text_align = self.tr("text_align");
        let tr_align_left = self.tr("align_left");
        let tr_align_center = self.tr("align_center");
        let tr_align_right = self.tr("align_right");
        
        let tr_web_remote = self.tr("web_remote");
        let tr_remote_url = self.tr("remote_url");

        // Calculate Text Statistics
        let char_count = self.text.chars().count();
        let word_count = self.text.split_whitespace().filter(|w| w.chars().any(|c| c.is_alphabetic())).count();
        let est_minutes = if self.scroll_speed > 0.0 {
            let total_seconds = self.max_scroll / self.scroll_speed;
            let m = (total_seconds / 60.0) as i32;
            let s = (total_seconds % 60.0) as i32;
            format!("{:02}:{:02}", m, s)
        } else {
            "--:--".to_string()
        };

        ui.vertical(|ui| {
            // Header bar
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.heading(tr_title);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(tr_start_prompter).clicked() {
                        self.mode = AppMode::Prompter;
                        self.scroll_y = 0.0;
                        self.target_scroll_y = 0.0;
                        self.countdown_secs = self.countdown_duration_secs;
                        self.is_playing = false;
                        self.elapsed_secs = 0.0;
                        self.remaining_limit_secs = self.timer_limit_minutes * 60.0;
                        self.fullscreen = false;
                        self.last_active_sec_idx = None;
                        self.active_slide_alert = None;
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
                        ui.heading(tr_settings);
                        ui.add_space(10.0);

                        // UI Language Toggle Buttons
                        ui.horizontal(|ui| {
                            ui.label(tr_ui_lang);
                            ui.selectable_value(&mut self.ui_language, UiLanguage::Chinese, "中文");
                            ui.selectable_value(&mut self.ui_language, UiLanguage::English, "English");
                        });
                        ui.add_space(8.0);

                        // Window always on top checkbox
                        ui.checkbox(&mut self.always_on_top, tr_always_on_top);
                        ui.add_space(6.0);

                        // Mouse passthrough click-through toggle
                        ui.checkbox(&mut self.mouse_passthrough, tr_mouse_passthrough);
                        ui.add_space(6.0);

                        // Pure Transparent Background checkbox
                        ui.checkbox(&mut self.transparent_background, tr_transparent_background);
                        ui.add_space(6.0);

                        // Window glass opacity/transparency slider (only enabled if not fully transparent)
                        if !self.transparent_background {
                            ui.horizontal(|ui| {
                                ui.label(tr_window_opacity);
                                ui.add(egui::Slider::new(&mut self.window_opacity, 0.1..=1.0).suffix(" x"));
                            });
                            ui.add_space(6.0);
                        }

                        // Text Alignment Selector (v1.1.2)
                        ui.horizontal(|ui| {
                            ui.label(tr_text_align);
                            ui.selectable_value(&mut self.text_align, TextAlignment::Left, tr_align_left);
                            ui.selectable_value(&mut self.text_align, TextAlignment::Center, tr_align_center);
                            ui.selectable_value(&mut self.text_align, TextAlignment::Right, tr_align_right);
                        });
                        ui.add_space(6.0);

                        // Web mobile remote controller checkbox (v1.1.3)
                        ui.checkbox(&mut self.enable_web_remote, tr_web_remote);
                        if self.enable_web_remote {
                            ui.add_space(2.0);
                            ui.horizontal(|ui| {
                                ui.label(tr_remote_url);
                                ui.colored_label(egui::Color32::from_rgb(0, 188, 212), format!("http://{}:9090", self.local_ip));
                            });
                        }
                        ui.add_space(8.0);

                        ui.separator();
                        ui.add_space(8.0);

                        // Countdown duration adjust
                        ui.horizontal(|ui| {
                            ui.label(tr_countdown_duration);
                            ui.add(egui::Slider::new(&mut self.countdown_duration_secs, 1.0..=10.0).suffix(" s"));
                        });
                        ui.add_space(6.0);

                        // Slide Flip alerts toggle
                        ui.checkbox(&mut self.enable_slide_alerts, tr_enable_slide_alerts);
                        ui.add_space(6.0);

                        // Timer configurations
                        ui.checkbox(&mut self.enable_timer_limit, tr_timer_enable);
                        if self.enable_timer_limit {
                            ui.horizontal(|ui| {
                                ui.label(tr_timer_limit);
                                ui.add(egui::Slider::new(&mut self.timer_limit_minutes, 0.5..=15.0).suffix(" m"));
                            });
                        }
                        ui.add_space(8.0);

                        ui.separator();
                        ui.add_space(8.0);

                        // Target Duration speed calibrator
                        ui.horizontal(|ui| {
                            ui.label(tr_target_duration);
                            ui.add(egui::Slider::new(&mut self.target_duration_minutes, 0.5..=10.0).suffix(" m"));
                        });
                        ui.add_space(4.0);
                        if ui.button(tr_calibrate_btn).clicked() {
                            self.calibrate_speed_to_target();
                        }
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        ui.horizontal(|ui| {
                            ui.label(tr_font_size);
                            ui.add(egui::Slider::new(&mut self.font_size, 16.0..=120.0).suffix(" px"));
                        });
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            ui.label(tr_scroll_speed);
                            ui.add(egui::Slider::new(&mut self.scroll_speed, 10.0..=500.0).suffix(" px/s"));
                        });
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            ui.label(tr_line_height);
                            ui.add(egui::Slider::new(&mut self.line_spacing, 1.0..=2.5).suffix(" x"));
                        });
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            ui.label(tr_column_width);
                            ui.add(egui::Slider::new(&mut self.text_width_pct, 0.4..=0.95).text("Width %"));
                        });
                        ui.add_space(6.0);

                        ui.checkbox(&mut self.is_mirrored, tr_mirror);
                        ui.checkbox(&mut self.show_guide, tr_guide);
                        
                        if self.show_guide {
                            ui.horizontal(|ui| {
                                ui.label(tr_guide_pos);
                                ui.add(egui::Slider::new(&mut self.guide_y_pct, 0.1..=0.9));
                            });
                        }
                        ui.add_space(6.0);

                        ui.checkbox(&mut self.show_edge_fade, tr_edge_fade);
                        ui.checkbox(&mut self.enable_focus_mode, tr_focus_mode);
                        ui.checkbox(&mut self.show_hud, tr_show_hud);
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            ui.label(tr_lang_filter);
                            egui::ComboBox::from_id_source("lang_filter_combo")
                                .selected_text(match self.language_filter {
                                    LanguageFilter::All => match self.ui_language {
                                        UiLanguage::Chinese => "全部显示 (中英文)",
                                        UiLanguage::English => "Show All (CN & EN)",
                                    },
                                    LanguageFilter::ChineseOnly => match self.ui_language {
                                        UiLanguage::Chinese => "仅中文",
                                        UiLanguage::English => "Chinese Only (中文)",
                                    },
                                    LanguageFilter::EnglishOnly => match self.ui_language {
                                        UiLanguage::Chinese => "仅英文 (English)",
                                        UiLanguage::English => "English Only",
                                    },
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.language_filter, LanguageFilter::All, match self.ui_language {
                                        UiLanguage::Chinese => "全部显示 (中英文)",
                                        UiLanguage::English => "Show All (CN & EN)",
                                    });
                                    ui.selectable_value(&mut self.language_filter, LanguageFilter::ChineseOnly, match self.ui_language {
                                        UiLanguage::Chinese => "仅中文",
                                        UiLanguage::English => "Chinese Only (中文)",
                                    });
                                    ui.selectable_value(&mut self.language_filter, LanguageFilter::EnglishOnly, match self.ui_language {
                                        UiLanguage::Chinese => "仅英文 (English)",
                                        UiLanguage::English => "English Only",
                                    });
                                });
                        });
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            ui.label(tr_color_preset);
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

                    // Text Script Stats Box
                    ui.add_space(10.0);
                    ui.group(|ui| {
                        ui.heading(tr_stats_title);
                        ui.add_space(5.0);
                        ui.label(format!("{} {}", tr_stats_chars, char_count));
                        ui.label(format!("{} {}", tr_stats_words, word_count));
                        ui.label(format!("{} {}", tr_stats_time, est_minutes));
                    });

                    ui.add_space(15.0);
                    ui.group(|ui| {
                        ui.heading(tr_shortcuts);
                        ui.add_space(6.0);
                        ui.label(sc_space);
                        ui.label(sc_esc);
                        ui.label(sc_arrows);
                        ui.label(sc_scroll);
                        ui.label(sc_num);
                        ui.label(sc_l);
                        ui.label(sc_h);
                        ui.label(tr_fullscreen_hint);
                        ui.label(sc_minus);
                        ui.label(sc_r);
                        ui.label(sc_m);
                        ui.label(sc_g);
                    });
                });

                // Column 1: Script Editor (Auto-saves changes, with template/clear buttons)
                columns[1].vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(tr_editor_title);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(self.tr("load_template")).clicked() {
                                self.text = DEFAULT_TEXT.to_string();
                                save_text(&self.text);
                            }
                            if ui.button(self.tr("clear_text")).clicked() {
                                self.text = String::new();
                                save_text(&self.text);
                            }
                        });
                    });
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

        // Trigger settings persistence if any config value was changed
        if old_font_size != self.font_size
            || old_scroll_speed != self.scroll_speed
            || old_line_spacing != self.line_spacing
            || old_text_width_pct != self.text_width_pct
            || old_is_mirrored != self.is_mirrored
            || old_show_guide != self.show_guide
            || old_guide_y_pct != self.guide_y_pct
            || old_show_edge_fade != self.show_edge_fade
            || old_enable_focus_mode != self.enable_focus_mode
            || old_show_hud != self.show_hud
            || old_color_preset != self.color_preset
            || old_ui_language != self.ui_language
            || old_always_on_top != self.always_on_top
            || old_enable_timer_limit != self.enable_timer_limit
            || old_timer_limit_minutes != self.timer_limit_minutes
            || old_target_duration_minutes != self.target_duration_minutes
            || old_window_opacity != self.window_opacity
            || old_countdown_duration_secs != self.countdown_duration_secs
            || old_enable_slide_alerts != self.enable_slide_alerts
            || old_transparent_background != self.transparent_background
            || old_mouse_passthrough != self.mouse_passthrough
            || old_text_align != self.text_align
            || old_enable_web_remote != self.enable_web_remote
        {
            self.save_settings();
        }
    }

    fn show_prompter_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx();
        let rect = ui.max_rect();
        let width = rect.width();
        let height = rect.height();
        let center_x = rect.center().x;
        
        // Listen to global inputs and consume remote mobile commands
        self.consume_web_remote_commands(ctx);

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
            self.fullscreen = false;
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
            self.target_scroll_y = (self.target_scroll_y - 150.0).max(0.0);
            self.record_action();
            ctx.request_repaint();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::PageDown)) {
            self.target_scroll_y = (self.target_scroll_y + 150.0).min(self.max_scroll);
            self.record_action();
            ctx.request_repaint();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::R)) {
            self.target_scroll_y = 0.0;
            self.scroll_y = 0.0;
            self.is_playing = false;
            self.countdown_secs = 0.0;
            self.elapsed_secs = 0.0;
            self.remaining_limit_secs = self.timer_limit_minutes * 60.0;
            self.active_slide_alert = None;
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
        // F11 and F key toggles borderless fullscreen in prompter mode
        if ctx.input(|i| i.key_pressed(egui::Key::F11) || i.key_pressed(egui::Key::F)) {
            self.fullscreen = !self.fullscreen;
            self.record_action();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::L)) {
            self.language_filter = match self.language_filter {
                LanguageFilter::All => LanguageFilter::ChineseOnly,
                LanguageFilter::ChineseOnly => LanguageFilter::EnglishOnly,
                LanguageFilter::EnglishOnly => KEY_LANG_FILTER_DEFAULT_STATE,
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
                // Scroll text manually when paused (adjusts the target Y for inertia)
                self.target_scroll_y = (self.target_scroll_y - scroll_delta.y * 1.5).clamp(0.0, self.max_scroll);
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
        let mut temp_sections: Vec<SectionInfo> = Vec::new();
        
        // Tracks stateful language blocks in parsing
        let mut current_block_lang = LanguageFilter::All;
        let bold_highlight_color = egui::Color32::from_rgb(255, 179, 0); // Amber/orange for bold text emphasis
        
        let mut slide_alert_to_trigger = None;

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
                temp_sections.push(SectionInfo {
                    name: clean_name,
                    offset_y: current_y,
                    speed: None,
                    cues: Vec::new(),
                });
                self.font_size * 0.8
            } else if is_meta {
                self.font_size * 0.2
            } else {
                0.0
            };

            // Parse Section Speed, Cue, and Slide tags
            let is_tag = trimmed.starts_with("[") && trimmed.ends_with("]");
            let mut is_speed_tag = false;
            let mut is_cue_tag = false;
            let mut is_slide_tag = false;

            if is_tag {
                let inner = &trimmed[1..trimmed.len() - 1];
                if inner.starts_with("speed:") {
                    if let Ok(parsed_speed) = inner["speed:".len()..].trim().parse::<f32>() {
                        is_speed_tag = true;
                        if let Some(current_sec) = temp_sections.last_mut() {
                            current_sec.speed = Some(parsed_speed);
                        }
                    }
                } else if inner.starts_with("cue:") {
                    is_cue_tag = true;
                    let cue_text = inner["cue:".len()..].trim().to_string();
                    if let Some(current_sec) = temp_sections.last_mut() {
                        current_sec.cues.push(cue_text);
                    }
                } else if inner.starts_with("slide:") {
                    is_slide_tag = true;
                    let slide_num = inner["slide:".len()..].trim().to_string();
                    
                    // We check if this slide tag is near the reading guide line (active line trigger)
                    let tag_draw_y = start_y + current_y - self.scroll_y;
                    if (tag_draw_y - guide_y).abs() < 80.0 {
                        slide_alert_to_trigger = Some(slide_num);
                    }
                }
            }

            // Do not render speed, cue, or slide tags in the scrolling viewport
            if is_speed_tag || is_cue_tag || is_slide_tag {
                continue;
            }

            // Layout the line/paragraph using the correct format, bold parser, and alignment
            let mut job = parse_formatted_line(
                trimmed,
                self.font_size,
                self.text_color,
                bold_highlight_color,
                wrapping_width,
                is_header,
                is_meta,
                self.text_align.to_egui_align(),
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

        // Assign current active slide flip alert badge
        self.active_slide_alert = slide_alert_to_trigger;

        // Calculate active section index
        let mut active_sec_idx = None;
        for (idx, sec) in self.sections.iter().enumerate() {
            if self.scroll_y >= sec.offset_y - 30.0 {
                active_sec_idx = Some(idx);
            }
        }

        // Trigger transition-based speed adjustments
        if active_sec_idx != self.last_active_sec_idx {
            self.last_active_sec_idx = active_sec_idx;
            if let Some(idx) = active_sec_idx {
                if let Some(section_speed) = self.sections[idx].speed {
                    self.scroll_speed = section_speed;
                }
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
        
        // Calculate Pace Estimator values
        let total_chars = self.text.chars().count();
        let cpm = if self.max_scroll > 0.0 && total_chars > 0 {
            let chars_per_pixel = total_chars as f32 / self.max_scroll;
            let chars_per_sec = self.scroll_speed * chars_per_pixel;
            (chars_per_sec * 60.0) as i32
        } else {
            0
        };
        let wpm = cpm / 5;

        if self.show_hud && has_room_for_hud && self.countdown_secs <= 0.0 {
            let hud_text_color = if self.bg_color == egui::Color32::WHITE && !self.transparent_background {
                egui::Color32::DARK_GRAY
            } else {
                egui::Color32::from_rgb(176, 190, 197) // Clean secondary grey
            };
            let cyan_accent = egui::Color32::from_rgb(0, 188, 212);

            // LEFT PANEL: Dynamic Timer & Mini Table of Contents
            let left_panel_x = rect.min.x + 20.0;
            
            // Build Timer String (either Countdown or Stopwatch)
            let timer_str = if self.enable_timer_limit {
                let limit_mins = (self.remaining_limit_secs / 60.0) as i32;
                let limit_secs = (self.remaining_limit_secs % 60.0) as i32;
                if self.remaining_limit_secs <= 0.0 {
                    self.tr("timer_ended").to_string()
                } else {
                    format!("⏱  {:02}:{:02}", limit_mins, limit_secs)
                }
            } else {
                let mins = (self.elapsed_secs / 60.0) as i32;
                let secs = (self.elapsed_secs % 60.0) as i32;
                format!("⏱  {:02}:{:02}", mins, secs)
            };

            // Dynamic color for timer based on urgency
            let timer_color = if self.enable_timer_limit {
                if self.remaining_limit_secs == 0.0 {
                    // Flash red
                    if (self.elapsed_secs * 2.0) as i32 % 2 == 0 { egui::Color32::RED } else { hud_text_color }
                } else if self.remaining_limit_secs <= 30.0 {
                    egui::Color32::from_rgb(244, 67, 54) // Bright red
                } else if self.remaining_limit_secs <= 60.0 {
                    egui::Color32::from_rgb(255, 152, 0) // Amber warning
                } else {
                    cyan_accent
                }
            } else {
                cyan_accent
            };

            let font_timer = egui::FontId::new(22.0, egui::FontFamily::Proportional);
            let timer_galley = ui.fonts(|f| f.layout(timer_str, font_timer, timer_color, f32::INFINITY));
            self.paint_shape(
                ctx,
                ui.painter(),
                ui.clip_rect(),
                egui::Shape::galley(egui::Pos2::new(left_panel_x, rect.min.y + 50.0), timer_galley, timer_color),
                center_x,
            );

            // Draw Section List (ToC)
            let toc_start_y = rect.min.y + 100.0;
            let step_y = 24.0;
            let max_visible_sections = ((height - 180.0) / step_y) as usize;

            for (idx, sec) in self.sections.iter().enumerate().take(max_visible_sections) {
                let is_active = Some(idx) == active_sec_idx;
                let color = if is_active { cyan_accent } else { hud_text_color };
                let prefix = if is_active { "● " } else { "○ " };
                
                // Truncate name if it exceeds the margin width safely
                let max_chars = ((padding - 40.0) / 10.0).max(10.0) as usize;
                let truncated_name: String = sec.name.chars().take(max_chars).collect();
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

            // RIGHT PANEL: Dynamic Pace Estimator & Next Up Preview & Real Clock Time
            let right_panel_x = rect.max.x - padding + 25.0;
            let right_panel_y = rect.min.y + 50.0;

            // 1. Pace Estimator (CPM / WPM)
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
            let mut preview_height = 0.0;
            if let Some(active_idx) = active_sec_idx {
                if active_idx + 1 < self.sections.len() {
                    let next_name = &self.sections[active_idx + 1].name;
                    
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
                    preview_height = 55.0;
                }
            }

            // 3. Real local clock time
            let clock_str = format!("🕒  {}", chrono::Local::now().format("%H:%M:%S"));
            let font_clock = egui::FontId::new(13.0, egui::FontFamily::Proportional);
            let clock_galley = ui.fonts(|f| f.layout(clock_str, font_clock, hud_text_color, padding - 45.0));
            self.paint_shape(
                ctx,
                ui.painter(),
                ui.clip_rect(),
                egui::Shape::galley(egui::Pos2::new(right_panel_x, right_panel_y + 60.0 + preview_height + 25.0), clock_galley, hud_text_color),
                center_x,
            );
        }

        // Draw Presenter Cue (v1.0.9 Pop-Up Card)
        if let Some(active_idx) = active_sec_idx {
            if active_idx < self.sections.len() {
                let active_cues = &self.sections[active_idx].cues;
                if !active_cues.is_empty() {
                    let cue_text = active_cues.join(" | ");
                    let overlay_bg = egui::Color32::from_rgba_unmultiplied(0, 151, 167, 180); // Cyan overlay
                    let tr_cue_title = self.tr("cue_title");
                    let display_str = format!("{} {}", tr_cue_title, cue_text);
                    
                    let font_cue = egui::FontId::new(14.0, egui::FontFamily::Proportional);
                    let job = parse_formatted_line(
                        &display_str,
                        font_cue.size,
                        egui::Color32::WHITE,
                        egui::Color32::from_rgb(255, 235, 59),
                        width - 300.0,
                        false,
                        false,
                        egui::Align::Center,
                    );
                    let cue_galley = ui.fonts(|f| f.layout_job(job));
                    let cue_width = cue_galley.rect.width();
                    
                    let x_pos = center_x - (cue_width / 2.0);
                    let y_pos = rect.min.y + 40.0;
                    
                    // Draw a subtle border backing card
                    let card_rect = egui::Rect::from_min_max(
                        egui::Pos2::new(x_pos - 15.0, y_pos - 6.0),
                        egui::Pos2::new(x_pos + cue_width + 15.0, y_pos + cue_galley.rect.height() + 6.0),
                    );
                    
                    self.paint_shape(
                        ctx,
                        ui.painter(),
                        ui.clip_rect(),
                        egui::Shape::rect_filled(card_rect, 6.0, overlay_bg),
                        center_x,
                    );
                    self.paint_shape(
                        ctx,
                        ui.painter(),
                        ui.clip_rect(),
                        egui::Shape::galley(egui::Pos2::new(x_pos, y_pos), cue_galley, egui::Color32::WHITE),
                        center_x,
                    );
                }
            }
        }

        // Draw Flashing Slide Flip Alert Badge (v1.1.0)
        if self.enable_slide_alerts {
            if let Some(ref slide_info) = self.active_slide_alert {
                // Flash alert (alternate opacity using elapsed time sine wave)
                let elapsed_factor = (self.elapsed_secs * 6.0).sin().abs();
                let alpha = (120.0 + elapsed_factor * 110.0) as u8; // Pulsing opacity [120, 230]
                
                let card_color = egui::Color32::from_rgba_unmultiplied(229, 57, 53, alpha); // Pulsing Red Card
                let text_banner = self.tr("slide_alert_banner");
                let alert_str = format!("🚨 {} {} 🚨", text_banner, slide_info);
                
                let font_alert = egui::FontId::new(20.0, egui::FontFamily::Proportional);
                let alert_galley = ui.fonts(|f| f.layout(alert_str, font_alert, egui::Color32::WHITE, width - 200.0));
                let alert_width = alert_galley.rect.width();
                
                let x_pos = center_x - (alert_width / 2.0);
                let y_pos = rect.max.y - 100.0; // Positioned near bottom center
                
                let alert_rect = egui::Rect::from_min_max(
                    egui::Pos2::new(x_pos - 20.0, y_pos - 8.0),
                    egui::Pos2::new(x_pos + alert_width + 20.0, y_pos + alert_galley.rect.height() + 8.0),
                );
                
                self.paint_shape(
                    ctx,
                    ui.painter(),
                    ui.clip_rect(),
                    egui::Shape::rect_filled(alert_rect, 8.0, card_color),
                    center_x,
                );
                self.paint_shape(
                    ctx,
                    ui.painter(),
                    ui.clip_rect(),
                    egui::Shape::galley(egui::Pos2::new(x_pos, y_pos), alert_galley, egui::Color32::WHITE),
                    center_x,
                );
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

        // 7. Control overlay (disappears after 2s of playing with no actions, showing mouse passthrough tip if active)
        let show_overlay = !self.is_playing || Instant::now().duration_since(self.last_action_time).as_secs_f32() < 2.0;
        if show_overlay && self.countdown_secs <= 0.0 {
            let overlay_bg = egui::Color32::from_rgba_unmultiplied(33, 33, 33, 200);
            let text_color = egui::Color32::WHITE;
            
            // Estimate Remaining Time
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
                            ui.vertical(|ui| {
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
                                // Display helper tip when mouse click-through is enabled
                                if self.mouse_passthrough {
                                    ui.add_space(4.0);
                                    let tr_tip = self.tr("sc_passthrough_tip");
                                    ui.label(egui::RichText::new(tr_tip).size(11.0).color(egui::Color32::from_rgb(0, 188, 212)));
                                }
                            });
                        });
                });
        }
    }
}

// HTTP Connection Client Handler
fn handle_client(mut stream: TcpStream, state: Arc<Mutex<SharedRemoteState>>) {
    let mut buffer = [0; 2048];
    if stream.read(&mut buffer).is_err() {
        return;
    }
    let req = String::from_utf8_lossy(&buffer);

    // Endpoint 1: GET /api/status -> return JSON string representing status
    if req.starts_with("GET /api/status") {
        let json = {
            let s = state.lock().unwrap();
            let sections_json = s.sections.iter()
                .map(|name| format!("\"{}\"", name.replace('"', "\\\"")))
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "{{\"is_playing\":{},\"scroll_speed\":{:.1},\"scroll_y\":{:.1},\"max_scroll\":{:.1},\"elapsed_secs\":{:.1},\"remaining_secs\":{:.1},\"active_section_idx\":{},\"sections\":[{}]}}",
                s.is_playing,
                s.scroll_speed,
                s.scroll_y,
                s.max_scroll,
                s.elapsed_secs,
                s.remaining_secs,
                match s.active_section_idx {
                    Some(idx) => idx.to_string(),
                    None => "null".to_string(),
                },
                sections_json
            )
        };
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
            json.len(),
            json
        );
        let _ = stream.write_all(response.as_bytes());
    } 
    // Endpoint 2: GET /api/control?action=xxx -> push command action
    else if req.starts_with("GET /api/control") {
        let action = if let Some(pos) = req.find("action=") {
            let start = pos + "action=".len();
            let end = req[start..].find(' ').unwrap_or(req[start..].len());
            req[start..start+end].trim().to_string()
        } else {
            "none".to_string()
        };

        if action != "none" {
            let mut s = state.lock().unwrap();
            s.command_queue.push(action);
        }

        let json = "{\"success\":true}";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
            json.len(),
            json
        );
        let _ = stream.write_all(response.as_bytes());
    } 
    // Endpoint 3: GET / or index.html -> serve HTML remote controller panel
    else if req.starts_with("GET / ") || req.starts_with("GET /index.html") {
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
            REMOTE_HTML.len(),
            REMOTE_HTML
        );
        let _ = stream.write_all(response.as_bytes());
    } else {
        // Fallback HTTP 404
        let response = "HTTP/1.1 404 NOT FOUND\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";
        let _ = stream.write_all(response.as_bytes());
    }
}

// Global default Language Filter state (v1.1.3 key mapping fix)
const KEY_LANG_FILTER_DEFAULT_STATE: LanguageFilter = LanguageFilter::All;

// Embedded mobile remote controller HTML page (Tailwind CSS based, fully responsive dark theme)
const REMOTE_HTML: &str = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
    <title>Sisyphus Remote Control</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <style>
        .active-btn { background-color: #00838F; border-color: #00acc1; }
        .glass-panel { background: rgba(30, 41, 59, 0.7); backdrop-filter: blur(12px); border: 1px solid rgba(255, 255, 255, 0.08); }
    </style>
</head>
<body class="bg-slate-950 text-slate-100 font-sans flex flex-col min-h-screen select-none">
    <!-- Header -->
    <header class="w-full py-4 px-6 glass-panel flex items-center justify-between sticky top-0 z-50">
        <h1 class="text-lg font-bold tracking-wider text-cyan-400">🚀 Sisyphus Remote</h1>
        <div id="status-badge" class="px-3 py-1 rounded-full text-xs font-semibold bg-gray-700 text-gray-300">DISCONNECTED</div>
    </header>

    <!-- Main Content Grid -->
    <main class="flex-grow w-full max-w-md mx-auto p-4 space-y-4 flex flex-col justify-center">
        <!-- Timer & Speed Display Card -->
        <div class="glass-panel rounded-2xl p-5 grid grid-cols-2 gap-4 text-center shadow-lg">
            <div class="border-r border-slate-800">
                <p class="text-xs text-slate-400 uppercase tracking-widest mb-1">Timer</p>
                <p id="timer" class="text-2xl font-bold text-cyan-300">00:00</p>
            </div>
            <div>
                <p class="text-xs text-slate-400 uppercase tracking-widest mb-1">Speed</p>
                <p id="speed" class="text-2xl font-bold text-cyan-300">-- px/s</p>
            </div>
        </div>

        <!-- Major Play/Pause Panel -->
        <div class="flex justify-center py-4">
            <button id="btn-toggle" class="w-40 h-40 rounded-full flex flex-col items-center justify-center font-bold text-lg border-4 border-slate-700 bg-slate-800 text-slate-200 transition-all active:scale-95 shadow-xl">
                <span id="play-icon" class="text-4xl mb-1">▶</span>
                <span id="play-text">PLAY</span>
            </button>
        </div>

        <!-- Micro Adjustments Control Grid -->
        <div class="grid grid-cols-2 gap-4">
            <button onclick="control('speed_up')" class="glass-panel py-4 rounded-xl text-center font-semibold active:bg-cyan-900 active:scale-95 transition-all shadow-md">
                ➕ Speed Up
            </button>
            <button onclick="control('speed_down')" class="glass-panel py-4 rounded-xl text-center font-semibold active:bg-cyan-900 active:scale-95 transition-all shadow-md">
                ➖ Speed Down
            </button>
            <button onclick="control('page_up')" class="glass-panel py-4 rounded-xl text-center font-semibold active:bg-cyan-900 active:scale-95 transition-all shadow-md">
                ⏮ Scroll Up
            </button>
            <button onclick="control('page_down')" class="glass-panel py-4 rounded-xl text-center font-semibold active:bg-cyan-900 active:scale-95 transition-all shadow-md">
                ⏭ Scroll Down
            </button>
        </div>

        <button onclick="control('reset')" class="w-full py-4 bg-red-950 border border-red-800 rounded-xl font-bold text-red-200 active:bg-red-900 active:scale-95 transition-all shadow-lg">
            🔄 Reset Prompter & Timer
        </button>

        <!-- Chapter Jump List Card -->
        <div class="glass-panel rounded-2xl p-5 space-y-3 flex-grow">
            <h2 class="text-sm font-semibold uppercase tracking-wider text-slate-400">Chapters & Sections</h2>
            <div id="section-list" class="space-y-2 max-h-48 overflow-y-auto pr-1">
                <!-- Spanned dynamically -->
            </div>
        </div>
    </main>

    <!-- Script JS -->
    <script>
        const btnToggle = document.getElementById('btn-toggle');
        const playIcon = document.getElementById('play-icon');
        const playText = document.getElementById('play-text');
        const statusBadge = document.getElementById('status-badge');
        const txtTimer = document.getElementById('timer');
        const txtSpeed = document.getElementById('speed');
        const listSection = document.getElementById('section-list');

        let sections = [];
        let activeIdx = null;

        btnToggle.addEventListener('click', () => {
            control('toggle');
        });

        function control(action) {
            fetch(`/api/control?action=${action}`)
                .then(res => res.json())
                .catch(err => console.error(err));
        }

        function poll() {
            fetch('/api/status')
                .then(res => res.json())
                .then(data => {
                    // Update connection state
                    statusBadge.innerText = data.is_playing ? 'PLAYING' : 'PAUSED';
                    statusBadge.className = `px-3 py-1 rounded-full text-xs font-semibold ${data.is_playing ? 'bg-emerald-950 text-emerald-300 border border-emerald-800' : 'bg-amber-950 text-amber-300 border border-amber-800'}`;

                    // Update toggler button state
                    if (data.is_playing) {
                        playIcon.innerText = '⏸';
                        playText.innerText = 'PAUSE';
                        btnToggle.className = "w-40 h-40 rounded-full flex flex-col items-center justify-center font-bold text-lg border-4 border-emerald-500 bg-emerald-900 text-white transition-all active:scale-95 shadow-xl";
                    } else {
                        playIcon.innerText = '▶';
                        playText.innerText = 'PLAY';
                        btnToggle.className = "w-40 h-40 rounded-full flex flex-col items-center justify-center font-bold text-lg border-4 border-slate-700 bg-slate-800 text-slate-200 transition-all active:scale-95 shadow-xl";
                    }

                    // Update speed and timer
                    txtSpeed.innerText = `${data.scroll_speed} px/s`;
                    
                    const m = Math.floor(data.remaining_secs / 60);
                    const s = Math.floor(data.remaining_secs % 60);
                    txtTimer.innerText = `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;

                    // Populate and update chapter list if changed
                    if (JSON.stringify(sections) !== JSON.stringify(data.sections) || activeIdx !== data.active_section_idx) {
                        sections = data.sections;
                        activeIdx = data.active_section_idx;
                        renderSections();
                    }
                })
                .catch(() => {
                    statusBadge.innerText = 'DISCONNECTED';
                    statusBadge.className = 'px-3 py-1 rounded-full text-xs font-semibold bg-red-950 text-red-300 border border-red-800';
                });
        }

        function renderSections() {
            listSection.innerHTML = '';
            sections.forEach((name, idx) => {
                const isActive = idx === activeIdx;
                const div = document.createElement('div');
                div.className = `w-full py-3 px-4 rounded-xl flex items-center justify-between transition-all active:scale-[0.98] ${isActive ? 'bg-cyan-950 border border-cyan-700 text-white font-semibold' : 'bg-slate-900 border border-slate-800 text-slate-300'}`;
                div.innerHTML = `<span>${name}</span><span>${isActive ? '👉' : ''}</span>`;
                div.onclick = () => control(`jump:${idx}`);
                listSection.appendChild(div);
            });
        }

        // Poll every 400ms
        setInterval(poll, 400);
        poll();
    </script>
</body>
</html>
"#;

const DEFAULT_TEXT: &str = r#"=== PAGE 1: Opening & Hardware Product Definition ===
[speed: 65]
[slide: PAGE 1]
[cue: 动作：微笑致意，保持眼神交流]
[中文]
各位评委老师，早上好。我是陶鑫旺。正如我作品集第一页所示，我将自己定位为**机器人系统研发工程师**与**工业AI全栈实践者**。我始终致力于打破代码与物理世界之间的边界。

我的第一个核心成果在于**硬件产品定义**。在李泽湘教授指导的InnoX创业营期间——您可以在图1中看到我们的路演合影——我主导了智能硬件Lumina从0到1的开发。针对年轻人因高功能焦虑引起的‘决策瘫痪’，我们创新地设计了**‘配重转子动态阻尼扭矩反馈’**的物理机制，将心理学机制具象化地实现在物理世界中。

[English]
Good morning, respected judges. I am Xinwang Tao. As shown on the first page of my portfolio, I position myself as a **Robotics System R&D Engineer** and an **Industrial AI Full-Stack Practitioner**. I am always driven to break the boundaries between code and the physical world.

My first core achievement lies in **hardware product definition**. During the InnoX Camp mentored by Prof. Zexiang Li—you can see our roadshow group photo in Fig. 1—I led the 0-to-1 development of the smart hardware, Lumina. Targeting the 'decision paralysis' caused by high-functioning anxiety among young people, we innovatively designed a physical mechanism with **'weighted rotor dynamic damping torque feedback,'** reifying psychological mechanisms into the physical world.

=== PAGE 2: Commercial Updates & Achievement 2: Industrial AI ===
[speed: 70]
[slide: PAGE 2]
[cue: 动作：指向图2 Demo，展现商业野心]
[中文]
请翻到第2页。在产品落地阶段，如顶部图2所示，通过持续、高强度的MVP快速迭代，我们成功打磨出了一款高度精致的产品Demo。

**更令人兴奋的是，该项目目前正加速迈向深度商业化。我们团队最近迎来了两位实力强劲的新伙伴，目前正全力进军国际市场流量。我们正积极建立海外种子用户社区，并筹备在Kickstarter上发起众筹活动。** 这一从产品定义、Demo实现到全球化拓展的完整历程，使我荣获了‘优秀产品经理’称号。

除了敏捷硬件开发，我的第二个核心成果是**前线工业级AI的全栈部署**。请看中间的图3；这是我构建的工业AI Agent决策流架构。面对复杂、非标的SMT车间，我开发了一套高效的视频处理管线，清洗了1289个异常视频片段，作为视觉大模型（VLM）微调的基础。基于LangGraph状态机和ReAct逻辑，该闭环系统现已部署于实际生产线，展现出替代约30%重度人工视觉检测岗位的巨大潜力。

[English]
Please turn to PAGE 2. In the product implementation phase, as shown in Fig. 2 at the top, through continuous, high-intensity MVP rapid iterations, we have successfully polished a highly refined product Demo.

**Even more excitingly, this project is now accelerating into deep commercialization. We recently onboarded two strong partners to our team and are now fully targeting international market traffic. We are actively building our overseas seed user community and preparing to launch a crowdfunding campaign on Kickstarter.** This complete journey from product definition and Demo realization to global expansion earned me the 'Outstanding Product Manager' title.

Beyond agile hardware development, my second core achievement is the **full-stack deployment of frontline industrial AI**. Please look at Fig. 3 in the middle; this is the industrial AI Agent decision-making flow architecture I built. Facing the complex, non-standard SMT workshops, I developed a highly efficient video processing pipeline, cleaning 1,289 error-related video segments as a Vision Large Model (VLM) fine-tuning foundation. Based on the LangGraph state machine and ReAct logic, this closed-loop system is now deployed on actual production lines, demonstrating the immense potential to replace approximately 30% of heavy manual visual inspection roles.

=== PAGE 3: Achievement 3: Systems Engineering & Commercialization ===
[speed: 60]
[slide: PAGE 3]
[cue: 动作：加重语气，介绍云影智巡CTO成果]
[中文]
请翻到第3页。我的第三个核心成果是大型系统的商业化运营。作为科技初创企业‘云影智巡’的CTO，我全面领导了机器人技术（包括教育无人机和仿生扑翼微型飞行器）的全栈研发。

图4在左侧展示了我们多代仿生扑翼无人机硬核的迭代过程。图5在右侧梳理了支持该产品矩阵的跨学科技术栈。基于ArduPilot和PX4等底层架构，我们成功攻克了罗盘校准和复杂PID调参等工程难题。

[English]
Please turn to PAGE 3. My third core achievement is the commercial operation of large-scale systems. As the CTO of the tech startup 'Yunying Zhixun', I comprehensively led the full-stack R&D of robotics, including educational drones and bionic flapping-wing UAVs.

Fig. 4 on the left displays the hardcore iterative process of our multi-generation bionic flapping-wing UAVs. Fig. 5 on the right maps out the cross-disciplinary tech stack supporting this product matrix. Based on underlying architectures like ArduPilot and PX4, we successfully resolved engineering challenges such as compass calibration and complex PID tuning.

=== PAGE 4: Commercial Validation & IP Moat ===
[speed: 65]
[slide: PAGE 4]
[cue: 动作：展示互联网+银奖与股交所挂牌证书]
[中文]
第4页展示了我们的商业验证与技术护城河。图6来自于我们在‘互联网+’大赛中荣获国家级银奖的颁奖现场。图7是我们公司在湖南股权交易所成功挂牌的证书。

为了构建技术壁垒，作为核心发明人，我累计获得了**10余项发明专利与软件著作权**。此外，底部的图9证明了我已获得鸿蒙（HarmonyOS）高级应用开发者认证，展现出全面的生态构建能力。

[English]
PAGE 4 showcases our commercial validation and technical moats. Fig. 6 is from the site where we won the National Silver Award in the 'Internet+' Competition. Fig. 7 is the certificate of our company's successful listing on the Hunan Equity Exchange.

To build technical barriers, as shown in Fig. 8 in the middle, I accumulated **over 10 invention patents and software copyrights** as the core inventor. Additionally, Fig. 9 at the bottom proves that I obtained the HarmonyOS Advanced Application Developer Certification, demonstrating a comprehensive ecosystem-building capability.

=== PAGE 5: Achievement 4: Academic Research & Theory ===
[speed: 55]
[slide: PAGE 5]
[cue: 动作：指向学术论文，语调保持沉稳、严谨]
[中文]
工程痛点反向驱动了我对学术界的深耕。请看包含我第四个成果的第5页。

左下角的图12是我在JCR Q1区期刊《Biomimetics》上发表的仿生无人机空间连杆机构研究，在此项研究中，我们克服了复杂湍流下的控制难题。右下角的图13是发表在自动驾驶顶级期刊《IEEE TVT》上的预测局部多重注意力（PLMA）模型。作为第二作者，我独立设计了核心进化网络拓扑，并在真实数据集上完成了高精度的风险评估验证。

[English]
Engineering pain points have inversely driven my deep dive into academia. Please look at PAGE 5, covering my fourth achievement.

Fig. 12 on the bottom left is my research on a bionic UAV spatial linkage mechanism published in the JCR Q1 journal, Biomimetics, where we conquered control challenges under complex turbulence. Fig. 13 on the bottom right is the Predictive Local Multiple Attention (PLMA) model published in the top-tier autonomous driving journal, IEEE TVT. As the second author, I independently designed the core evolutionary network topology and completed high-precision risk assessment validation on real-world datasets.

=== PAGE 6: Full-Stack Skill Tree & Vision ===
[speed: 70]
[slide: PAGE 6]
[cue: 动作：眼神交流，自信有力地收尾]
[中文]
最后，请看第6页。顶部区域总结了我的全栈技能树。从熟练使用Cursor等AI工具构建自动化工作流，到将边缘计算底层彻底从C++重构为性能更高的Rust语言并部署在ESP32-S3微控制器上——我正不断且敏捷地迭代我的技术边界。

我坚信，顶尖的技术创新是**数字计算与人类感官体验的深度共鸣**。我非常渴望加入贵团队，与顶尖的跨学科头脑进行思想碰撞，共同定义机器人与自主系统（RAS）的下一个颠覆性范式！谢谢大家。

[English]
Finally, please look at PAGE 6. The top section summarizes my full-stack skill tree. From proficiently using AI tools like Cursor to build automated workflows, to completely refactoring the edge computing base from C++ to the higher-performance Rust language and deploying it on ESP32-S3 microcontrollers—I am constantly and agilely iterating my technical boundaries.

I firmly believe that top-tier technological innovation is a deep resonance between digital computing and human sensory experience. I am eager to join your team, brainstorm with top interdisciplinary minds, and co-define the next disruptive paradigm in Robotics and Autonomous Systems (RAS)! Thank you.
"#;
