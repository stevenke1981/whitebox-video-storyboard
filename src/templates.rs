//! Built-in scene templates (intro, outro, quiz, …) and user templates.
//!
//! Every template is laid out from fractions of the current canvas, with separate
//! arrangements for landscape and tall (square / portrait) frames, so it fits
//! 16:9, 9:16, 1:1, 4:5 and 4:3 projects. Texts follow the current language.

use crate::i18n::t;
use crate::model::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Template {
    Intro,
    Outro,
    TransitionWipe,
    TransitionFade,
    TransitionZoom,
    Chapter,
    Quiz,
    Comparison,
    KeyPoints,
    TalkingHead,
    Countdown,
    Quote,
    Product,
    Credits,
}

impl Template {
    pub const ALL: [Template; 14] = [
        Template::Intro,
        Template::Outro,
        Template::TransitionWipe,
        Template::TransitionFade,
        Template::TransitionZoom,
        Template::Chapter,
        Template::Quiz,
        Template::Comparison,
        Template::KeyPoints,
        Template::TalkingHead,
        Template::Countdown,
        Template::Quote,
        Template::Product,
        Template::Credits,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Template::Intro => "intro",
            Template::Outro => "outro",
            Template::TransitionWipe => "transition_wipe",
            Template::TransitionFade => "transition_fade",
            Template::TransitionZoom => "transition_zoom",
            Template::Chapter => "chapter",
            Template::Quiz => "quiz",
            Template::Comparison => "comparison",
            Template::KeyPoints => "key_points",
            Template::TalkingHead => "talking_head",
            Template::Countdown => "countdown",
            Template::Quote => "quote",
            Template::Product => "product",
            Template::Credits => "credits",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Template::Intro => t("片頭", "Intro / opening"),
            Template::Outro => t("片尾（訂閱 / CTA）", "Outro (subscribe / CTA)"),
            Template::TransitionWipe => t("轉場卡：擦除", "Transition card: wipe"),
            Template::TransitionFade => t("轉場卡：淡入淡出", "Transition card: fade"),
            Template::TransitionZoom => t("轉場卡：放大", "Transition card: zoom"),
            Template::Chapter => t("章節標題", "Chapter title"),
            Template::Quiz => t("問答 / 測驗", "Quiz"),
            Template::Comparison => t("對比（A vs B）", "Comparison (A vs B)"),
            Template::KeyPoints => t("清單 / 重點", "List / key points"),
            Template::TalkingHead => t("訪談 / 主持人", "Talking head + lower third"),
            Template::Countdown => t("倒數", "Countdown"),
            Template::Quote => t("引言", "Quote"),
            Template::Product => t("商品展示", "Product showcase"),
            Template::Credits => t("結尾名單", "Credits roll"),
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Template::Intro => t("Logo 彈出、節目名稱淡入、本集主題", "Logo pop, show name fade-in, episode topic"),
            Template::Outro => t(
                "感謝收看、訂閱/按讚按鈕、推薦影片、製作名單",
                "Thanks, subscribe/like buttons, end-screen videos, credits",
            ),
            Template::TransitionWipe => t("1 秒擦除轉場提示卡", "1 s wipe transition hint card"),
            Template::TransitionFade => t("1 秒黑場淡入提示卡", "1 s fade-through-black hint card"),
            Template::TransitionZoom => t("1 秒放大轉場提示卡", "1 s zoom transition hint card"),
            Template::Chapter => t("大章節編號、章節名稱與說明", "Big chapter number, title and summary"),
            Template::Quiz => {
                t("題目字卡、A/B/C/D 選項、倒數、進度條", "Question card, A/B/C/D options, countdown, progress bar")
            }
            Template::Comparison => {
                t("兩個素材並排比較，VS 標記與優缺點", "Two media side by side, VS badge, pros / cons")
            }
            Template::KeyPoints => t("標題＋3 個依序滑入的重點", "Title + 3 points sliding in one by one"),
            Template::TalkingHead => {
                t("人物鏡頭、下三分之一名牌、字幕", "Camera shot, lower-third name strap, subtitles")
            }
            Template::Countdown => t("大型倒數計時與進度條", "Big countdown timer and progress bar"),
            Template::Quote => t("置中引言與作者出處", "Centred quotation with author"),
            Template::Product => t("商品圖、賣點清單、價格與購買按鈕", "Product shot, features, price and buy button"),
            Template::Credits => t("向上捲動的工作人員名單", "Scrolling staff credits"),
        }
    }

    pub fn parse(key: &str) -> Option<Template> {
        let k = key.trim().to_ascii_lowercase().replace('-', "_");
        Template::ALL.into_iter().find(|t| t.key() == k).or(match k.as_str() {
            "opening" => Some(Template::Intro),
            "ending" | "end" => Some(Template::Outro),
            "wipe" | "transition" => Some(Template::TransitionWipe),
            "fade" => Some(Template::TransitionFade),
            "zoom" => Some(Template::TransitionZoom),
            "list" | "points" => Some(Template::KeyPoints),
            "interview" | "host" => Some(Template::TalkingHead),
            "vs" | "compare" => Some(Template::Comparison),
            _ => None,
        })
    }
}

