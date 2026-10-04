//! Built-in sample project (a three-shot quiz video) used by `--sample`,
//! File ▸ 範例專案 and the tests.

use crate::model::*;

pub fn sample_project(preset_key: &str) -> Project {
    if preset_key == "9:16" {
        return sample_vertical();
    }
    let mut p = Project::new("MoviePy 小測驗（範例）", "16:9");
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
    let mut s = Scene::new("scene_1".into(), "開場".into());
    s.duration = 4.0;
    s.background = Rgb::gray(232);
    s.notes = "開場：標題淡入，主持人自我介紹。".into();
    p.scenes.push(s);
    add(&mut p, 0, ElementKind::Logo, 96.0, 64.0, 220.0, 110.0, &|_| {});
    add(&mut p, 0, ElementKind::Title, 360.0, 330.0, 1200.0, 160.0, &|e| {
        e.text = "三分鐘學會 MoviePy 排版".into();
        e.font_size = 100.0;
        e.start = 0.3;
    });
    add(&mut p, 0, ElementKind::Subheading, 460.0, 500.0, 1000.0, 90.0, &|e| {
        e.text = "白模草稿 → layout.json → 自動合成影片".into();
        e.start = 0.8;
        e.animation = Animation::SlideUp;
    });
    add(&mut p, 0, ElementKind::AvatarFrame, 1500.0, 700.0, 300.0, 300.0, &|e| e.text = "主持人".into());
    add(&mut p, 0, ElementKind::LowerThird, 96.0, 820.0, 720.0, 140.0, &|e| {
        e.text = "Ke Sheng Da\n影片創作者 · 自動化剪輯".into();
        e.start = 1.0;
        e.end = Some(3.8);
    });
    add(&mut p, 0, ElementKind::Watermark, 1464.0, 64.0, 360.0, 60.0, &|_| {});

    // Scene 2 – question
    let mut s = Scene::new("scene_2".into(), "題目".into());
    s.duration = 6.0;
    s.background = Rgb::gray(225);
    s.notes = "出題：選項依序出現，倒數 5 秒，進度條跑完。".into();
    p.scenes.push(s);
    add(&mut p, 1, ElementKind::TextCard, 96.0, 140.0, 800.0, 560.0, &|e| {
        e.text = "Q1. 下列哪一個是\nMoviePy v2 設定位置的寫法？".into();
        e.font_size = 58.0;
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
        e.text = "倒數中！".into();
        e.start = 1.0;
        e.animation = Animation::Pop;
    });
    add(&mut p, 1, ElementKind::Subtitle, 260.0, 900.0, 1400.0, 90.0, &|e| {
        e.text = "請在倒數結束前，選出正確答案".into()
    });
    add(&mut p, 1, ElementKind::ProgressBar, 0.0, 1050.0, 1920.0, 30.0, &|e| e.bg_opacity = 0.6);
    add(&mut p, 1, ElementKind::Sticker, 900.0, 20.0, 120.0, 120.0, &|e| e.text = "？".into());

    // Scene 3 – outro
    let mut s = Scene::new("scene_3".into(), "結尾".into());
    s.duration = 4.0;
    s.background = Rgb::gray(236);
    s.notes = "結尾：公布答案、放精華片段、引導訂閱。".into();
    p.scenes.push(s);
    add(&mut p, 2, ElementKind::MediaPlaceholder, 96.0, 120.0, 960.0, 540.0, &|e| {
        e.text = "精華片段 B-roll".into();
        e.src = "assets/broll.mp4".into();
    });
    add(&mut p, 2, ElementKind::Title, 1120.0, 160.0, 700.0, 130.0, &|e| {
        e.text = "答案：B".into();
        e.font_size = 96.0;
        e.align = Align::Left;
        e.animation = Animation::Pop;
    });
    add(&mut p, 2, ElementKind::Subheading, 1120.0, 300.0, 700.0, 140.0, &|e| {
        e.text = "v2 統一用 with_* 方法回傳新的 clip".into();
        e.align = Align::Left;
        e.font_size = 46.0;
    });
    add(&mut p, 2, ElementKind::CtaButton, 1120.0, 500.0, 440.0, 110.0, &|e| e.start = 1.0);
    add(&mut p, 2, ElementKind::QrCode, 1600.0, 470.0, 220.0, 220.0, &|_| {});
    add(&mut p, 2, ElementKind::Shape, 96.0, 720.0, 960.0, 12.0, &|e| {
        e.bg_color = Rgb([250, 180, 40]);
        e.name = "裝飾線".into();
    });
    add(&mut p, 2, ElementKind::Subtitle, 260.0, 900.0, 1400.0, 90.0, &|e| e.text = "我們下一集見！".into());

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
    let mut p = Project::new("MoviePy 小測驗（直式範例）", "9:16");
    p.scenes.clear();
    let (cw, ch) = (1080.0, 1920.0);
    type Setup<'a> = &'a dyn Fn(&mut Element);
    let add = |p: &mut Project, si: usize, kind: ElementKind, r: [f32; 4], f: Setup| {
        let mut e = Element::new(kind, p.next_element_id(kind), cw, ch, 0.0, 0.0);
        [e.x, e.y, e.w, e.h] = r;
        f(&mut e);
        p.scenes[si].elements.push(e);
    };
    let scene = |id: &str, name: &str, dur: f32, notes: &str| {
        let mut s = Scene::new(id.into(), name.into());
        s.duration = dur;
        s.background = Rgb::gray(232);
        s.notes = notes.into();
        s
    };

    p.scenes.push(scene("scene_1", "開場", 4.0, "開場：標題淡入，主持人自我介紹。"));
    add(&mut p, 0, ElementKind::Logo, [70.0, 90.0, 200.0, 100.0], &|_| {});
    add(&mut p, 0, ElementKind::Watermark, [690.0, 110.0, 320.0, 60.0], &|_| {});
    add(&mut p, 0, ElementKind::Title, [90.0, 520.0, 900.0, 300.0], &|e| {
        e.text = "三分鐘學會\nMoviePy 排版".into();
        e.font_size = 110.0;
        e.start = 0.3;
    });
    add(&mut p, 0, ElementKind::Subheading, [110.0, 840.0, 860.0, 90.0], &|e| {
        e.text = "白模草稿 → 自動合成影片".into();
        e.font_size = 52.0;
        e.start = 0.8;
        e.animation = Animation::SlideUp;
    });
    add(&mut p, 0, ElementKind::AvatarFrame, [340.0, 1060.0, 400.0, 400.0], &|e| e.text = "主持人".into());
    add(&mut p, 0, ElementKind::LowerThird, [90.0, 1560.0, 760.0, 150.0], &|e| {
        e.text = "Ke Sheng Da\n影片創作者 · 自動化剪輯".into();
        e.font_size = 46.0;
        e.start = 1.0;
        e.end = Some(3.8);
    });

    p.scenes.push(scene("scene_2", "題目", 6.0, "出題：選項依序出現，倒數 5 秒，進度條跑完。"));
    add(&mut p, 1, ElementKind::Sticker, [880.0, 80.0, 130.0, 130.0], &|e| e.text = "？".into());
    add(&mut p, 1, ElementKind::TextCard, [90.0, 230.0, 900.0, 400.0], &|e| {
        e.text = "Q1. 下列哪一個是\nMoviePy v2 設定位置的寫法？".into();
        e.font_size = 58.0;
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
        e.font_size = 50.0;
        e.start = 0.5;
        e.animation = Animation::SlideLeft;
    });
    add(&mut p, 1, ElementKind::CalloutArrow, [360.0, 1400.0, 400.0, 150.0], &|e| {
        e.text = "倒數中！".into();
        e.font_size = 46.0;
        e.start = 1.0;
        e.animation = Animation::Pop;
    });
    add(&mut p, 1, ElementKind::Countdown, [790.0, 1375.0, 200.0, 200.0], &|e| {
        e.text = "5".into();
        e.font_size = 100.0;
        e.start = 1.0;
    });
    add(&mut p, 1, ElementKind::Subtitle, [90.0, 1640.0, 900.0, 110.0], &|e| {
        e.text = "請在倒數結束前，選出正確答案".into();
        e.font_size = 52.0;
    });
    add(&mut p, 1, ElementKind::ProgressBar, [0.0, 1880.0, 1080.0, 40.0], &|e| e.bg_opacity = 0.6);

    p.scenes.push(scene("scene_3", "結尾", 4.0, "結尾：公布答案、放精華片段、引導訂閱。"));
    add(&mut p, 2, ElementKind::MediaPlaceholder, [90.0, 200.0, 900.0, 506.0], &|e| {
        e.text = "精華片段 B-roll".into();
        e.src = "assets/broll.mp4".into();
    });
    add(&mut p, 2, ElementKind::Title, [90.0, 780.0, 900.0, 150.0], &|e| {
        e.text = "答案：B".into();
        e.font_size = 120.0;
        e.animation = Animation::Pop;
    });
    add(&mut p, 2, ElementKind::Subheading, [90.0, 940.0, 900.0, 150.0], &|e| {
        e.text = "v2 統一用 with_* 方法回傳新的 clip".into();
        e.font_size = 50.0;
    });
    add(&mut p, 2, ElementKind::CtaButton, [270.0, 1150.0, 540.0, 130.0], &|e| {
        e.font_size = 58.0;
        e.start = 1.0;
    });
    add(&mut p, 2, ElementKind::QrCode, [420.0, 1330.0, 240.0, 240.0], &|_| {});
    add(&mut p, 2, ElementKind::Subtitle, [90.0, 1640.0, 900.0, 110.0], &|e| {
        e.text = "我們下一集見！".into();
        e.font_size = 52.0;
    });
    p.normalize();
    p
}
