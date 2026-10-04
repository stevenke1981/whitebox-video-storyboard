//! `storyboard.md`: a pure-text description of every scene and element (Traditional
//! Chinese or English, following the current language), so an agent can understand
//! the layout without looking at images.

use crate::draw::R;
use crate::export::Formats;
use crate::i18n::{is_en, t};
use crate::model::{ACTION_SAFE, Animation, Element, ElementKind, Project, Scene, TITLE_SAFE};
use std::fmt::Write;

/// Nine-grid region of the element centre, e.g. `上方中央` / `top centre`.
pub fn region(e: &Element, cw: f32, ch: f32) -> String {
    if e.w >= cw * 0.98 && e.h >= ch * 0.98 {
        return t("滿版（整個畫面）", "full frame").into();
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
    if e.w >= cw * 0.9 {
        let band = [
            t("畫面頂部", "top of the frame"),
            t("畫面中段", "middle of the frame"),
            t("畫面底部", "bottom of the frame"),
        ][row];
        return tf!("{band}，橫跨整個畫面寬度", "{band}, spanning the full width");
    }
    [
        t("左上角區域", "top-left area"),
        t("上方中央", "top centre"),
        t("右上角區域", "top-right area"),
        t("左側中段", "middle left"),
        t("畫面正中央", "centre of the frame"),
        t("右側中段", "middle right"),
        t("左下角區域", "bottom-left area"),
        t("下方中央", "bottom centre"),
        t("右下角區域", "bottom-right area"),
    ][row * 3 + col]
        .to_string()
}

pub fn animation_desc(a: Animation) -> &'static str {
    match a {
        Animation::None => t("無動畫，直接出現／消失", "no animation, appears / disappears instantly"),
        Animation::FadeIn => t("出現時淡入（約 0.5 秒）", "fades in when it appears (about 0.5 s)"),
        Animation::FadeOut => t("結束前淡出（約 0.5 秒）", "fades out before it ends (about 0.5 s)"),
        Animation::FadeInOut => {
            t("出現時淡入、結束前淡出（各約 0.5 秒）", "fades in on appear and out before the end (about 0.5 s each)")
        }
        Animation::SlideUp => t(
            "由下方往上滑入定位並同時淡入（約 0.5 秒）",
            "slides up from below into place while fading in (about 0.5 s)",
        ),
        Animation::SlideLeft => t(
            "由右側往左滑入定位並同時淡入（約 0.5 秒）",
            "slides in from the right into place while fading in (about 0.5 s)",
        ),
        Animation::Pop => t(
            "由小放大彈出（約 0.35 秒，略為超過再回彈）",
            "pops in from small to full size (about 0.35 s, slight overshoot)",
        ),
        Animation::Typewriter => t(
            "文字由左至右逐步顯示（打字機效果，約 1.5 秒）",
            "text is revealed left to right (typewriter, about 1.5 s)",
        ),
        Animation::ScrollUp => t(
            "從畫面下方外側向上捲動，到元件結束時剛好捲出畫面上方（片尾名單）；框的位置表示水平位置與寬度",
            "scrolls from just below the frame to just above it over the element's time (credits roll); the box gives the horizontal position and width",
        ),
    }
}

fn pct(v: f32, total: f32) -> String {
    format!("{:.1}%", v / total * 100.0)
}