const ACCENT: Rgb = Rgb([240, 128, 20]);

/// Small helper that places elements with unique ids into a new scene.
struct B<'a> {
    p: &'a Project,
    s: Scene,
    w: f32,
    h: f32,
    /// 1 px at a 1080 px short side.
    u: f32,
    /// Square or portrait frame → stacked layouts.
    tall: bool,
}

impl B<'_> {
    fn id(&self, kind: ElementKind) -> String {
        let mut n = 1;
        loop {
            let id = format!("{}_{n}", kind.key());
            let used = self.s.elements.iter().any(|e| e.id == id)
                || self.p.scenes.iter().any(|s| s.elements.iter().any(|e| e.id == id));
            if !used {
                return id;
            }
            n += 1;
        }
    }

    /// Add an element at a rectangle given in fractions of the frame.
    fn add(&mut self, kind: ElementKind, r: [f32; 4], f: impl FnOnce(&mut Element)) {
        let px = [r[0] * self.w, r[1] * self.h, r[2] * self.w, r[3] * self.h];
        self.add_px(kind, px, f);
    }

    /// Add an element at a pixel rectangle.
    fn add_px(&mut self, kind: ElementKind, r: [f32; 4], f: impl FnOnce(&mut Element)) {
        let mut e = Element::new(kind, self.id(kind), self.w, self.h, 0.0, 0.0);
        e.x = r[0].round();
        e.y = r[1].round();
        e.w = r[2].round().max(1.0);
        e.h = r[3].round().max(1.0);
        f(&mut e);
        // keep inside the frame
        e.w = e.w.min(self.w);
        e.h = e.h.min(self.h);
        e.x = e.x.clamp(0.0, self.w - e.w);
        e.y = e.y.clamp(0.0, self.h - e.h);
        self.s.elements.push(e);
    }

    /// Square of `side` px (scaled by `u`) centred at fractional (cx, cy).
    fn square(&self, cx: f32, cy: f32, side: f32) -> [f32; 4] {
        let s = side * self.u;
        [cx * self.w - s / 2.0, cy * self.h - s / 2.0, s, s]
    }

    fn fs(&self, v: f32) -> f32 {
        (v * self.u).round()
    }
}

