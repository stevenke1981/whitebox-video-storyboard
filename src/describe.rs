//! `storyboard.md`: a pure-text (Traditional Chinese) description of every scene
//! and element, so an agent can understand the layout without looking at images.

use crate::draw::R;
use crate::export::Formats;
use crate::model::{ACTION_SAFE, Animation, Element, ElementKind, Project, Scene, TITLE_SAFE};
use std::fmt::Write;

/// Nine-grid region of the element centre, e.g. `上方中央`.
pub fn region(e: &Element, cw: f32, ch: f32) -> String {
    if e.w >= cw * 0.98 && e.h >= ch * 0.98 {
        return "滿版（整個畫面）".into();
    }
    let (cx, cy) = (e.x + e.w / 2.0, e.y + e.h / 2.0);
    let col = if cx < cw / 3.0 {
        0
    } else if cx < cw * 2.0 / 3.0 {
        1
    } else {
        2
    };
    let row = if cy < ch / 3.0 {
        0
    } else if cy < ch * 2.0 / 3.0 {
        1
    } else {
        2
    };
    let base = [
        "左上角區域",
        "上方中央",
        "右上角區域",
        "左側中段",
        "畫面正中央",
        "右側中段",
        "左下角區域",
        "下方中央",
        "右下角區域",
    ][row * 3 + col];
    if e.w >= cw * 0.9 {
        let band = ["畫面頂部", "畫面中段", "畫面底部"][row];
        return format!("{band}，橫跨整個畫面寬度");
    }
    base.to_string()
}

pub fn animation_desc(a: Animation) -> &'static str {
    match a {
        Animation::None => "無動畫，直接出現／消失",
        Animation::FadeIn => "出現時淡入（約 0.5 秒）",
        Animation::FadeOut => "結束前淡出（約 0.5 秒）",
        Animation::FadeInOut => "出現時淡入、結束前淡出（各約 0.5 秒）",
        Animation::SlideUp => "由下方往上滑入定位並同時淡入（約 0.5 秒）",
        Animation::SlideLeft => "由右側往左滑入定位並同時淡入（約 0.5 秒）",
        Animation::Pop => "由小放大彈出（約 0.35 秒，略為超過再回彈）",
        Animation::Typewriter => "文字由左至右逐步顯示（打字機效果，約 1.5 秒）",
    }
}

fn pct(v: f32, total: f32) -> String {
    format!("{:.1}%", v / total * 100.0)
}

fn quote(s: &str) -> String {
    format!("「{}」", s.trim().replace('\n', "／"))
}

fn overlaps(a: &Element, b: &Element) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

/// Coarse ASCII sketch of a scene: each element is drawn with its legend letter.
pub fn ascii_sketch(p: &Project, s: &Scene) -> (String, Vec<(char, String)>) {
    let (cw, ch) = (p.canvas.width as f32, p.canvas.height as f32);
    let cols: usize = if cw >= ch { 64 } else { 36 };
    let rows: usize = ((cols as f32 * ch / cw) / 2.0).round().max(6.0) as usize;
    let mut grid = vec![vec!['·'; cols]; rows];
    let mut legend = vec![];
    let letters: Vec<char> = ('A'..='Z').chain('a'..='z').chain('0'..='9').collect();
    let mut n = 0;
    for e in s.elements.iter().filter(|e| e.visible) {
        let full = e.w >= cw * 0.98 && e.h >= ch * 0.98;
        if full {
            legend.push(('·', format!("`{}`（{}，滿版背景）", e.id, e.kind.info().zh)));
            continue;
        }
        let c = letters[n % letters.len()];
        n += 1;
        let x0 = ((e.x / cw) * cols as f32).floor().clamp(0.0, cols as f32 - 1.0) as usize;
        let x1 = (((e.x + e.w) / cw) * cols as f32).ceil().clamp(x0 as f32 + 1.0, cols as f32) as usize;
        let y0 = ((e.y / ch) * rows as f32).floor().clamp(0.0, rows as f32 - 1.0) as usize;
        let y1 = (((e.y + e.h) / ch) * rows as f32).ceil().clamp(y0 as f32 + 1.0, rows as f32) as usize;
        for row in grid.iter_mut().take(y1).skip(y0) {
            for cell in row.iter_mut().take(x1).skip(x0) {
                *cell = c;
            }
        }
        legend.push((c, format!("`{}`（{}）", e.id, e.kind.info().zh)));
    }
    let mut out = String::new();
    out.push('+');
    out.push_str(&"-".repeat(cols));
    out.push_str("+\n");
    for row in grid {
        out.push('|');
        out.extend(row);
        out.push_str("|\n");
    }
    out.push('+');
    out.push_str(&"-".repeat(cols));
    out.push('+');
    (out, legend)
}

