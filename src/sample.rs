//! Built-in sample project (a three-shot quiz video) used by `--sample`,
//! File ▸ 範例專案 and the tests.

use crate::i18n::t;
use crate::model::*;

pub fn sample_project(preset_key: &str) -> Project {
    if preset_key == "9:16" {
        return sample_vertical();
    }
    let mut p = Project::new(t("MoviePy 小測驗（範例）", "MoviePy quiz (sample)"), "16:9");
    p.scenes.clear();
    let (cw, ch) = (1920.0, 1080.0);
    let add =
        |p: &mut Project, si: usize, kind: ElementKind, x: f32, y: f32, w: f32, h: f32, f: &dyn Fn(&mut Element)| {
            let mut e = Element::new(kind, p.next_element_id(kind), cw, ch, 0.0, 0.0);
            e.x = x;
            e.y = y;
            e.w = w;
            e.h = h;
            f(&mut e);
            p.scenes[si].elements.push(e);
        };

    // Scene 1 – opening
    let mut s = Scene::new("scene_1".into(), t("開場", "Opening").into());
    s.duration = 4.0;
    s.background = Rgb::gray(232);
    s.notes = t("開場：標題淡入，主持人自我介紹。", "Opening: title fades in, host introduces themself.").into();
    p.scenes.push(s);
    add(&mut p, 0, ElementKind::Logo, 96.0, 64.0, 220.0, 110.0, &|_| {});
    add(&mut p, 0, ElementKind::Title, 360.0, 330.0, 1200.0, 160.0, &|e| {
        e.text = t("三分鐘學會 MoviePy 排版", "MoviePy layouts in 3 minutes").into();
        e.font_size = 69.0;
        e.start = 0.3;
    });
    add(&mut p, 0, ElementKind::Subheading, 460.0, 500.0, 1000.0, 90.0, &|e| {
        e.text =
            t("白模草稿 → layout.json → 自動合成影片", "Whitebox draft → layout.json → auto-rendered video").into();
        e.start = 0.8;
        e.animation = Animation::SlideUp;
    });
    add(&mut p, 0, ElementKind::AvatarFrame, 1500.0, 700.0, 300.0, 300.0, &|e| e.text = t("主持人", "Host").into());
    add(&mut p, 0, ElementKind::LowerThird, 96.0, 820.0, 720.0, 140.0, &|e| {
        e.text = t("Ke Sheng Da\n影片創作者 · 自動化剪輯", "Ke Sheng Da\nVideo creator · editing automation").into();
        e.start = 1.0;
        e.end = Some(3.8);
    });
    add(&mut p, 0, ElementKind::Watermark, 1464.0, 64.0, 360.0, 60.0, &|_| {});

    // Scene 2 – question
    let mut s = Scene::new("scene_2".into(), t("題目", "Question").into());
    s.duration = 6.0;
    s.background = Rgb::gray(225);
    s.notes = t(
        "出題：選項依序出現，倒數 5 秒，進度條跑完。",
        "Question: options appear one by one, 5 s countdown, progress bar fills.",
    )
    .into();
    s.transition = Transition::new(TransitionKind::Crossfade, 0.6);
    p.scenes.push(s);
    add(&mut p, 1, ElementKind::TextCard, 96.0, 140.0, 800.0, 560.0, &|e| {
        e.text = t(
            "Q1. 下列哪一個是\nMoviePy v2 設定位置的寫法？",
            "Q1. Which one sets the position\nof a clip in MoviePy v2?",
        )
        .into();
        e.font_size = 40.0;
        e.animation = Animation::FadeIn;
    });
    add(&mut p, 1, ElementKind::Options, 1040.0, 140.0, 780.0, 560.0, &|e| {
        e.options = vec![
            "clip.set_pos(...)".into(),
            "clip.with_position(...)".into(),
            "clip.position = ...".into(),
            "clip.move(...)".into(),
        ];
        e.answer = Some(1);
        e.start = 0.5;
        e.animation = Animation::SlideLeft;
    });
    add(&mut p, 1, ElementKind::Countdown, 1640.0, 760.0, 170.0, 170.0, &|e| {
        e.text = "5".into();
        e.start = 1.0;
    });
    add(&mut p, 1, ElementKind::CalloutArrow, 1260.0, 760.0, 360.0, 130.0, &|e| {
        e.text = t("倒數中！", "Counting down!").into();
        e.start = 1.0;
        e.animation = Animation::Pop;
    });
    add(&mut p, 1, ElementKind::Subtitle, 260.0, 900.0, 1400.0, 90.0, &|e| {
        e.text = t("請在倒數結束前，選出正確答案", "Pick the right answer before time runs out").into()
    });
    add(&mut p, 1, ElementKind::ProgressBar, 0.0, 1050.0, 1920.0, 30.0, &|e| e.bg_opacity = 0.6);
    add(&mut p, 1, ElementKind::Sticker, 900.0, 20.0, 120.0, 120.0, &|e| e.text = t("？", "?").into());

    // Scene 3 – outro
    let mut s = Scene::new("scene_3".into(), t("結尾", "Ending").into());
    s.duration = 4.0;
    s.background = Rgb::gray(236);
    s.notes = t(
        "結尾：公布答案、放精華片段、引導訂閱。",
        "Ending: reveal the answer, show highlights, ask viewers to subscribe.",
    )
    .into();
    s.transition = Transition::new(TransitionKind::SlideLeft, 0.5);
    p.scenes.push(s);
    add(&mut p, 2, ElementKind::MediaPlaceholder, 96.0, 120.0, 960.0, 540.0, &|e| {
        e.text = t("精華片段 B-roll", "Highlights B-roll").into();
        e.src = "assets/broll.mp4".into();
    });
    add(&mut p, 2, ElementKind::Title, 1120.0, 160.0, 700.0, 130.0, &|e| {
        e.text = t("答案：B", "Answer: B").into();
        e.font_size = 66.0;
        e.align = Align::Left;
        e.animation = Animation::Pop;
    });
    add(&mut p, 2, ElementKind::Subheading, 1120.0, 300.0, 700.0, 140.0, &|e| {
        e.text = t("v2 統一用 with_* 方法回傳新的 clip", "v2 uses with_* methods that return a new clip").into();
        e.align = Align::Left;
        e.font_size = 32.0;
    });
    add(&mut p, 2, ElementKind::CtaButton, 1120.0, 500.0, 440.0, 110.0, &|e| e.start = 1.0);
    add(&mut p, 2, ElementKind::QrCode, 1600.0, 470.0, 220.0, 220.0, &|_| {});
    add(&mut p, 2, ElementKind::Shape, 96.0, 720.0, 960.0, 12.0, &|e| {
        e.bg_color = Rgb([250, 180, 40]);
        e.name = t("裝飾線", "Accent line").into();
    });
    add(&mut p, 2, ElementKind::Subtitle, 260.0, 900.0, 1400.0, 90.0, &|e| {
        e.text = t("我們下一集見！", "See you in the next episode!").into()
    });

    if let Some(pr) = preset(preset_key)
        && pr.key != "16:9"
    {
        p.resize_canvas(pr.width, pr.height, pr.key);
    }
    p.normalize();
    p
}