/// Build a new scene from a built-in template for project `p` (ids are unique in `p`).
pub fn build(tpl: Template, p: &Project) -> Scene {
    let (w, h) = (p.canvas.width as f32, p.canvas.height as f32);
    let mut s = Scene::new(p.next_scene_id(), tpl.label().to_string());
    s.background = Rgb::gray(232);
    let tall = h > w * 1.1;
    // font unit: 1.0 at 1920×1080 / 1080×1920; squarer "wide" frames get smaller text
    let u = if tall { w / 1080.0 } else { (h / 1080.0).min(w / 1500.0) };
    let mut b = B { p, s, w, h, u, tall };
    let tall = b.tall;
    let fs = |b: &B, v: f32| b.fs(v);
    use ElementKind as K;
    match tpl {
        Template::Intro => {
            b.s.duration = 4.0;
            b.s.background = Rgb::gray(48);
            b.s.transition = Transition::new(TransitionKind::FadeBlack, 0.8);
            b.s.notes = t(
                "片頭 3–5 秒：Logo 彈出、節目名稱淡入、主題由下滑入；搭配片頭音樂或音效。",
                "3–5 s intro: logo pops, show name fades in, topic slides up; add intro music or a sting.",
            )
            .into();
            let logo = if tall { [0.3, 0.2, 0.4, 0.09] } else { [0.42, 0.16, 0.16, 0.14] };
            b.add(K::Logo, logo, |e| {
                e.animation = Animation::Pop;
                e.start = 0.2;
            });
            let fsz = fs(&b, 76.0);
            b.add(K::Title, [0.08, 0.38, 0.84, if tall { 0.14 } else { 0.16 }], |e| {
                e.text = t("頻道 / 節目名稱", "Channel / show name").into();
                e.font_size = fsz;
                e.font_color = Rgb::WHITE;
                e.start = 0.5;
            });
            b.add(K::Shape, [0.35, if tall { 0.535 } else { 0.555 }, 0.3, 0.006], |e| {
                e.bg_color = ACCENT;
                e.name = t("裝飾線", "Accent line").into();
                e.start = 0.8;
                e.animation = Animation::FadeIn;
            });
            let fsz = fs(&b, 37.0);
            b.add(K::Subheading, [0.1, if tall { 0.56 } else { 0.59 }, 0.8, if tall { 0.08 } else { 0.09 }], |e| {
                e.text = t("本集主題：一句話說明", "Episode topic in one line").into();
                e.font_size = fsz;
                e.font_color = Rgb::gray(215);
                e.start = 1.0;
                e.animation = Animation::SlideUp;
            });
        }
        Template::Outro => {
            b.s.duration = 10.0;
            b.s.transition = Transition::new(TransitionKind::Crossfade, 0.6);
            b.s.notes = t(
                "片尾 / 結束畫面：推薦影片框對應 YouTube 結束畫面元素（5–20 秒）；訂閱按鈕彈出，製作名單可縮小。",
                "Outro / end screen: the video boxes map to YouTube end-screen elements (5–20 s); subscribe button pops, credits can be small.",
            )
            .into();
            let fsz = fs(&b, 61.0);
            let (title, v1, v2) = if tall {
                ([0.08, 0.05, 0.84, 0.08], [0.1, 0.15, 0.8, 0.2], [0.1, 0.37, 0.8, 0.2])
            } else {
                ([0.05, 0.08, 0.45, 0.13], [0.53, 0.1, 0.42, 0.34], [0.53, 0.5, 0.42, 0.34])
            };
            b.add(K::Title, title, |e| {
                e.text = t("感謝收看！", "Thanks for watching!").into();
                e.font_size = fsz;
                e.align = if tall { Align::Center } else { Align::Left };
                e.animation = Animation::FadeIn;
            });
            for (i, r) in [v1, v2].into_iter().enumerate() {
                b.add(K::MediaPlaceholder, r, |e| {
                    e.text = if i == 0 {
                        t("推薦影片 1", "Suggested video 1")
                    } else {
                        t("推薦影片 2", "Suggested video 2")
                    }
                    .into();
                    e.start = 0.5 + i as f32 * 0.3;
                    e.animation = Animation::FadeIn;
                    e.notes = t("YouTube 結束畫面：影片 / 播放清單", "YouTube end screen: video / playlist").into();
                });
            }
            let (c1, c2, avatar, credits) = if tall {
                ([0.2, 0.61, 0.6, 0.065], [0.2, 0.69, 0.6, 0.06], b.square(0.2, 0.86, 200.0), [0.34, 0.78, 0.58, 0.16])
            } else {
                ([0.05, 0.3, 0.3, 0.11], [0.05, 0.44, 0.3, 0.1], b.square(0.13, 0.76, 240.0), [0.24, 0.62, 0.26, 0.28])
            };
            let fsz = fs(&b, 34.0);
            b.add(K::CtaButton, c1, |e| {
                e.text = t("訂閱頻道 ▶", "Subscribe ▶").into();
                e.font_size = fsz;
                e.start = 0.8;
            });
            let fsz = fs(&b, 29.0);
            b.add(K::CtaButton, c2, |e| {
                e.text = t("按讚・分享・開啟小鈴鐺", "Like · share · bell").into();
                e.font_size = fsz;
                e.bg_color = Rgb::gray(90);
                e.start = 1.4;
            });
            b.add_px(K::AvatarFrame, avatar, |e| {
                e.notes = t("YouTube 結束畫面：訂閱元素", "YouTube end screen: subscribe element").into()
            });
            let fsz = fs(&b, 21.0);
            b.add(K::TextCard, credits, |e| {
                e.text = t(
                    "製作：頻道名稱\n剪輯：王小明\n音樂：曲名 / 作者",
                    "Produced by: Channel\nEditing: Alex Chen\nMusic: Track / Artist",
                )
                .into();
                e.font_size = fsz;
                e.align = Align::Left;
                e.name = t("製作名單", "Credits").into();
            });
        }
        Template::TransitionWipe | Template::TransitionFade | Template::TransitionZoom => {
            b.s.duration = 1.2;
            let (kind, title, bg, fc) = match tpl {
                Template::TransitionWipe => {
                    (TransitionKind::Wipe, t("擦除轉場 WIPE", "WIPE transition"), Rgb::gray(205), Rgb::gray(30))
                }
                Template::TransitionFade => {
                    (TransitionKind::FadeBlack, t("淡入淡出 FADE", "FADE transition"), Rgb::gray(28), Rgb::gray(230))
                }
                _ => (TransitionKind::Zoom, t("放大轉場 ZOOM", "ZOOM transition"), Rgb::gray(215), Rgb::gray(30)),
            };
            b.s.background = bg;
            b.s.transition = Transition::new(kind, 1.0);
            b.s.notes = tf!(
                "轉場提示卡（{}）：可當作短暫的過場畫面，或刪掉此卡、只把相同轉場套用到下一個場景。",
                "Transition hint card ({}): use as a short stinger, or delete it and apply the same transition to the next scene.",
                kind.key()
            );
            match tpl {
                Template::TransitionWipe => {
                    b.add(K::Shape, [0.47, 0.0, 0.03, 1.0], |e| {
                        e.bg_color = ACCENT;
                        e.name = t("擦除邊界", "Wipe edge").into();
                        e.notes = t("由左往右移動的擦除邊界", "wipe edge moving left → right").into();
                    });
                    let r = if tall { [0.25, 0.6, 0.5, 0.08] } else { [0.6, 0.62, 0.25, 0.14] };
                    let fsz = fs(&b, 28.0);
                    b.add(K::CalloutArrow, r, |e| {
                        e.text = t("擦除方向", "wipe direction").into();
                        e.font_size = fsz;
                        e.direction = Direction::Right;
                    });
                }
                Template::TransitionZoom => {
                    let r = b.square(0.5, 0.5, 520.0);
                    b.add_px(K::Shape, r, |e| {
                        e.shape = ShapeKind::Circle;
                        e.bg_color = Rgb::gray(185);
                        e.name = t("放大焦點", "Zoom focus").into();
                        e.animation = Animation::Pop;
                    });
                }
                _ => {}
            }
            let fsz = fs(&b, 66.0);
            b.add(K::Title, [0.08, 0.4, 0.84, 0.14], |e| {
                e.text = title.into();
                e.font_size = fsz;
                e.font_color = fc;
                e.animation = Animation::None;
            });
            let fsz = fs(&b, 28.0);
            b.add(K::Subheading, [0.1, if tall { 0.55 } else { 0.56 }, 0.8, 0.07], |e| {
                e.text = t("示意轉場效果，可縮短或刪除", "Transition hint — shorten or delete").into();
                e.font_size = fsz;
                e.font_color = fc;
            });
        }
        Template::Chapter => {
            b.s.duration = 3.0;
            b.s.transition = Transition::new(TransitionKind::SlideLeft, 0.5);
            b.s.notes = t(
                "章節標題：編號放大彈出、章節名稱淡入；下方進度條表示影片進度。",
                "Chapter title: number pops, chapter name fades in; the bottom bar shows overall progress.",
            )
            .into();
            let (num, title, line, sub) = if tall {
                ([0.2, 0.26, 0.6, 0.14], [0.08, 0.43, 0.84, 0.09], [0.35, 0.53, 0.3, 0.005], [0.1, 0.55, 0.8, 0.07])
            } else {
                ([0.06, 0.3, 0.26, 0.32], [0.34, 0.33, 0.6, 0.14], [0.34, 0.48, 0.3, 0.007], [0.34, 0.5, 0.6, 0.09])
            };
            let fsz = fs(&b, 159.0);
            b.add(K::Title, num, |e| {
                e.text = "01".into();
                e.font_size = fsz;
                e.font_color = ACCENT;
                e.name = t("章節編號", "Chapter number").into();
                e.animation = Animation::Pop;
            });
            let fsz = fs(&b, 66.0);
            b.add(K::Title, title, |e| {
                e.text = t("章節名稱", "Chapter name").into();
                e.font_size = fsz;
                e.align = if tall { Align::Center } else { Align::Left };
                e.start = 0.3;
            });
            b.add(K::Shape, line, |e| {
                e.bg_color = ACCENT;
                e.name = t("裝飾線", "Accent line").into();
            });
            let fsz = fs(&b, 34.0);
            b.add(K::Subheading, sub, |e| {
                e.text = t("這一章要講什麼", "What this chapter covers").into();
                e.font_size = fsz;
                e.align = if tall { Align::Center } else { Align::Left };
                e.start = 0.6;
                e.animation = Animation::SlideUp;
            });
            b.add(K::ProgressBar, [0.0, 0.975, 1.0, 0.025], |e| {
                e.bg_opacity = 0.6;
                e.notes =
                    t("影片整體進度（例如第 1 / 5 章 = 20%）", "overall video progress (e.g. chapter 1 / 5 = 20 %)")
                        .into();
            });
        }
        Template::Quiz => {
            b.s.duration = 10.0;
            b.s.transition = Transition::new(TransitionKind::Crossfade, 0.5);
            b.s.notes = t("問答：題目淡入、選項滑入、倒數 5 秒，最後公布答案（answer 標示正確選項）。", "Quiz: question fades in, options slide in, 5 s countdown, then reveal the answer (answer = correct option).").into();
            let (q, o, cd, sub) = if tall {
                (
                    [0.08, 0.1, 0.84, 0.2],
                    [0.08, 0.33, 0.84, 0.33],
                    b.square(0.5, 0.735, 200.0),
                    [0.08, 0.84, 0.84, 0.06],
                )
            } else {
                (
                    [0.05, 0.13, 0.42, 0.52],
                    [0.53, 0.13, 0.42, 0.52],
                    b.square(0.9, 0.76, 170.0),
                    [0.12, 0.84, 0.66, 0.08],
                )
            };
            let fsz = fs(&b, 39.0);
            b.add(K::TextCard, q, |e| {
                e.text = t("Q. 在這裡輸入題目？", "Q. Type your question here?").into();
                e.font_size = fsz;
                e.animation = Animation::FadeIn;
            });
            let fsz = fs(&b, 32.0);
            b.add(K::Options, o, |e| {
                e.font_size = fsz;
                e.answer = Some(1);
                e.start = 0.5;
                e.animation = Animation::SlideLeft;
            });
            let fsz = fs(&b, 62.0);
            b.add_px(K::Countdown, cd, |e| {
                e.text = "5".into();
                e.font_size = fsz;
                e.start = 1.0;
                e.end = Some(6.0);
            });
            let fsz = fs(&b, 33.0);
            b.add(K::Subtitle, sub, |e| {
                e.text = t("倒數結束前選出答案！", "Choose before time runs out!").into();
                e.font_size = fsz;
            });
            b.add(K::ProgressBar, [0.0, 0.975, 1.0, 0.025], |e| {
                e.bg_opacity = 0.6;
                e.start = 1.0;
                e.end = Some(6.0);
            });
        }
        Template::Comparison => {
            b.s.duration = 8.0;
            b.s.transition = Transition::new(TransitionKind::Wipe, 0.6);
            b.s.notes = t(
                "對比：左右（直式為上下）兩個素材比較，VS 標記彈出，優缺點依序出現。",
                "Comparison: two media side by side (stacked when tall), VS badge pops, pros / cons appear in turn.",
            )
            .into();
            let fsz = fs(&b, 55.0);
            b.add(K::Title, [0.08, 0.04, 0.84, if tall { 0.07 } else { 0.11 }], |e| {
                e.text = t("A vs B：哪個比較好？", "A vs B: which is better?").into();
                e.font_size = fsz;
            });
            let rects = if tall {
                [
                    ([0.08, 0.13, 0.84, 0.26], [0.08, 0.4, 0.84, 0.08]),
                    ([0.08, 0.56, 0.84, 0.26], [0.08, 0.83, 0.84, 0.08]),
                ]
            } else {
                [
                    ([0.05, 0.18, 0.42, 0.44], [0.05, 0.65, 0.42, 0.24]),
                    ([0.53, 0.18, 0.42, 0.44], [0.53, 0.65, 0.42, 0.24]),
                ]
            };
            let fsz = fs(&b, 26.0);
            for (i, (m, c)) in rects.into_iter().enumerate() {
                let (name, pts) = if i == 0 {
                    (t("A 方案", "Option A"), t("A：優點…\n缺點…", "A: pros…\ncons…"))
                } else {
                    (t("B 方案", "Option B"), t("B：優點…\n缺點…", "B: pros…\ncons…"))
                };
                b.add(K::MediaPlaceholder, m, |e| {
                    e.text = name.into();
                    e.start = i as f32 * 0.4;
                    e.animation = Animation::FadeIn;
                });
                b.add(K::TextCard, c, |e| {
                    e.text = pts.into();
                    e.font_size = fsz;
                    e.align = Align::Left;
                    e.start = 1.5 + i as f32 * 0.8;
                    e.animation = Animation::SlideUp;
                });
            }
            let vs = if tall { b.square(0.5, 0.485, 150.0) } else { b.square(0.5, 0.4, 150.0) };
            let fsz = fs(&b, 41.0);
            b.add_px(K::Shape, vs, |e| {
                e.shape = ShapeKind::Circle;
                e.text = "VS".into();
                e.font_size = fsz;
                e.font_color = Rgb::WHITE;
                e.bg_color = ACCENT;
                e.name = t("VS 標記", "VS badge").into();
                e.start = 0.8;
                e.animation = Animation::Pop;
            });
        }
        Template::KeyPoints => {
            b.s.duration = 10.0;
            b.s.transition = Transition::new(TransitionKind::SlideUp, 0.5);
            b.s.notes = t(
                "清單：標題先出現，3 個重點每隔約 1.5 秒依序滑入，旁白對應每一點。",
                "List: the title appears first, then 3 points slide in about 1.5 s apart, matching the voice-over.",
            )
            .into();
            let fsz = fs(&b, 58.0);
            b.add(K::Title, [0.06, 0.07, 0.88, if tall { 0.08 } else { 0.13 }], |e| {
                e.text = t("3 個重點", "3 key points").into();
                e.font_size = fsz;
                e.align = Align::Left;
            });
            let points = [
                t("第一個重點：一句話說明", "First point in one sentence"),
                t("第二個重點：一句話說明", "Second point in one sentence"),
                t("第三個重點：一句話說明", "Third point in one sentence"),
            ];
            for (i, text) in points.into_iter().enumerate() {
                let (y, rh) = if tall { (0.22 + 0.2 * i as f32, 0.15) } else { (0.27 + 0.22 * i as f32, 0.17) };
                let side = 120.0;
                let num = [0.06 * b.w, y * b.h + (rh * b.h - side * b.u) / 2.0, side * b.u, side * b.u];
                let start = 0.8 + 1.5 * i as f32;
                let fsz = fs(&b, 44.0);
                b.add_px(K::Shape, num, |e| {
                    e.shape = ShapeKind::Circle;
                    e.text = format!("{}", i + 1);
                    e.font_size = fsz;
                    e.font_color = Rgb::WHITE;
                    e.bg_color = ACCENT;
                    e.name = tf!("編號 {}", "Number {}", i + 1);
                    e.start = start;
                    e.animation = Animation::Pop;
                });
                let x0 = (0.06 * b.w + side * b.u + 24.0 * b.u) / b.w;
                let fsz = fs(&b, 34.0);
                b.add(K::TextCard, [x0, y, 0.94 - x0, rh], |e| {
                    e.text = text.into();
                    e.font_size = fsz;
                    e.align = Align::Left;
                    e.start = start;
                    e.animation = Animation::SlideLeft;
                });
            }
        }
        Template::TalkingHead => {
            b.s.duration = 12.0;
            b.s.notes = t("訪談 / 主持人：全畫面人物鏡頭（A-roll），名牌在開頭幾秒出現後退場，字幕跟著旁白。", "Talking head: full-frame camera (A-roll); the name strap shows for the first seconds, subtitles follow the voice.").into();
            let fsz = fs(&b, 41.0);
            b.add(K::MediaPlaceholder, [0.0, 0.0, 1.0, 1.0], |e| {
                e.text = t("人物鏡頭（A-roll）", "Camera shot (A-roll)").into();
                e.font_size = fsz;
                e.src = "assets/a_roll.mp4".into();
                e.name = t("人物鏡頭", "Camera").into();
            });
            let lt = if tall { [0.06, 0.66, 0.8, 0.075] } else { [0.05, 0.68, 0.38, 0.13] };
            let fsz = fs(&b, 30.0);
            b.add(K::LowerThird, lt, |e| {
                e.text = t("受訪者姓名\n職稱 / 單位", "Guest name\nRole / company").into();
                e.font_size = fsz;
                e.start = 1.0;
                e.end = Some(6.0);
            });
            let sub = if tall { [0.06, 0.8, 0.88, 0.07] } else { [0.12, 0.85, 0.76, 0.09] };
            let fsz = fs(&b, 34.0);
            b.add(K::Subtitle, sub, |e| {
                e.text = t("字幕：跟著旁白逐句出現", "Subtitles follow the voice line by line").into();
                e.font_size = fsz;
            });
            let wm = if tall { [0.62, 0.05, 0.32, 0.035] } else { [0.76, 0.06, 0.19, 0.06] };
            b.add(K::Watermark, wm, |_| {});
        }
        Template::Countdown => {
            b.s.duration = 5.0;
            b.s.transition = Transition::new(TransitionKind::Zoom, 0.6);
            b.s.notes = t(
                "倒數：數字每秒變化（5→0），進度條同步跑完，可配滴答音效。",
                "Countdown: the number changes every second (5 → 0), the bar fills in sync; add a tick sound.",
            )
            .into();
            let fsz = fs(&b, 55.0);
            b.add(K::Title, [0.08, if tall { 0.18 } else { 0.08 }, 0.84, if tall { 0.08 } else { 0.13 }], |e| {
                e.text = t("即將開始", "Starting soon").into();
                e.font_size = fsz;
            });
            let cd = b.square(0.5, if tall { 0.46 } else { 0.5 }, 440.0);
            let fsz = fs(&b, 172.0);
            b.add_px(K::Countdown, cd, |e| {
                e.text = "5".into();
                e.font_size = fsz;
            });
            let fsz = fs(&b, 34.0);
            b.add(K::Subheading, [0.1, if tall { 0.64 } else { 0.8 }, 0.8, 0.08], |e| {
                e.text = t("準備好了嗎？", "Ready?").into();
                e.font_size = fsz;
                e.start = 0.5;
                e.animation = Animation::FadeIn;
            });
            b.add(K::ProgressBar, [0.0, 0.975, 1.0, 0.025], |e| e.bg_opacity = 0.6);
        }
        Template::Quote => {
            b.s.duration = 6.0;
            b.s.transition = Transition::new(TransitionKind::Crossfade, 0.8);
            b.s.notes = t("引言：引號與文字淡入，作者出處稍後出現；畫面保持簡潔、可加慢速推近。", "Quote: quote mark and text fade in, the author appears a moment later; keep it clean, optionally a slow push-in.").into();
            let qm = if tall { b.square(0.16, 0.24, 220.0) } else { b.square(0.12, 0.22, 240.0) };
            let fsz = fs(&b, 179.0);
            b.add_px(K::Sticker, qm, |e| {
                e.text = "“".into();
                e.font_size = fsz;
                e.font_color = ACCENT;
                e.name = t("引號", "Quote mark").into();
                e.animation = Animation::FadeIn;
            });
            let fsz = fs(&b, if tall { 44.0 } else { 48.0 });
            b.add(K::TextCard, [0.1, if tall { 0.3 } else { 0.27 }, 0.8, if tall { 0.3 } else { 0.38 }], |e| {
                e.text = t(
                    "一句值得被記住的話，\n放在畫面中央。",
                    "A line worth remembering,\nright in the middle of the frame.",
                )
                .into();
                e.font_size = fsz;
                e.bg_opacity = 0.0;
                e.start = 0.3;
                e.animation = Animation::FadeIn;
            });
            let fsz = fs(&b, 32.0);
            b.add(K::Subheading, [0.3, if tall { 0.62 } else { 0.68 }, 0.6, 0.08], |e| {
                e.text = t("— 作者 / 出處", "— Author / source").into();
                e.font_size = fsz;
                e.align = Align::Right;
                e.start = 1.5;
                e.animation = Animation::FadeIn;
            });
        }
        Template::Product => {
            b.s.duration = 8.0;
            b.s.transition = Transition::new(TransitionKind::SlideLeft, 0.5);
            b.s.notes = t("商品展示：商品圖（可用 360° 影片）、名稱與賣點依序出現，價格彈出，最後購買按鈕。", "Product showcase: product shot (360° video works), name and features appear in turn, price pops, then the buy button.").into();
            let (img, name, tag, feats, price, cta) = if tall {
                (
                    [0.08, 0.06, 0.84, 0.34],
                    [0.08, 0.42, 0.84, 0.07],
                    [0.08, 0.49, 0.84, 0.05],
                    [0.08, 0.56, 0.84, 0.17],
                    [0.08, 0.75, 0.84, 0.07],
                    [0.2, 0.84, 0.6, 0.065],
                )
            } else {
                (
                    [0.05, 0.1, 0.45, 0.78],
                    [0.55, 0.1, 0.4, 0.12],
                    [0.55, 0.22, 0.4, 0.08],
                    [0.55, 0.32, 0.4, 0.3],
                    [0.55, 0.64, 0.4, 0.11],
                    [0.55, 0.78, 0.28, 0.1],
                )
            };
            let al = if tall { Align::Center } else { Align::Left };
            b.add(K::MediaPlaceholder, img, |e| {
                e.text = t("商品圖 / 360° 影片", "Product shot / 360° video").into();
                e.src = "assets/product.png".into();
                e.animation = Animation::FadeIn;
            });
            let fsz = fs(&b, 55.0);
            b.add(K::Title, name, |e| {
                e.text = t("商品名稱", "Product name").into();
                e.font_size = fsz;
                e.align = al;
                e.start = 0.3;
            });
            let fsz = fs(&b, 29.0);
            b.add(K::Subheading, tag, |e| {
                e.text = t("一句話說出最大賣點", "The main selling point in one line").into();
                e.font_size = fsz;
                e.align = al;
                e.start = 0.6;
            });
            let fsz = fs(&b, 28.0);
            b.add(K::TextCard, feats, |e| {
                e.text = t("• 特色一\n• 特色二\n• 特色三", "• Feature one\n• Feature two\n• Feature three").into();
                e.font_size = fsz;
                e.align = Align::Left;
                e.start = 1.2;
                e.animation = Animation::SlideLeft;
            });
            let fsz = fs(&b, 58.0);
            b.add(K::Title, price, |e| {
                e.text = t("NT$ 990", "$29.99").into();
                e.font_size = fsz;
                e.font_color = ACCENT;
                e.align = al;
                e.name = t("價格", "Price").into();
                e.start = 2.5;
                e.animation = Animation::Pop;
            });
            let fsz = fs(&b, 33.0);
            b.add(K::CtaButton, cta, |e| {
                e.text = t("立即購買 ▶", "Buy now ▶").into();
                e.font_size = fsz;
                e.start = 3.2;
            });
        }
        Template::Credits => {
            b.s.duration = 12.0;
            b.s.background = Rgb::gray(25);
            b.s.transition = Transition::new(TransitionKind::FadeBlack, 1.0);
            b.s.notes = t("結尾名單：名單從畫面下方向上捲動（scroll_up），捲動時間＝元件時間；標題固定在上方。", "Credits roll: the list scrolls from below the frame to above it (scroll_up) over the element's time; the heading stays at the top.").into();
            let fsz = fs(&b, 55.0);
            b.add(K::Title, [0.1, 0.05, 0.8, if tall { 0.07 } else { 0.11 }], |e| {
                e.text = t("工作人員", "Credits").into();
                e.font_size = fsz;
                e.font_color = Rgb::WHITE;
            });
            let fsz = fs(&b, 29.0);
            b.add(K::TextCard, [0.2, if tall { 0.15 } else { 0.2 }, 0.6, if tall { 0.78 } else { 0.74 }], |e| {
                e.text = t(
                    "企劃\n王小明\n\n腳本\n陳大文\n\n攝影\n林小華\n\n剪輯\n張美玲\n\n音樂\n曲名 / 作者",
                    "Producer\nAlex Chen\n\nScript\nSam Lee\n\nCamera\nJamie Lin\n\nEditing\nMia Chang\n\nMusic\nTrack / Artist",
                )
                .into();
                e.font_size = fsz;
                e.font_color = Rgb::gray(235);
                e.bg_opacity = 0.0;
                e.start = 0.5;
                e.animation = Animation::ScrollUp;
                e.name = t("捲動名單", "Scrolling list").into();
            });
        }
    }
    let mut s = b.s;
    for (i, e) in s.elements.iter_mut().enumerate() {
        e.z = i as i32;
    }
    s
}