pub fn element_md(s: &mut String, p: &Project, scene: &Scene, scene_start: f32, n: usize, e: &Element) {
    let (cw, ch) = (p.canvas.width as f32, p.canvas.height as f32);
    let info = e.kind.info();
    let mut flags = vec![format!("z = {}", e.z)];
    if !e.visible {
        flags.push("已隱藏，合成時請略過".into());
    }
    if e.locked {
        flags.push("已鎖定".into());
    }
    let name = if e.name.is_empty() || e.name == info.zh { String::new() } else { format!("「{}」", e.name) };
    let _ = writeln!(s, "#### {}. 【{}】`{}`{}（{}）\n", n, info.zh, e.id, name, flags.join("，"));
    let r = R::of(e);
    let _ = writeln!(
        s,
        "- **位置**：{}。左上角 ({}, {})，大小 {} × {} px（佔畫面寬 {}、高 {}），中心點 ({}, {})；涵蓋 x {}–{}、y {}–{}。",
        region(e, cw, ch),
        e.x,
        e.y,
        e.w,
        e.h,
        pct(e.w, cw),
        pct(e.h, ch),
        r.cx(),
        r.cy(),
        e.x,
        r.right(),
        e.y,
        r.bottom()
    );
    let (tx, ty) = (cw * TITLE_SAFE, ch * TITLE_SAFE);
    let full = e.w >= cw * 0.98 && e.h >= ch * 0.98;
    let has_text = e.kind.has_text() && (!e.text.trim().is_empty() || !e.options.is_empty());
    if !full && has_text && (e.x < tx || e.y < ty || r.right() > cw - tx || r.bottom() > ch - ty) {
        let (ax, ay) = (cw * ACTION_SAFE, ch * ACTION_SAFE);
        let inside_action = e.x >= ax && e.y >= ay && r.right() <= cw - ax && r.bottom() <= ch - ay;
        let _ = writeln!(
            s,
            "- **安全框**：超出 title-safe（10%）範圍{}。",
            if inside_action {
                "，但仍在 action-safe（5%）內"
            } else {
                "，也超出 action-safe（5%），文字可能被裁切"
            }
        );
    }
    // content
    match e.kind {
        ElementKind::Options => {
            let opts: Vec<String> = e
                .options
                .iter()
                .enumerate()
                .map(|(i, o)| {
                    format!(
                        "{}. {}{}",
                        (b'A' + (i as u8 % 26)) as char,
                        o,
                        if e.answer == Some(i) { "（正確答案／強調）" } else { "" }
                    )
                })
                .collect();
            let _ = writeln!(s, "- **內容**：{} 個選項，由上而下等高排列：{}", opts.len(), opts.join("；"));
            if !e.text.trim().is_empty() {
                let _ = writeln!(s, "- **題目文字**：{}", quote(&e.text));
            }
        }
        ElementKind::LowerThird => {
            let mut lines = e.text.lines();
            let _ = writeln!(
                s,
                "- **內容**：第一行（姓名）{}，第二行（職稱）{}；左側有一條強調色直條。",
                quote(lines.next().unwrap_or("")),
                quote(&lines.collect::<Vec<_>>().join(" "))
            );
        }
        ElementKind::Countdown => {
            let _ = writeln!(s, "- **內容**：從 {} 倒數到 0，平均分配在元件顯示時間內。", e.text.trim());
        }
        ElementKind::ProgressBar => {
            let _ = writeln!(s, "- **內容**：進度條，在元件顯示期間由 0% 填滿到 100%（填色 {}）。", e.font_color.hex());
        }
        ElementKind::Background => {
            let _ = writeln!(s, "- **內容**：背景色塊 {}。", e.bg_color.hex());
        }
        _ if !e.text.trim().is_empty() => {
            let _ = writeln!(s, "- **內容**：文字{}", quote(&e.text));
        }
        _ => {}
    }
    if !e.src.is_empty() {
        let _ = writeln!(s, "- **素材**：`{}`（請以此檔案取代灰色佔位，等比填滿後裁切到框內）", e.src);
    } else if matches!(
        e.kind,
        ElementKind::MediaPlaceholder | ElementKind::Logo | ElementKind::AvatarFrame | ElementKind::QrCode
    ) {
        let _ = writeln!(s, "- **素材**：尚未指定，保留灰色佔位或由 Agent 依備註準備素材。");
    }
    if has_text {
        let align = e.align.zh();
        let stroke = if e.stroke_width > 0.0 {
            format!("，描邊 {} px {}", e.stroke_width, e.stroke_color.hex())
        } else {
            String::new()
        };
        let _ = writeln!(
            s,
            "- **文字樣式**：字級 {} px，文字顏色 {}，{}對齊（垂直置中）{}。",
            e.font_size,
            e.font_color.hex(),
            align,
            stroke
        );
    }
    let bg = if e.bg_opacity <= 0.001 {
        "無底色（透明）".to_string()
    } else {
        format!("底色 {}，不透明度 {:.0}%", e.bg_color.hex(), e.bg_opacity * 100.0)
    };
    let mut look = vec![bg];
    if matches!(e.kind, ElementKind::Shape | ElementKind::AvatarFrame | ElementKind::Countdown | ElementKind::CtaButton)
    {
        look.push(format!("形狀：{}", e.shape.zh()));
    }
    if e.kind == ElementKind::CalloutArrow {
        look.push(format!("箭頭方向：{}", e.direction.zh()));
    }
    let _ = writeln!(s, "- **外觀**：{}。", look.join("，"));
    let end = e.end_in(scene.duration);
    let _ = writeln!(
        s,
        "- **時間**：場景內 {:.2}–{:.2} 秒（持續 {:.2} 秒{}）；影片絕對時間 {:.2}–{:.2} 秒。",
        e.start,
        end,
        (end - e.start).max(0.0),
        if e.end.is_none() { "，到場景結束" } else { "" },
        scene_start + e.start,
        scene_start + end
    );
    let _ = writeln!(s, "- **動畫**：{}（`{}`）。", animation_desc(e.animation), e.animation.key());
    let others: Vec<String> = scene
        .elements
        .iter()
        .filter(|o| o.id != e.id && o.visible && !(o.w >= cw * 0.98 && o.h >= ch * 0.98) && overlaps(e, o))
        .map(|o| format!("`{}`{}", o.id, if o.z > e.z { "（在其上方）" } else { "（在其下方）" }))
        .collect();
    if !others.is_empty() && !full {
        let _ = writeln!(s, "- **重疊**：與 {} 重疊。", others.join("、"));
    }
    let _ = writeln!(s, "- **MoviePy**：{}", info.moviepy);
    if !e.notes.trim().is_empty() {
        let _ = writeln!(s, "- **備註**：{}", e.notes.trim());
    }
    s.push('\n');
}