fn quote(s: &str) -> String {
    if is_en() {
        format!("“{}”", s.trim().replace('\n', " / "))
    } else {
        format!("「{}」", s.trim().replace('\n', "／"))
    }
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
            legend.push(('·', tf!("`{}`（{}，滿版背景）", "`{}` ({}, full-frame background)", e.id, e.kind.label())));
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
        legend.push((c, tf!("`{}`（{}）", "`{}` ({})", e.id, e.kind.label())));
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
    let label = e.kind.label();
    let sep = t("，", ", ");
    let mut flags = vec![format!("z = {}", e.z)];
    if !e.visible {
        flags.push(t("已隱藏，合成時請略過", "hidden, skip when rendering").into());
    }
    if e.locked {
        flags.push(t("已鎖定", "locked").into());
    }
    let name = if e.name.is_empty() || e.name == info.zh || e.name == info.en {
        String::new()
    } else {
        tf!("「{}」", " “{}”", e.name)
    };
    let _ = writeln!(
        s,
        "{}",
        tf!("#### {}. 【{}】`{}`{}（{}）\n", "#### {}. [{}] `{}`{} ({})\n", n, label, e.id, name, flags.join(sep))
    );
    let r = R::of(e);
    let _ = writeln!(
        s,
        "{}",
        tf!(
            "- **位置**：{}。左上角 ({}, {})，大小 {} × {} px（佔畫面寬 {}、高 {}），中心點 ({}, {})；涵蓋 x {}–{}、y {}–{}。",
            "- **Position**: {}. Top-left ({}, {}), size {} × {} px ({} of the width, {} of the height), centre ({}, {}); covers x {}–{}, y {}–{}.",
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
        )
    );
    let (tx, ty) = (cw * TITLE_SAFE, ch * TITLE_SAFE);
    let full = e.w >= cw * 0.98 && e.h >= ch * 0.98;
    let has_text = e.kind.has_text() && (!e.text.trim().is_empty() || !e.options.is_empty());
    if !full && has_text && (e.x < tx || e.y < ty || r.right() > cw - tx || r.bottom() > ch - ty) {
        let (ax, ay) = (cw * ACTION_SAFE, ch * ACTION_SAFE);
        let inside_action = e.x >= ax && e.y >= ay && r.right() <= cw - ax && r.bottom() <= ch - ay;
        let _ = writeln!(
            s,
            "{}",
            if inside_action {
                t(
                    "- **安全框**：超出 title-safe（10%）範圍，但仍在 action-safe（5%）內。",
                    "- **Safe area**: outside title-safe (10 %) but inside action-safe (5 %).",
                )
            } else {
                t(
                    "- **安全框**：超出 title-safe（10%）範圍，也超出 action-safe（5%），文字可能被裁切。",
                    "- **Safe area**: outside both title-safe (10 %) and action-safe (5 %); text may be cropped.",
                )
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
                        if e.answer == Some(i) {
                            t("（正確答案／強調）", " (correct answer / highlighted)")
                        } else {
                            ""
                        }
                    )
                })
                .collect();
            let _ = writeln!(
                s,
                "{}",
                tf!(
                    "- **內容**：{} 個選項，由上而下等高排列：{}",
                    "- **Content**: {} options stacked top to bottom with equal height: {}",
                    opts.len(),
                    opts.join(t("；", "; "))
                )
            );
            if !e.text.trim().is_empty() {
                let _ = writeln!(s, "{}", tf!("- **題目文字**：{}", "- **Question text**: {}", quote(&e.text)));
            }
        }
        ElementKind::LowerThird => {
            let mut lines = e.text.lines();
            let first = quote(lines.next().unwrap_or(""));
            let rest = quote(&lines.collect::<Vec<_>>().join(" "));
            let _ = writeln!(
                s,
                "{}",
                tf!(
                    "- **內容**：第一行（姓名）{}，第二行（職稱）{}；左側有一條強調色直條。",
                    "- **Content**: line 1 (name) {}, line 2 (role) {}; an accent-coloured bar on the left.",
                    first,
                    rest
                )
            );
        }
        ElementKind::Countdown => {
            let _ = writeln!(
                s,
                "{}",
                tf!(
                    "- **內容**：從 {} 倒數到 0，平均分配在元件顯示時間內。",
                    "- **Content**: counts down from {} to 0, spread evenly over the element's time.",
                    e.text.trim()
                )
            );
        }
        ElementKind::ProgressBar => {
            let _ = writeln!(
                s,
                "{}",
                tf!(
                    "- **內容**：進度條，在元件顯示期間由 0% 填滿到 100%（填色 {}）。",
                    "- **Content**: progress bar filling 0 % → 100 % while visible (fill colour {}).",
                    e.font_color.hex()
                )
            );
        }
        ElementKind::Background => {
            let _ = writeln!(
                s,
                "{}",
                tf!("- **內容**：背景色塊 {}。", "- **Content**: background colour block {}.", e.bg_color.hex())
            );
        }
        _ if !e.text.trim().is_empty() => {
            let _ = writeln!(s, "{}", tf!("- **內容**：文字{}", "- **Content**: text {}", quote(&e.text)));
        }
        _ => {}
    }
    if !e.src.is_empty() {
        let _ = writeln!(
            s,
            "{}",
            tf!(
                "- **素材**：`{}`（請以此檔案取代灰色佔位，等比填滿後裁切到框內）",
                "- **Asset**: `{}` (replace the grey placeholder with this file, cover-fit and crop to the box)",
                e.src
            )
        );
    } else if matches!(
        e.kind,
        ElementKind::MediaPlaceholder | ElementKind::Logo | ElementKind::AvatarFrame | ElementKind::QrCode
    ) {
        let _ = writeln!(
            s,
            "{}",
            t(
                "- **素材**：尚未指定，保留灰色佔位或由 Agent 依備註準備素材。",
                "- **Asset**: not set yet — keep the grey placeholder or prepare an asset from the notes.",
            )
        );
    }
    if has_text {
        let stroke = if e.stroke_width > 0.0 {
            tf!("，描邊 {} px {}", ", stroke {} px {}", e.stroke_width, e.stroke_color.hex())
        } else {
            String::new()
        };
        let _ = writeln!(
            s,
            "{}",
            tf!(
                "- **文字樣式**：字級 {} px，文字顏色 {}，{}對齊（垂直置中）{}。",
                "- **Text style**: {} px, colour {}, aligned {} (vertically centred){}.",
                e.font_size,
                e.font_color.hex(),
                e.align.label().to_lowercase(),
                stroke
            )
        );
    }
    let bg = if e.bg_opacity <= 0.001 {
        t("無底色（透明）", "no background (transparent)").to_string()
    } else {
        tf!("底色 {}，不透明度 {:.0}%", "background {} at {:.0} % opacity", e.bg_color.hex(), e.bg_opacity * 100.0)
    };
    let mut look = vec![bg];
    if matches!(e.kind, ElementKind::Shape | ElementKind::AvatarFrame | ElementKind::Countdown | ElementKind::CtaButton)
    {
        look.push(tf!("形狀：{}", "shape: {}", e.shape.label()));
    }
    if e.kind == ElementKind::CalloutArrow {
        look.push(tf!("箭頭方向：{}", "arrow direction: {}", e.direction.label()));
    }
    let _ = writeln!(s, "{}", tf!("- **外觀**：{}。", "- **Look**: {}.", look.join(sep)));
    let end = e.end_in(scene.duration);
    let _ = writeln!(
        s,
        "{}",
        tf!(
            "- **時間**：場景內 {:.2}–{:.2} 秒（持續 {:.2} 秒{}）；影片絕對時間 {:.2}–{:.2} 秒。",
            "- **Timing**: {:.2}–{:.2} s within the scene ({:.2} s{}); absolute video time {:.2}–{:.2} s.",
            e.start,
            end,
            (end - e.start).max(0.0),
            if e.end.is_none() { t("，到場景結束", ", until the scene ends") } else { "" },
            scene_start + e.start,
            scene_start + end
        )
    );
    let _ = writeln!(
        s,
        "{}",
        tf!("- **動畫**：{}（`{}`）。", "- **Animation**: {} (`{}`).", animation_desc(e.animation), e.animation.key())
    );
    let others: Vec<String> = scene
        .elements
        .iter()
        .filter(|o| o.id != e.id && o.visible && !(o.w >= cw * 0.98 && o.h >= ch * 0.98) && overlaps(e, o))
        .map(|o| {
            format!(
                "`{}`{}",
                o.id,
                if o.z > e.z { t("（在其上方）", " (above it)") } else { t("（在其下方）", " (below it)") }
            )
        })
        .collect();
    if !others.is_empty() && !full {
        let _ = writeln!(s, "{}", tf!("- **重疊**：與 {} 重疊。", "- **Overlaps**: {}.", others.join(t("、", ", "))));
    }
    let _ = writeln!(s, "- **MoviePy**{}{}", t("：", ": "), info.moviepy);
    if !e.notes.trim().is_empty() {
        let _ = writeln!(s, "{}", tf!("- **備註**：{}", "- **Notes**: {}", e.notes.trim()));
    }
    s.push('\n');
}