/// A scene saved by the user as a reusable template (stored as JSON).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserTemplate {
    pub name: String,
    /// Canvas the scene was designed on (used to rescale on insert).
    pub canvas: Canvas,
    pub scene: Scene,
}

impl UserTemplate {
    pub fn from_scene(name: &str, p: &Project, scene: &Scene) -> UserTemplate {
        UserTemplate { name: name.to_string(), canvas: p.canvas.clone(), scene: scene.clone() }
    }

    /// The scene rescaled to `p`'s canvas, with fresh ids that are unique in `p`.
    pub fn instantiate(&self, p: &Project) -> Scene {
        let mut tmp = Project { canvas: self.canvas.clone(), scenes: vec![self.scene.clone()], ..Project::default() };
        if tmp.canvas.width != p.canvas.width || tmp.canvas.height != p.canvas.height {
            tmp.resize_canvas(p.canvas.width, p.canvas.height, &p.canvas.preset);
        }
        let mut s = tmp.scenes.remove(0);
        if s.name.trim().is_empty() {
            s.name = self.name.clone();
        }
        adopt_scene(p, s)
    }
}

/// Give `scene` a new scene id and element ids that do not collide with `p`.
pub fn adopt_scene(p: &Project, mut scene: Scene) -> Scene {
    scene.id = p.next_scene_id();
    let elems = std::mem::take(&mut scene.elements);
    let mut b = B { p, s: scene, w: 1.0, h: 1.0, u: 1.0, tall: false };
    for mut e in elems {
        e.id = b.id(e.kind);
        b.s.elements.push(e);
    }
    b.s
}