/// Hand-laid-out 9:16 (Shorts / Reels) version of the sample.
fn sample_vertical() -> Project {
    let mut p = Project::new(t("MoviePy 小測驗（直式範例）", "MoviePy quiz (vertical sample)"), "9:16");
    p.scenes.clear();
    let (cw, ch) = (1080.0, 1920.0);
    type Setup<'a> = &'a dyn Fn(&mut Element);
    let add = |p: &mut Project, si: usize, kind: ElementKind, r: [f32; 4], f: Setup| {
        let mut e = Element::new(kind, p.next_element_id(kind), cw, ch, 0.0, 0.0);
        [e.x, e.y, e.w, e.h] = r;
        f(&mut e);
        p.scenes[si].elements.push(e);
    };
    let scene = |id: &str, name: &str, dur: f32, notes: &str, tr: Transition| {
        let mut s = Scene::new(id.into(), name.into());
        s.transition = tr;
        s.duration = dur;
        s.background = Rgb::gray(232);
        s.notes = notes.into();
        s
    };

    p.scenes.push(scene(
        "scene_1",
        t("開場", "Opening"),
        4.0,
        t("開場：標題淡入，主持人自我介紹。", "Opening: title fades in, host introduces themself."),
        Transition::default(),
    ));
    add(&mut p, 0, ElementKind::Logo, [70.0, 90.0, 200.0, 100.0], &|_| {});
    add(&mut p, 0, ElementKind::Watermark, [690.0, 110.0, 320.0, 60.0], &|_| {});
    add(&mut p, 0, ElementKind::Title, [90.0, 520.0, 900.0, 300.0], &|e| {
        e.text = t("三分鐘學會\nMoviePy 排版", "MoviePy layouts\nin 3 minutes").into();
        e.font_size = 76.0;
        e.start = 0.3;
    });
    add(&mut p, 0, ElementKind::Subheading, [110.0, 840.0, 860.0, 90.0], &|e| {
        e.text = t("白模草稿 → 自動合成影片", "Whitebox draft → auto video").into();
        e.font_size = 36.0;
        e.start = 0.8;
        e.animation = Animation::SlideUp;
    });
    add(&mut p, 0, ElementKind::AvatarFrame, [340.0, 1060.0, 400.0, 400.0], &|e| e.text = t("主持人", "Host").into());
    add(&mut p, 0, ElementKind::LowerThird, [90.0, 1560.0, 760.0, 150.0], &|e| {
        e.text = t("Ke Sheng Da\n影片創作者 · 自動化剪輯", "Ke Sheng Da\nVideo creator · editing automation").into();
        e.font_size = 32.0;
        e.start = 1.0;
        e.end = Some(3.8);
    });

    p.scenes.push(scene(
        "scene_2",
        t("題目", "Question"),
        6.0,
        t(
            "出題：選項依序出現，倒數 5 秒，進度條跑完。",
            "Question: options appear one by one, 5 s countdown, progress bar fills.",
        ),
        Transition::new(TransitionKind::Crossfade, 0.6),
    ));
    add(&mut p, 1, ElementKind::Sticker, [880.0, 80.0, 130.0, 130.0], &|e| e.text = t("？", "?").into());
    add(&mut p, 1, ElementKind::TextCard, [90.0, 230.0, 900.0, 400.0], &|e| {
        e.text = t(
            "Q1. 下列哪一個是\nMoviePy v2 設定位置的寫法？",
            "Q1. Which one sets the position\nof a clip in MoviePy v2?",
        )
        .into();
        e.font_size = 40.0;
        e.animation = Animation::FadeIn;
    });
    add(&mut p, 1, ElementKind::Options, [90.0, 690.0, 900.0, 640.0], &|e| {
        e.options = vec![
            "clip.set_pos(...)".into(),
            "clip.with_position(...)".into(),
            "clip.position = ...".into(),
            "clip.move(...)".into(),
        ];
        e.answer = Some(1);
        e.font_size = 34.0;
        e.start = 0.5;
        e.animation = Animation::SlideLeft;
    });
    add(&mut p, 1, ElementKind::CalloutArrow, [360.0, 1400.0, 400.0, 150.0], &|e| {
        e.text = t("倒數中！", "Counting down!").into();
        e.font_size = 32.0;
        e.start = 1.0;
        e.animation = Animation::Pop;
    });
    add(&mut p, 1, ElementKind::Countdown, [790.0, 1375.0, 200.0, 200.0], &|e| {
        e.text = "5".into();
        e.font_size = 69.0;
        e.start = 1.0;
    });
    add(&mut p, 1, ElementKind::Subtitle, [90.0, 1640.0, 900.0, 110.0], &|e| {
        e.text = t("請在倒數結束前，選出正確答案", "Pick the right answer before time runs out").into();
        e.font_size = 36.0;
    });
    add(&mut p, 1, ElementKind::ProgressBar, [0.0, 1880.0, 1080.0, 40.0], &|e| e.bg_opacity = 0.6);

    p.scenes.push(scene(
        "scene_3",
        t("結尾", "Ending"),
        4.0,
        t(
            "結尾：公布答案、放精華片段、引導訂閱。",
            "Ending: reveal the answer, show highlights, ask viewers to subscribe.",
        ),
        Transition::new(TransitionKind::SlideUp, 0.5),
    ));
    add(&mut p, 2, ElementKind::MediaPlaceholder, [90.0, 200.0, 900.0, 506.0], &|e| {
        e.text = t("精華片段 B-roll", "Highlights B-roll").into();
        e.src = "assets/broll.mp4".into();
    });
    add(&mut p, 2, ElementKind::Title, [90.0, 780.0, 900.0, 150.0], &|e| {
        e.text = t("答案：B", "Answer: B").into();
        e.font_size = 83.0;
        e.animation = Animation::Pop;
    });
    add(&mut p, 2, ElementKind::Subheading, [90.0, 940.0, 900.0, 150.0], &|e| {
        e.text = t("v2 統一用 with_* 方法回傳新的 clip", "v2 uses with_* methods that return a new clip").into();
        e.font_size = 34.0;
    });
    add(&mut p, 2, ElementKind::CtaButton, [270.0, 1150.0, 540.0, 130.0], &|e| {
        e.font_size = 40.0;
        e.start = 1.0;
    });
    add(&mut p, 2, ElementKind::QrCode, [420.0, 1330.0, 240.0, 240.0], &|_| {});
    add(&mut p, 2, ElementKind::Subtitle, [90.0, 1640.0, 900.0, 110.0], &|e| {
        e.text = t("我們下一集見！", "See you in the next episode!").into();
        e.font_size = 36.0;
    });
    p.normalize();
    p
}