/// One-line description of the transition into scene `i` (None for hard cuts / first scene).
pub fn transition_line(p: &Project, i: usize) -> Option<String> {
    let sc = &p.scenes[i];
    if i == 0 && sc.transition.kind != crate::model::TransitionKind::FadeBlack || sc.transition.is_none() {
        return None;
    }
    Some(tf!(
        "`{}` {:.2} 秒 — {}（發生在本場景開頭 {:.2} 秒內，不改變時間軸）",
        "`{}` {:.2} s — {} (happens during the first {:.2} s of this scene; timing is unchanged)",
        sc.transition.kind.key(),
        sc.transition.duration,
        sc.transition.kind.describe(),
        sc.transition.duration
    ))
}

pub fn storyboard_md(p: &Project, f: &Formats) -> String {
    let mut s = String::new();
    let (cw, ch) = (p.canvas.width as f32, p.canvas.height as f32);
    let orient = if cw > ch {
        t("橫式", "landscape")
    } else if cw < ch {
        t("直式", "portrait")
    } else {
        t("方形", "square")
    };
    let _ = writeln!(s, "{}", tf!("# 版面文字說明 — {}\n", "# Layout description — {}\n", p.name));
    let _ = writeln!(
        s,
        "{}",
        tf!(
            "> 由 whitebox-video-storyboard {} 產生。本文件以**純文字**描述每個場景與元件的位置、大小、文字、時間與動畫，\n\
             > 讓 AI Agent 不必看草稿圖也能理解版面。精確數值以 `layout.json` 為準。\n",
            "> Generated by whitebox-video-storyboard {}. This document describes, in **plain text**, the position, size, text, timing and\n\
             > animation of every scene and element, so an AI agent can understand the layout without the draft images. `layout.json` has the exact values.\n",
            env!("CARGO_PKG_VERSION")
        )
    );
    let _ = writeln!(s, "{}", t("## 總覽\n", "## Overview\n"));
    let preset = if p.canvas.preset.is_empty() {
        t("自訂比例", "custom ratio").to_string()
    } else {
        p.canvas.preset.clone()
    };
    let _ = writeln!(
        s,
        "{}",
        tf!(
            "- 畫布：{}×{} px（{}，{}），{} fps。\n\
             - 總長 {:.2} 秒，共 {} 個場景，依序播放。\n\
             - 座標：原點在畫面左上角，x 向右、y 向下，單位為像素；元件的 (x, y) 是它的**左上角**，百分比是相對於畫面寬／高。\n\
             - 疊放：z 值越大越上層（後畫的蓋住先畫的）。\n\
             - 轉場：每個場景的「轉場進入」發生在該場景開頭，不會讓場景重疊，也不改變總長。\n\
             - 安全框：title-safe（內縮 10%）為 x {}–{}、y {}–{}，重要文字應放在其中；action-safe（內縮 5%）為 x {}–{}、y {}–{}。\n",
            "- Canvas: {}×{} px ({}, {}), {} fps.\n\
             - Total length {:.2} s, {} scenes played in order.\n\
             - Coordinates: origin at the top-left, x to the right, y downwards, in pixels; an element's (x, y) is its **top-left corner**; percentages are of the frame width / height.\n\
             - Stacking: higher z is drawn on top.\n\
             - Transitions: a scene's \"transition in\" happens at the start of that scene; scenes never overlap and the total length does not change.\n\
             - Safe areas: title-safe (10 % inset) is x {}–{}, y {}–{} — keep important text inside; action-safe (5 % inset) is x {}–{}, y {}–{}.\n",
            p.canvas.width,
            p.canvas.height,
            preset,
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
        )
    );
    let _ = writeln!(s, "{}", t("## 時間軸\n", "## Timeline\n"));
    for (i, sc) in p.scenes.iter().enumerate() {
        let t0 = p.scene_start(i);
        let tr = transition_line(p, i).map(|l| tf!("；轉場進入 {}", "; transition in {}", l)).unwrap_or_default();
        let _ = writeln!(
            s,
            "{}",
            tf!(
                "{}. 場景 {}「{}」：{:.2}–{:.2} 秒（{:.2} 秒），{} 個元件{}。",
                "{}. Scene {} “{}”: {:.2}–{:.2} s ({:.2} s), {} elements{}.",
                i + 1,
                i + 1,
                sc.name,
                t0,
                t0 + sc.duration,
                sc.duration,
                sc.elements.len(),
                tr
            )
        );
    }
    s.push('\n');
    for (i, sc) in p.scenes.iter().enumerate() {
        let t0 = p.scene_start(i);
        let _ =
            writeln!(s, "{}", tf!("## 場景 {}「{}」（`{}`）\n", "## Scene {} “{}” (`{}`)\n", i + 1, sc.name, sc.id));
        let _ = writeln!(
            s,
            "{}",
            tf!(
                "- 時間：影片 {:.2}–{:.2} 秒，長 {:.2} 秒。",
                "- Time: {:.2}–{:.2} s of the video, {:.2} s long.",
                t0,
                t0 + sc.duration,
                sc.duration
            )
        );
        let _ = writeln!(s, "{}", tf!("- 背景：純色 {}。", "- Background: solid {}.", sc.background.hex()));
        if let Some(l) = transition_line(p, i) {
            let _ = writeln!(
                s,
                "{}",
                tf!("- 轉場進入：{}；MoviePy：{}", "- Transition in: {}; MoviePy: {}", l, sc.transition.kind.moviepy())
            );
        }
        if !sc.notes.trim().is_empty() {
            let _ = writeln!(s, "{}", tf!("- 導演備註：{}", "- Director notes: {}", sc.notes.trim()));
        }
        if f.png {
            let _ = writeln!(
                s,
                "{}",
                tf!("- 對應草稿圖：`{}`", "- Draft image: `{}`", crate::export::scene_png_name(i, &sc.id))
            );
        }
        s.push('\n');
        if sc.elements.is_empty() {
            let _ = writeln!(
                s,
                "{}",
                t("（此場景沒有元件，只有背景色。）\n", "(No elements in this scene, background colour only.)\n")
            );
            continue;
        }
        let (sketch, legend) = ascii_sketch(p, sc);
        let _ = writeln!(
            s,
            "{}\n\n```text\n{sketch}\n```\n",
            t(
                "### 版面速寫（文字示意，字母 = 元件，上層蓋住下層）",
                "### Layout sketch (text approximation; letter = element, upper layers cover lower ones)"
            )
        );
        for (c, d) in legend {
            let _ = writeln!(s, "- `{c}` = {d}");
        }
        s.push('\n');
        let _ = writeln!(s, "{}", t("### 元件（由下層到上層）\n", "### Elements (bottom to top)\n"));
        for (n, e) in sc.elements.iter().enumerate() {
            element_md(&mut s, p, sc, t0, n + 1, e);
        }
        // timing summary
        let mut events: Vec<(f32, String)> = vec![];
        for e in sc.elements.iter().filter(|e| e.visible) {
            let end = e.end_in(sc.duration);
            if e.start > 0.0 {
                events.push((e.start, tf!("`{}` 出現（{}）", "`{}` appears ({})", e.id, e.animation.label())));
            }
            if end < sc.duration - 1e-3 {
                events.push((end, tf!("`{}` 消失", "`{}` disappears", e.id)));
            }
        }
        if !events.is_empty() {
            events.sort_by(|a, b| a.0.total_cmp(&b.0));
            let _ = writeln!(s, "{}", t("### 場景內事件順序\n", "### Order of events in the scene\n"));
            let _ = writeln!(
                s,
                "{}",
                t("- 0.00 秒：其餘元件與場景同時出現。", "- 0.00 s: all other elements appear with the scene.")
            );
            for (tm, d) in events {
                let _ = writeln!(s, "{}", tf!("- {:.2} 秒：{}", "- {:.2} s: {}", tm, d));
            }
            s.push('\n');
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use crate::i18n::{Lang, with_lang};

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
        assert!(md.contains("crossfade"));
    }

    #[test]
    fn md_in_english() {
        let md = with_lang(Lang::En, || {
            let p = crate::sample::sample_project("16:9");
            super::storyboard_md(&p, &crate::export::Formats::default())
        });
        assert!(md.contains("# Layout description"));
        assert!(md.contains("Transition in"));
        assert!(!md.contains("場景"), "no Chinese UI words in English output");
    }
}