/// Safe file name for a user template.
pub fn file_stem(name: &str) -> String {
    let mut s = String::new();
    for c in name.chars() {
        let c = if c.is_alphanumeric() || c == '-' { c } else { '_' };
        if !(c == '_' && s.ends_with('_')) {
            s.push(c);
        }
    }
    let s = s.trim_matches('_').to_string();
    if s.is_empty() { "template".into() } else { s }
}

/// A new project made of the given templates.
pub fn project_from_templates(tpls: &[Template], preset_key: &str, name: &str) -> Project {
    let mut p = Project::new(name, preset_key);
    p.scenes.clear();
    for &tp in tpls {
        let s = build(tp, &p);
        p.scenes.push(s);
    }
    if let Some(first) = p.scenes.first_mut() {
        // nothing to transition from
        if first.transition.kind != TransitionKind::FadeBlack {
            first.transition = Transition::default();
        }
    }
    p.normalize();
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{Lang, with_lang};

    #[test]
    fn templates_fit_every_preset() {
        for pr in PRESETS {
            let p = Project::new("t", pr.key);
            for tp in Template::ALL {
                let s = build(tp, &p);
                assert!(!s.elements.is_empty(), "{tp:?}");
                for e in &s.elements {
                    assert!(
                        e.x >= 0.0
                            && e.y >= 0.0
                            && e.x + e.w <= pr.width as f32 + 0.5
                            && e.y + e.h <= pr.height as f32 + 0.5,
                        "{} {:?} {}",
                        pr.key,
                        tp,
                        e.id
                    );
                }
                let mut ids: Vec<_> = s.elements.iter().map(|e| e.id.clone()).collect();
                ids.sort();
                ids.dedup();
                assert_eq!(ids.len(), s.elements.len());
            }
        }
    }

    #[test]
    fn project_ids_unique_and_languages() {
        let zh = project_from_templates(&Template::ALL, "9:16", "x");
        let mut ids: Vec<_> = zh.scenes.iter().flat_map(|s| s.elements.iter().map(|e| e.id.clone())).collect();
        let n = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), n);
        assert_eq!(zh.scenes[0].name, "片頭");
        let en = with_lang(Lang::En, || project_from_templates(&[Template::Intro, Template::Outro], "16:9", "x"));
        assert_eq!(en.scenes[1].name, "Outro (subscribe / CTA)");
        assert!(en.scenes[1].elements.iter().any(|e| e.text == "Thanks for watching!"));
        assert_eq!(Template::parse("talking-head"), Some(Template::TalkingHead));
    }

    #[test]
    fn user_template_rescales() {
        let p = crate::sample::sample_project("16:9");
        let ut = UserTemplate::from_scene("mine", &p, &p.scenes[1]);
        let json = serde_json::to_string(&ut).unwrap();
        let ut: UserTemplate = serde_json::from_str(&json).unwrap();
        let q = Project::new("v", "9:16");
        let s = ut.instantiate(&q);
        assert_eq!(s.elements.len(), p.scenes[1].elements.len());
        assert!(s.elements.iter().all(|e| e.x + e.w <= 1080.5));
        // ids unique when inserted into the same project
        let s2 = ut.instantiate(&p);
        assert!(s2.elements.iter().all(|e| p.scenes.iter().all(|sc| sc.find(&e.id).is_none())));
    }

    #[test]
    fn english_docs_have_no_cjk() {
        use crate::i18n::{Lang, with_lang};
        let cjk = |s: &str| {
            s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c) || ('\u{ff00}'..='\u{ffef}').contains(&c))
        };
        with_lang(Lang::En, || {
            for preset in ["16:9", "9:16"] {
                let p = project_from_templates(&Template::ALL, preset, "Demo");
                let f = crate::export::Formats::default();
                let md = crate::describe::storyboard_md(&p, &f);
                let guide = crate::guide::agent_guide(&p, None, &f);
                let html = crate::html::storyboard_html(&p, &[], &f);
                assert!(!cjk(&md), "markdown");
                assert!(!cjk(&guide), "guide");
                assert!(!cjk(&html), "html");
                assert!(guide.contains("crossfade") && guide.contains("scroll_up"));
            }
        });
    }

    #[test]
    fn v1_project_font_sizes_are_migrated() {
        let mut p = crate::sample::sample_project("16:9");
        p.version = 1;
        p.scenes[0].elements[0].font_size = 100.0;
        let mut json: serde_json::Value = serde_json::from_str(&p.to_json()).unwrap();
        json["version"] = 1.into();
        let q = Project::from_json(&json.to_string()).unwrap();
        assert_eq!(q.version, crate::model::PROJECT_VERSION);
        assert_eq!(q.scenes[0].elements[0].font_size, 69.0);
        // current files are left alone
        let r = Project::from_json(&q.to_json()).unwrap();
        assert_eq!(r.scenes[0].elements[0].font_size, 69.0);
    }
}
