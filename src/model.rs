//! Project data model: canvas, scenes (shots) and white-model elements.
//!
//! The same structures are used for the editable project file and (with a few
//! derived fields added by [`crate::export`]) for the exported `layout.json`.

use serde::{Deserialize, Serialize};

/// An sRGB colour, serialised as `"#RRGGBB"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub [u8; 3]);

impl Rgb {
    pub const WHITE: Rgb = Rgb([255, 255, 255]);
    pub const BLACK: Rgb = Rgb([0, 0, 0]);
    pub const fn gray(v: u8) -> Rgb {
        Rgb([v, v, v])
    }
    pub fn hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.0[0], self.0[1], self.0[2])
    }
    pub fn parse(s: &str) -> Option<Rgb> {
        let s = s.trim().trim_start_matches('#');
        let v = |i: usize| u8::from_str_radix(s.get(i..i + 2)?, 16).ok();
        match s.len() {
            6 | 8 => Some(Rgb([v(0)?, v(2)?, v(4)?])),
            3 => {
                let c = |i: usize| u8::from_str_radix(&s[i..i + 1], 16).ok().map(|x| x * 17);
                Some(Rgb([c(0)?, c(1)?, c(2)?]))
            }
            _ => None,
        }
    }
    /// Relative luminance in 0..1 (rough, for picking contrasting label colours).
    pub fn luma(&self) -> f32 {
        (0.299 * self.0[0] as f32 + 0.587 * self.0[1] as f32 + 0.114 * self.0[2] as f32) / 255.0
    }
}

impl Serialize for Rgb {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.hex())
    }
}

impl<'de> Deserialize<'de> for Rgb {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Rgb::parse(&s).ok_or_else(|| serde::de::Error::custom(format!("invalid colour {s:?}, expected #RRGGBB")))
    }
}

/// All component types. Adding a new one: add a variant here, give it a row in
/// [`ElementKind::ALL`] / [`ElementKind::info`] and defaults in [`Element::new`];
/// drawing ([`crate::draw`]) and the MoviePy script fall back to a labelled box.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElementKind {
    Background,
    Title,
    Subheading,
    Subtitle,
    TextCard,
    Options,
    LowerThird,
    MediaPlaceholder,
    Logo,
    AvatarFrame,
    ProgressBar,
    Countdown,
    Watermark,
    CtaButton,
    CalloutArrow,
    Shape,
    QrCode,
    Sticker,
}

pub struct KindInfo {
    /// Traditional Chinese label shown in the palette and on the white model.
    pub zh: &'static str,
    /// English label.
    pub en: &'static str,
    /// Single glyph icon for the palette.
    pub icon: &'static str,
    /// Palette group.
    pub group: &'static str,
    /// Short MoviePy mapping hint (also written to layout.json).
    pub moviepy: &'static str,
}

impl ElementKind {
    pub const ALL: [ElementKind; 18] = [
        ElementKind::Title,
        ElementKind::Subheading,
        ElementKind::Subtitle,
        ElementKind::TextCard,
        ElementKind::Options,
        ElementKind::LowerThird,
        ElementKind::CtaButton,
        ElementKind::Watermark,
        ElementKind::MediaPlaceholder,
        ElementKind::Logo,
        ElementKind::AvatarFrame,
        ElementKind::QrCode,
        ElementKind::Sticker,
        ElementKind::ProgressBar,
        ElementKind::Countdown,
        ElementKind::CalloutArrow,
        ElementKind::Shape,
        ElementKind::Background,
    ];