pub fn storyboard_md(p: &Project, f: &Formats) -> String {
    let mut s = String::new();
    let (cw, ch) = (p.canvas.width as f32, p.canvas.height as f32);
    let orient = if cw > ch {
        "橫式"
    } else if cw < ch {
        "直式"
    } else {
        "方形"
    };
    let _ = writeln!(s, "# 版面文字說明 — {}\n", p.name);
    let _ = writeln!(
        s,
        "> 由 whitebox-video-storyboard {} 產生。本文件以**純文字**描述每個場景與元件的位置、大小、文字、時間與動畫，\n\
         > 讓 AI Agent 不必看草稿圖也能理解版面。精確數值以 `layout.json` 為準。\n",
        env!("CARGO_PKG_VERSION")
    );
    let _ = writeln!(s, "## 總覽\n");
    let _ = writeln!(
        s,
        "- 畫布：{}×{} px（{}，{}），{} fps。\n\
         - 總長 {:.2} 秒，共 {} 個場景，依序播放。\n\
         - 座標：原點在畫面左上角，x 向右、y 向下，單位為像素；元件的 (x, y) 是它的**左上角**，百分比是相對於畫面寬／高。\n\
         - 疊放：z 值越大越上層（後畫的蓋住先畫的）。\n\
         - 安全框：title-safe（內縮 10%）為 x {}–{}、y {}–{}，重要文字應放在其中；action-safe（內縮 5%）為 x {}–{}、y {}–{}。\n",
        p.canvas.width,
        p.canvas.height,
        if p.canvas.preset.is_empty() { "自訂比例" } else { &p.canvas.preset },
        orient,
        p.fps,
        p.total_duration(),
        p.scenes.len(),
        cw * TITLE_SAFE,
        cw * (1.0 - TITLE_SAFE),
        ch * TITLE_SAFE,
        ch * (1.0 - TITLE_SAFE),
        cw * ACTION_SAFE,
        cw * (1.0 - ACTION_SAFE),
        ch * ACTION_SAFE,
        ch * (1.0 - ACTION_SAFE),
    );
    let _ = writeln!(s, "## 時間軸\n");
    for (i, sc) in p.scenes.iter().enumerate() {
        let t0 = p.scene_start(i);
        let _ = writeln!(
            s,
            "{}. 場景 {}「{}」：{:.2}–{:.2} 秒（{:.2} 秒），{} 個元件。",
            i + 1,
            i + 1,
            sc.name,
            t0,
            t0 + sc.duration,
            sc.duration,
            sc.elements.len()
        );
    }
    s.push('\n');
    for (i, sc) in p.scenes.iter().enumerate() {
        let t0 = p.scene_start(i);
        let _ = writeln!(s, "## 場景 {}「{}」（`{}`）\n", i + 1, sc.name, sc.id);
        let _ = writeln!(s, "- 時間：影片 {:.2}–{:.2} 秒，長 {:.2} 秒。", t0, t0 + sc.duration, sc.duration);
        let _ = writeln!(s, "- 背景：純色 {}。", sc.background.hex());
        if !sc.notes.trim().is_empty() {
            let _ = writeln!(s, "- 導演備註：{}", sc.notes.trim());
        }
        if f.png {
            let _ = writeln!(s, "- 對應草稿圖：`{}`", crate::export::scene_png_name(i, &sc.id));
        }
        s.push('\n');
        if sc.elements.is_empty() {
            let _ = writeln!(s, "（此場景沒有元件，只有背景色。）\n");
            continue;
        }
        let (sketch, legend) = ascii_sketch(p, sc);
        let _ = writeln!(s, "### 版面速寫（文字示意，字母 = 元件，上層蓋住下層）\n\n```text\n{sketch}\n```\n");
        for (c, d) in legend {
            let _ = writeln!(s, "- `{c}` = {d}");
        }
        s.push('\n');
        let _ = writeln!(s, "### 元件（由下層到上層）\n");
        for (n, e) in sc.elements.iter().enumerate() {
            element_md(&mut s, p, sc, t0, n + 1, e);
        }
        // timing summary
        let mut events: Vec<(f32, String)> = vec![];
        for e in sc.elements.iter().filter(|e| e.visible) {
            let end = e.end_in(sc.duration);
            if e.start > 0.0 {
                events.push((e.start, format!("`{}` 出現（{}）", e.id, e.animation.zh())));
            }
            if end < sc.duration - 1e-3 {
                events.push((end, format!("`{}` 消失", e.id)));
            }
        }
        if !events.is_empty() {
            events.sort_by(|a, b| a.0.total_cmp(&b.0));
            let _ = writeln!(s, "### 場景內事件順序\n");
            let _ = writeln!(s, "- 0.00 秒：其餘元件與場景同時出現。");
            for (t, d) in events {
                let _ = writeln!(s, "- {t:.2} 秒：{d}");
            }
            s.push('\n');
        }
    }
    s
}

#[cfg(test)]
mod tests {
    #[test]
    fn md_mentions_every_element() {
        let p = crate::sample::sample_project("16:9");
        let md = super::storyboard_md(&p, &crate::export::Formats::default());
        for s in &p.scenes {
            for e in &s.elements {
                assert!(md.contains(&format!("`{}`", e.id)), "{}", e.id);
            }
        }
        assert!(md.contains("版面速寫"));
    }
}
