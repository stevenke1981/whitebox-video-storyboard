//! Project data model: canvas, scenes (shots) and white-model elements.
//!
//! The same structures are used for the editable project file and (with a few
//! derived fields added by [`crate::export`]) for the exported `layout.json`.

use crate::i18n::t;
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

/// Palette groups (zh keys, in display order).
pub const GROUPS: [&str; 4] = ["文字", "互動", "媒體", "版面"];

/// Palette group name in the current language (`group` is the zh key).
pub fn group_label(group: &str) -> &'static str {
    match group {
        "文字" => t("文字", "Text"),
        "互動" => t("互動", "Interactive"),
        "媒體" => t("媒體", "Media"),
        _ => t("版面", "Layout"),
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

    /// Label in the current UI language.
    pub fn label(self) -> &'static str {
        let i = self.info();
        t(i.zh, i.en)
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
    pub fn label(self) -> &'static str {
        match self {
            Align::Left => t("靠左", "Left"),
            Align::Center => t("置中", "Center"),
            Align::Right => t("靠右", "Right"),
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
    /// Moves from below the frame to above it over the element's time (credits roll).
    ScrollUp,
}

impl Animation {
    pub const ALL: [Animation; 9] = [
        Animation::None,
        Animation::FadeIn,
        Animation::FadeOut,
        Animation::FadeInOut,
        Animation::SlideUp,
        Animation::SlideLeft,
        Animation::Pop,
        Animation::Typewriter,
        Animation::ScrollUp,
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
            Animation::ScrollUp => "scroll_up",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Animation::None => t("無", "None"),
            Animation::FadeIn => t("淡入", "Fade in"),
            Animation::FadeOut => t("淡出", "Fade out"),
            Animation::FadeInOut => t("淡入淡出", "Fade in/out"),
            Animation::SlideUp => t("由下滑入", "Slide up"),
            Animation::SlideLeft => t("由右滑入", "Slide from right"),
            Animation::Pop => t("彈出放大", "Pop"),
            Animation::Typewriter => t("打字機", "Typewriter"),
            Animation::ScrollUp => t("向上捲動（名單）", "Scroll up (credits)"),
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
    pub fn label(self) -> &'static str {
        match self {
            ShapeKind::Rect => t("矩形", "Rectangle"),
            ShapeKind::Rounded => t("圓角矩形", "Rounded rect"),
            ShapeKind::Circle => t("圓形/橢圓", "Circle / ellipse"),
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
    pub fn label(self) -> &'static str {
        match self {
            Direction::Left => t("← 向左", "← Left"),
            Direction::Right => t("→ 向右", "→ Right"),
            Direction::Up => t("↑ 向上", "↑ Up"),
            Direction::Down => t("↓ 向下", "↓ Down"),
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
    33.0
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
            Background => (cw, ch, t("背景", "Background"), 28.0, Rgb::gray(90), Rgb::gray(225), 1.0),
            Title => (1100.0, 150.0, t("主標題文字", "Main title"), 66.0, Rgb::gray(20), Rgb::gray(200), 0.0),
            Subheading => {
                (900.0, 90.0, t("副標題說明文字", "Subheading text"), 37.0, Rgb::gray(50), Rgb::gray(200), 0.0)
            }
            Subtitle => (
                1400.0,
                100.0,
                t("這裡是字幕，一行約 18 字", "Subtitle line goes here"),
                36.0,
                Rgb::WHITE,
                Rgb::BLACK,
                0.45,
            ),
            TextCard => (
                900.0,
                420.0,
                t("字卡重點\n第二行說明", "Key point\nSecond line"),
                41.0,
                Rgb::gray(30),
                Rgb::gray(245),
                0.92,
            ),
            Options => (760.0, 420.0, "", 30.0, Rgb::gray(30), Rgb::gray(240), 0.95),
            LowerThird => (
                720.0,
                140.0,
                t("姓名 Name\n職稱 / 頻道", "Name\nTitle / channel"),
                30.0,
                Rgb::WHITE,
                Rgb::gray(40),
                0.88,
            ),
            MediaPlaceholder => {
                (800.0, 450.0, t("圖片 / 影片", "Image / video"), 30.0, Rgb::gray(70), Rgb::gray(185), 1.0)
            }
            Logo => (220.0, 120.0, "LOGO", 28.0, Rgb::gray(70), Rgb::gray(205), 1.0),
            AvatarFrame => (320.0, 320.0, t("主持人", "Host"), 25.0, Rgb::gray(60), Rgb::gray(195), 1.0),
            ProgressBar => (1600.0, 24.0, "", 17.0, Rgb([250, 180, 40]), Rgb::gray(90), 0.8),
            Countdown => (180.0, 180.0, "10", 62.0, Rgb::gray(20), Rgb::gray(235), 0.95),
            Watermark => (360.0, 60.0, "@my_channel", 23.0, Rgb::gray(110), Rgb::gray(0), 0.0),
            CtaButton => (420.0, 110.0, t("立即訂閱 ▶", "Subscribe ▶"), 34.0, Rgb::WHITE, Rgb([220, 60, 60]), 1.0),
            CalloutArrow => (360.0, 140.0, t("看這裡！", "Look here!"), 28.0, Rgb::gray(20), Rgb([255, 210, 60]), 1.0),
            Shape => (300.0, 300.0, "", 25.0, Rgb::gray(60), Rgb::gray(175), 1.0),
            QrCode => (240.0, 240.0, "QR", 22.0, Rgb::gray(20), Rgb::WHITE, 1.0),
            Sticker => (160.0, 160.0, "★", 66.0, Rgb([250, 190, 30]), Rgb::gray(235), 0.0),
        };
        let (w, h) = if kind == Background { (w, h) } else { ((w * u).min(cw), (h * u).min(ch)) };
        let (x, y) = if kind == Background { (0.0, 0.0) } else { (cx - w / 2.0, cy - h / 2.0) };
        let mut e = Element {
            id,
            name: kind.label().to_string(),
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
                e.options = if crate::i18n::is_en() {
                    ["Option A", "Option B", "Option C", "Option D"]
                } else {
                    ["選項 A", "選項 B", "選項 C", "選項 D"]
                }
                .map(String::from)
                .to_vec();
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

/// How a scene enters (transition from the previous scene). The transition
/// happens during the first `duration` seconds of the scene, so it never changes
/// scene start times or the total length.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionKind {
    #[default]
    None,
    Crossfade,
    FadeBlack,
    SlideLeft,
    SlideUp,
    Wipe,
    Zoom,
}

impl TransitionKind {
    pub const ALL: [TransitionKind; 7] = [
        TransitionKind::None,
        TransitionKind::Crossfade,
        TransitionKind::FadeBlack,
        TransitionKind::SlideLeft,
        TransitionKind::SlideUp,
        TransitionKind::Wipe,
        TransitionKind::Zoom,
    ];
    pub fn key(self) -> &'static str {
        match self {
            TransitionKind::None => "none",
            TransitionKind::Crossfade => "crossfade",
            TransitionKind::FadeBlack => "fade_black",
            TransitionKind::SlideLeft => "slide_left",
            TransitionKind::SlideUp => "slide_up",
            TransitionKind::Wipe => "wipe",
            TransitionKind::Zoom => "zoom",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            TransitionKind::None => t("無（直接切換）", "None (hard cut)"),
            TransitionKind::Crossfade => t("交叉淡化", "Crossfade"),
            TransitionKind::FadeBlack => t("黑場淡入", "Fade through black"),
            TransitionKind::SlideLeft => t("由右推入", "Slide in from right"),
            TransitionKind::SlideUp => t("由下推入", "Slide in from bottom"),
            TransitionKind::Wipe => t("擦除（左→右）", "Wipe (left → right)"),
            TransitionKind::Zoom => t("放大淡入", "Zoom in + fade"),
        }
    }
    /// One-line description for the agent docs.
    pub fn describe(self) -> &'static str {
        match self {
            TransitionKind::None => t("直接切換，沒有轉場", "hard cut, no transition"),
            TransitionKind::Crossfade => t(
                "新場景從透明淡入，蓋在上一場景最後一格上",
                "the new scene fades in over the last frame of the previous scene",
            ),
            TransitionKind::FadeBlack => t("從全黑淡入新場景", "the new scene fades in from black"),
            TransitionKind::SlideLeft => t(
                "新場景從畫面右側推入，蓋住上一場景最後一格",
                "the new scene slides in from the right over the previous scene's last frame",
            ),
            TransitionKind::SlideUp => t(
                "新場景從畫面下方推入，蓋住上一場景最後一格",
                "the new scene slides in from the bottom over the previous scene's last frame",
            ),
            TransitionKind::Wipe => t(
                "一條垂直邊界由左往右掃過，逐步露出新場景",
                "a vertical edge sweeps left → right revealing the new scene",
            ),
            TransitionKind::Zoom => {
                t("新場景從 70% 大小放大到 100% 並同時淡入", "the new scene scales 70% → 100% while fading in")
            }
        }
    }
    pub fn moviepy(self) -> &'static str {
        match self {
            TransitionKind::None => "",
            TransitionKind::Crossfade => {
                "scene.with_effects([vfx.CrossFadeIn(d)]) composited over prev.to_ImageClip(prev.duration - 1/fps)"
            }
            TransitionKind::FadeBlack => "scene.with_effects([vfx.FadeIn(d)])",
            TransitionKind::SlideLeft => {
                "scene.with_position(lambda t: (W * max(0, 1 - t / d), 0)) over prev last frame"
            }
            TransitionKind::SlideUp => "scene.with_position(lambda t: (0, H * max(0, 1 - t / d))) over prev last frame",
            TransitionKind::Wipe => {
                "scene.with_mask(VideoClip(lambda t: mask with x < W * t / d, is_mask=True)) over prev last frame"
            }
            TransitionKind::Zoom => "scene.resized(lambda t: 0.7 + 0.3 * min(1, t / d)) centred + CrossFadeIn(d)",
        }
    }
}

fn default_transition_duration() -> f32 {
    0.6
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transition {
    #[serde(rename = "type", default)]
    pub kind: TransitionKind,
    /// Seconds (taken from the start of the scene).
    #[serde(default = "default_transition_duration")]
    pub duration: f32,
}

impl Default for Transition {
    fn default() -> Self {
        Transition { kind: TransitionKind::None, duration: default_transition_duration() }
    }
}

impl Transition {
    pub fn new(kind: TransitionKind, duration: f32) -> Transition {
        Transition { kind, duration }
    }
    pub fn is_none(&self) -> bool {
        self.kind == TransitionKind::None
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
    /// Transition *into* this scene (ignored for the first scene).
    #[serde(default, skip_serializing_if = "Transition::is_none")]
    pub transition: Transition,
    #[serde(default)]
    pub elements: Vec<Element>,
}

fn default_scene_bg() -> Rgb {
    Rgb::gray(236)
}

impl Scene {
    pub fn new(id: String, name: String) -> Scene {
        Scene {
            id,
            name,
            duration: 5.0,
            background: default_scene_bg(),
            notes: String::new(),
            transition: Transition::default(),
            elements: vec![],
        }
    }
    pub fn find(&self, id: &str) -> Option<usize> {
        self.elements.iter().position(|e| e.id == id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AspectPreset {
    pub key: &'static str,
    pub label: &'static str,
    pub label_en: &'static str,
    pub width: u32,
    pub height: u32,
}

pub const PRESETS: [AspectPreset; 5] = [
    AspectPreset {
        key: "16:9",
        label: "16:9 橫式 1920×1080",
        label_en: "16:9 landscape 1920×1080",
        width: 1920,
        height: 1080,
    },
    AspectPreset {
        key: "9:16",
        label: "9:16 直式 1080×1920",
        label_en: "9:16 portrait 1080×1920",
        width: 1080,
        height: 1920,
    },
    AspectPreset {
        key: "1:1",
        label: "1:1 方形 1080×1080",
        label_en: "1:1 square 1080×1080",
        width: 1080,
        height: 1080,
    },
    AspectPreset {
        key: "4:5",
        label: "4:5 直式 1080×1350",
        label_en: "4:5 portrait 1080×1350",
        width: 1080,
        height: 1350,
    },
    AspectPreset { key: "4:3", label: "4:3 1440×1080", label_en: "4:3 1440×1080", width: 1440, height: 1080 },
];

impl AspectPreset {
    pub fn label(&self) -> &'static str {
        t(self.label, self.label_en)
    }
}

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

/// Project format version. v2: `font_size` is the em size in pixels (CSS / Pillow /
/// MoviePy semantics). v1 files measured the ascent-to-descent height instead, so
/// their font sizes are scaled by [`V1_FONT_FACTOR`] when loaded.
pub const PROJECT_VERSION: u32 = 2;
/// em / (ascent − descent) of the bundled Noto Sans CJK font (1000 / 1448).
pub const V1_FONT_FACTOR: f32 = 0.69;
fn default_fps() -> u32 {
    30
}

impl Default for Project {
    fn default() -> Self {
        Project::new(t("未命名專案", "Untitled project"), "16:9")
    }
}

impl Project {
    pub fn new(name: &str, preset_key: &str) -> Project {
        let p = preset(preset_key).unwrap_or(PRESETS[0]);
        Project {
            version: PROJECT_VERSION,
            name: name.to_string(),
            canvas: Canvas { width: p.width, height: p.height, preset: p.key.to_string() },
            fps: 30,
            scenes: vec![Scene::new("scene_1".into(), tf!("場景 1", "Scene 1"))],
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
            self.scenes.push(Scene::new("scene_1".into(), tf!("場景 1", "Scene 1")));
        }
        for s in &mut self.scenes {
            if s.duration.is_nan() || s.duration <= 0.0 {
                s.duration = 1.0;
            }
            if !s.transition.duration.is_finite() {
                s.transition.duration = default_transition_duration();
            }
            s.transition.duration = s.transition.duration.clamp(0.1, s.duration.max(0.1));
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
        let mut p: Project = serde_json::from_str(s).map_err(|e| tf!("JSON 解析失敗: {e}", "JSON parse error: {e}"))?;
        if p.version < 2 {
            for e in p.scenes.iter_mut().flat_map(|s| s.elements.iter_mut()) {
                e.font_size = (e.font_size * V1_FONT_FACTOR).round().max(6.0);
            }
            p.version = PROJECT_VERSION;
        }
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