    pub fn info(self) -> KindInfo {
        use ElementKind::*;
        let (zh, en, icon, group, moviepy) = match self {
            Background => ("背景", "Background", "▦", "版面", "ColorClip(size=canvas, color=bg_color)"),
            Title => ("標題", "Title", "T", "文字", "TextClip(method='caption') over optional ColorClip"),
            Subheading => ("副標題", "Subheading", "t", "文字", "TextClip(method='caption') over optional ColorClip"),
            Subtitle => ("字幕", "Subtitle", "≡", "文字", "TextClip with stroke, bottom-centred, timed per line"),
            TextCard => ("字卡", "Text card", "▤", "文字", "ColorClip card + TextClip, CompositeVideoClip"),
            Options => ("選項", "Options", "☰", "互動", "one ColorClip+TextClip row per option (A/B/C/D)"),
            LowerThird => {
                ("下三分之一名牌", "Lower third", "▁", "文字", "ColorClip bar + accent + 2-line TextClip, slide in")
            }
            MediaPlaceholder => {
                ("圖片/影片佔位", "Media placeholder", "▶", "媒體", "ImageClip / VideoFileClip(src).resized((w,h))")
            }
            Logo => ("Logo", "Logo", "◆", "媒體", "ImageClip(src).resized(height=h)"),
            AvatarFrame => {
                ("頭像/人物框", "Avatar / presenter", "☺", "媒體", "ImageClip/VideoFileClip(src) with circular mask")
            }
            ProgressBar => (
                "進度條",
                "Progress bar",
                "━",
                "互動",
                "VideoClip(frame_function) filling 0→100% over the element time",
            ),
            Countdown => ("倒數計時", "Countdown", "⏱", "互動", "one TextClip per second, counting down to 0"),
            Watermark => {
                ("浮水印", "Watermark", "©", "版面", "semi-transparent TextClip/ImageClip for the whole scene")
            }
            CtaButton => {
                ("CTA 按鈕", "CTA button", "☞", "互動", "rounded ColorClip/ImageClip + TextClip, pop animation")
            }
            CalloutArrow => {
                ("箭頭/標註", "Callout arrow", "→", "版面", "RGBA ImageClip of an arrow (PIL) + TextClip label")
            }
            Shape => ("形狀", "Shape", "■", "版面", "ColorClip (rect) or RGBA ImageClip (circle/rounded)"),
            QrCode => ("QR Code", "QR code", "▣", "媒體", "ImageClip(qr.png) – generate with the `qrcode` package"),
            Sticker => {
                ("貼圖/圖示", "Emoji / icon sticker", "★", "媒體", "ImageClip(icon.png) or TextClip(emoji font)")
            }
        };
        KindInfo { zh, en, icon, group, moviepy }
    }

    pub fn key(self) -> &'static str {
        use ElementKind::*;
        match self {
            Background => "background",
            Title => "title",
            Subheading => "subheading",
            Subtitle => "subtitle",
            TextCard => "text_card",
            Options => "options",
            LowerThird => "lower_third",
            MediaPlaceholder => "media_placeholder",
            Logo => "logo",
            AvatarFrame => "avatar_frame",
            ProgressBar => "progress_bar",
            Countdown => "countdown",
            Watermark => "watermark",
            CtaButton => "cta_button",
            CalloutArrow => "callout_arrow",
            Shape => "shape",
            QrCode => "qr_code",
            Sticker => "sticker",
        }
    }

    /// Whether the element's `text` is rendered as the main content.
    pub fn has_text(self) -> bool {
        !matches!(self, ElementKind::Background | ElementKind::ProgressBar)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Align {
    Left,
    #[default]
    Center,
    Right,
}

impl Align {
    pub const ALL: [Align; 3] = [Align::Left, Align::Center, Align::Right];
    pub fn zh(self) -> &'static str {
        match self {
            Align::Left => "靠左",
            Align::Center => "置中",
            Align::Right => "靠右",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Animation {
    #[default]
    None,
    FadeIn,
    FadeOut,
    FadeInOut,
    SlideUp,
    SlideLeft,
    Pop,
    Typewriter,
}

impl Animation {
    pub const ALL: [Animation; 8] = [
        Animation::None,
        Animation::FadeIn,
        Animation::FadeOut,
        Animation::FadeInOut,
        Animation::SlideUp,
        Animation::SlideLeft,
        Animation::Pop,
        Animation::Typewriter,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Animation::None => "none",
            Animation::FadeIn => "fade_in",
            Animation::FadeOut => "fade_out",
            Animation::FadeInOut => "fade_in_out",
            Animation::SlideUp => "slide_up",
            Animation::SlideLeft => "slide_left",
            Animation::Pop => "pop",
            Animation::Typewriter => "typewriter",
        }
    }
    pub fn zh(self) -> &'static str {
        match self {
            Animation::None => "無",
            Animation::FadeIn => "淡入",
            Animation::FadeOut => "淡出",
            Animation::FadeInOut => "淡入淡出",
            Animation::SlideUp => "由下滑入",
            Animation::SlideLeft => "由右滑入",
            Animation::Pop => "彈出放大",
            Animation::Typewriter => "打字機",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShapeKind {
    #[default]
    Rect,
    Rounded,
    Circle,
}

impl ShapeKind {
    pub const ALL: [ShapeKind; 3] = [ShapeKind::Rect, ShapeKind::Rounded, ShapeKind::Circle];
    pub fn zh(self) -> &'static str {
        match self {
            ShapeKind::Rect => "矩形",
            ShapeKind::Rounded => "圓角矩形",
            ShapeKind::Circle => "圓形/橢圓",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Left,
    #[default]
    Right,
    Up,
    Down,
}

impl Direction {
    pub const ALL: [Direction; 4] = [Direction::Left, Direction::Right, Direction::Up, Direction::Down];
    pub fn zh(self) -> &'static str {
        match self {
            Direction::Left => "← 向左",
            Direction::Right => "→ 向右",
            Direction::Up => "↑ 向上",
            Direction::Down => "↓ 向下",
        }
    }
}

fn default_true() -> bool {
    true
}
fn default_opacity() -> f32 {
    1.0
}

/// One white-model component on the canvas. Coordinates are pixels in the
/// target video resolution, origin top-left, `x,y` = top-left corner.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Element {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(rename = "type")]
    pub kind: ElementKind,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    #[serde(default)]
    pub text: String,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    #[serde(default = "default_font_color")]
    pub font_color: Rgb,
    #[serde(default = "default_bg")]
    pub bg_color: Rgb,
    #[serde(default = "default_opacity")]
    pub bg_opacity: f32,
    #[serde(default)]
    pub align: Align,
    #[serde(default = "default_stroke")]
    pub stroke_color: Rgb,
    #[serde(default)]
    pub stroke_width: f32,
    /// Seconds from the start of the scene.
    #[serde(default)]
    pub start: f32,
    /// Seconds from the start of the scene; `null` = until the scene ends.
    #[serde(default)]
    pub end: Option<f32>,
    #[serde(default)]
    pub animation: Animation,
    /// Stacking order (0 = bottom). Kept equal to the element's index on save/export.
    #[serde(default)]
    pub z: i32,
    /// Option rows for `options` elements (A/B/C/D…).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
    /// Index of the highlighted / correct option (`options` only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<usize>,
    #[serde(default)]
    pub shape: ShapeKind,
    #[serde(default)]
    pub direction: Direction,
    /// Optional asset path (image / video / logo / icon) the agent should use.
    #[serde(default)]
    pub src: String,
    /// Free-form instructions for the agent.
    #[serde(default)]
    pub notes: String,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default)]
    pub locked: bool,
}

fn default_font_size() -> f32 {
    48.0
}
fn default_font_color() -> Rgb {
    Rgb::WHITE
}
fn default_bg() -> Rgb {
    Rgb::gray(160)
}
fn default_stroke() -> Rgb {
    Rgb::BLACK
}

impl Element {
    /// A new element of `kind` with sensible defaults for a `cw`×`ch` canvas, centred at (`cx`,`cy`).
    pub fn new(kind: ElementKind, id: String, cw: f32, ch: f32, cx: f32, cy: f32) -> Element {
        use ElementKind::*;
        let u = cw.min(ch) / 1080.0; // unit: 1 px at 1080p short side
        let (w, h, text, fs, fc, bg, op): (f32, f32, &str, f32, Rgb, Rgb, f32) = match kind {
            Background => (cw, ch, "背景", 40.0, Rgb::gray(90), Rgb::gray(225), 1.0),
            Title => (1100.0, 150.0, "主標題文字", 96.0, Rgb::gray(20), Rgb::gray(200), 0.0),
            Subheading => (900.0, 90.0, "副標題說明文字", 54.0, Rgb::gray(50), Rgb::gray(200), 0.0),
            Subtitle => (1400.0, 100.0, "這裡是字幕，一行約 18 字", 52.0, Rgb::WHITE, Rgb::BLACK, 0.45),
            TextCard => (900.0, 420.0, "字卡重點\n第二行說明", 60.0, Rgb::gray(30), Rgb::gray(245), 0.92),
            Options => (760.0, 420.0, "", 44.0, Rgb::gray(30), Rgb::gray(240), 0.95),
            LowerThird => (720.0, 140.0, "姓名 Name\n職稱 / 頻道", 44.0, Rgb::WHITE, Rgb::gray(40), 0.88),
            MediaPlaceholder => (800.0, 450.0, "圖片 / 影片", 44.0, Rgb::gray(70), Rgb::gray(185), 1.0),
            Logo => (220.0, 120.0, "LOGO", 40.0, Rgb::gray(70), Rgb::gray(205), 1.0),
            AvatarFrame => (320.0, 320.0, "主持人", 36.0, Rgb::gray(60), Rgb::gray(195), 1.0),
            ProgressBar => (1600.0, 24.0, "", 24.0, Rgb([250, 180, 40]), Rgb::gray(90), 0.8),
            Countdown => (180.0, 180.0, "10", 90.0, Rgb::gray(20), Rgb::gray(235), 0.95),
            Watermark => (360.0, 60.0, "@my_channel", 34.0, Rgb::gray(110), Rgb::gray(0), 0.0),
            CtaButton => (420.0, 110.0, "立即訂閱 ▶", 50.0, Rgb::WHITE, Rgb([220, 60, 60]), 1.0),
            CalloutArrow => (360.0, 140.0, "看這裡！", 40.0, Rgb::gray(20), Rgb([255, 210, 60]), 1.0),
            Shape => (300.0, 300.0, "", 36.0, Rgb::gray(60), Rgb::gray(175), 1.0),
            QrCode => (240.0, 240.0, "QR", 32.0, Rgb::gray(20), Rgb::WHITE, 1.0),
            Sticker => (160.0, 160.0, "★", 96.0, Rgb([250, 190, 30]), Rgb::gray(235), 0.0),
        };
        let (w, h) = if kind == Background { (w, h) } else { ((w * u).min(cw), (h * u).min(ch)) };
        let (x, y) = if kind == Background { (0.0, 0.0) } else { (cx - w / 2.0, cy - h / 2.0) };
        let mut e = Element {
            id,
            name: kind.info().zh.to_string(),
            kind,
            x: x.round(),
            y: y.round(),
            w: w.round(),
            h: h.round(),
            text: text.to_string(),
            font_size: (fs * u).round(),
            font_color: fc,
            bg_color: bg,
            bg_opacity: op,
            align: Align::Center,
            stroke_color: Rgb::BLACK,
            stroke_width: 0.0,
            start: 0.0,
            end: None,
            animation: Animation::None,
            z: 0,
            options: vec![],
            answer: None,
            shape: ShapeKind::Rect,
            direction: Direction::Right,
            src: String::new(),
            notes: String::new(),
            visible: true,
            locked: false,
        };
        match kind {
            Subtitle => e.stroke_width = (3.0 * u).round(),
            Options => {
                e.options = ["選項 A", "選項 B", "選項 C", "選項 D"].map(String::from).to_vec();
                e.align = Align::Left;
            }
            LowerThird => {
                e.align = Align::Left;
                e.animation = Animation::SlideLeft;
            }
            CtaButton => {
                e.shape = ShapeKind::Rounded;
                e.animation = Animation::Pop;
            }
            Watermark => e.align = Align::Right,
            AvatarFrame | Countdown => e.shape = ShapeKind::Circle,
            Title => e.animation = Animation::FadeIn,
            _ => {}
        }
        e
    }

    /// End time within the scene, resolved against the scene duration.
    pub fn end_in(&self, scene_duration: f32) -> f32 {
        self.end.unwrap_or(scene_duration).clamp(0.0, scene_duration.max(0.0))
    }

    pub fn display_name(&self) -> String {
        if self.name.is_empty() { self.id.clone() } else { self.name.clone() }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Scene {
    pub id: String,
    #[serde(default)]
    pub name: String,
    /// Seconds.
    pub duration: f32,
    #[serde(default = "default_scene_bg")]
    pub background: Rgb,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub elements: Vec<Element>,
}

fn default_scene_bg() -> Rgb {
    Rgb::gray(236)
}

impl Scene {
    pub fn new(id: String, name: String) -> Scene {
        Scene { id, name, duration: 5.0, background: default_scene_bg(), notes: String::new(), elements: vec![] }
    }
    pub fn find(&self, id: &str) -> Option<usize> {
        self.elements.iter().position(|e| e.id == id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AspectPreset {
    pub key: &'static str,
    pub label: &'static str,
    pub width: u32,
    pub height: u32,
}

pub const PRESETS: [AspectPreset; 5] = [
    AspectPreset { key: "16:9", label: "16:9 橫式 1920×1080", width: 1920, height: 1080 },
    AspectPreset { key: "9:16", label: "9:16 直式 1080×1920", width: 1080, height: 1920 },
    AspectPreset { key: "1:1", label: "1:1 方形 1080×1080", width: 1080, height: 1080 },
    AspectPreset { key: "4:5", label: "4:5 直式 1080×1350", width: 1080, height: 1350 },
    AspectPreset { key: "4:3", label: "4:3 1440×1080", width: 1440, height: 1080 },
];

pub fn preset(key: &str) -> Option<AspectPreset> {
    PRESETS.iter().copied().find(|p| p.key == key)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Canvas {
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub preset: String,
}

/// Safe-area margins as a fraction of each dimension (broadcast convention).
pub const ACTION_SAFE: f32 = 0.05;
pub const TITLE_SAFE: f32 = 0.10;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub name: String,
    pub canvas: Canvas,
    #[serde(default = "default_fps")]
    pub fps: u32,
    pub scenes: Vec<Scene>,
}

fn default_version() -> u32 {
    1
}
fn default_fps() -> u32 {
    30
}

impl Default for Project {
    fn default() -> Self {
        Project::new("未命名專案", "16:9")
    }
}

impl Project {
    pub fn new(name: &str, preset_key: &str) -> Project {
        let p = preset(preset_key).unwrap_or(PRESETS[0]);
        Project {
            version: 1,
            name: name.to_string(),
            canvas: Canvas { width: p.width, height: p.height, preset: p.key.to_string() },
            fps: 30,
            scenes: vec![Scene::new("scene_1".into(), "場景 1".into())],
        }
    }

    pub fn total_duration(&self) -> f32 {
        self.scenes.iter().map(|s| s.duration.max(0.0)).sum::<f32>() + 0.0
    }

    /// Absolute start time of scene `i`.
    pub fn scene_start(&self, i: usize) -> f32 {
        self.scenes.iter().take(i).map(|s| s.duration.max(0.0)).sum::<f32>() + 0.0
    }

    /// A project-wide unique element id like `title_3`.
    pub fn next_element_id(&self, kind: ElementKind) -> String {
        let base = kind.key();
        let mut n = 1;
        loop {
            let id = format!("{base}_{n}");
            if !self.scenes.iter().any(|s| s.elements.iter().any(|e| e.id == id)) {
                return id;
            }
            n += 1;
        }
    }

    pub fn next_scene_id(&self) -> String {
        let mut n = self.scenes.len() + 1;
        loop {
            let id = format!("scene_{n}");
            if !self.scenes.iter().any(|s| s.id == id) {
                return id;
            }
            n += 1;
        }
    }

    /// Re-number `z` to match the element order and fix invalid values.
    pub fn normalize(&mut self) {
        if self.fps == 0 {
            self.fps = 30;
        }
        self.canvas.width = self.canvas.width.clamp(16, 8192);
        self.canvas.height = self.canvas.height.clamp(16, 8192);
        if self.scenes.is_empty() {
            self.scenes.push(Scene::new("scene_1".into(), "場景 1".into()));
        }
        for s in &mut self.scenes {
            if s.duration.is_nan() || s.duration <= 0.0 {
                s.duration = 1.0;
            }
            for (i, e) in s.elements.iter_mut().enumerate() {
                e.z = i as i32;
                e.w = e.w.max(1.0);
                e.h = e.h.max(1.0);
                e.bg_opacity = e.bg_opacity.clamp(0.0, 1.0);
                e.start = e.start.max(0.0);
            }
        }
    }

    /// Change canvas size, scaling element geometry to the new frame.
    pub fn resize_canvas(&mut self, width: u32, height: u32, preset_key: &str) {
        let (ow, oh) = (self.canvas.width as f32, self.canvas.height as f32);
        let (nw, nh) = (width as f32, height as f32);
        let (sx, sy) = (nw / ow, nh / oh);
        let s = sx.min(sy);
        for scene in &mut self.scenes {
            for e in &mut scene.elements {
                if e.kind == ElementKind::Background || (e.w >= ow - 1.0 && e.h >= oh - 1.0) {
                    e.x = 0.0;
                    e.y = 0.0;
                    e.w = nw;
                    e.h = nh;
                    continue;
                }
                let (cx, cy) = ((e.x + e.w / 2.0) * sx, (e.y + e.h / 2.0) * sy);
                // Full-width bars keep spanning the frame; everything else scales uniformly.
                e.w = if e.w >= ow * 0.7 { e.w * sx } else { e.w * s };
                e.h *= s;
                e.w = e.w.min(nw);
                e.x = (cx - e.w / 2.0).clamp(0.0, (nw - e.w).max(0.0)).round();
                e.y = (cy - e.h / 2.0).clamp(0.0, (nh - e.h).max(0.0)).round();
                e.w = e.w.round();
                e.h = e.h.round();
                e.font_size = (e.font_size * s).round().max(6.0);
                e.stroke_width = (e.stroke_width * s).round();
            }
        }
        self.canvas = Canvas { width, height, preset: preset_key.to_string() };
    }

    pub fn from_json(s: &str) -> Result<Project, String> {
        let mut p: Project = serde_json::from_str(s).map_err(|e| format!("JSON 解析失敗: {e}"))?;
        p.normalize();
        Ok(p)
    }

    pub fn to_json(&self) -> String {
        let mut p = self.clone();
        p.normalize();
        serde_json::to_string_pretty(&p).expect("serialise project")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colour_roundtrip() {
        assert_eq!(Rgb::parse("#ff8000"), Some(Rgb([255, 128, 0])));
        assert_eq!(Rgb::parse("fff"), Some(Rgb::WHITE));
        assert_eq!(Rgb([1, 2, 255]).hex(), "#0102FF");
    }

    #[test]
    fn project_roundtrip() {
        let p = crate::sample::sample_project("16:9");
        let s = p.to_json();
        let q = Project::from_json(&s).unwrap();
        assert_eq!(p.scenes.len(), q.scenes.len());
        assert_eq!(q.to_json(), s);
    }

    #[test]
    fn every_kind_has_defaults() {
        let p = Project::default();
        for k in ElementKind::ALL {
            let e = Element::new(k, p.next_element_id(k), 1920.0, 1080.0, 960.0, 540.0);
            assert!(e.w > 0.0 && e.h > 0.0, "{k:?}");
        }
    }

    #[test]
    fn resize_keeps_elements_inside() {
        let mut p = crate::sample::sample_project("16:9");
        p.resize_canvas(1080, 1920, "9:16");
        for s in &p.scenes {
            for e in &s.elements {
                assert!(e.x >= 0.0 && e.y >= 0.0 && e.x + e.w <= 1080.5 && e.y + e.h <= 1920.5, "{}", e.id);
            }
        }
    }
}
