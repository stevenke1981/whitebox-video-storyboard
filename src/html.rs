//! `storyboard.html`: a self-contained storyboard page (scene images embedded as
//! base64 PNG, element overlays, tables, timing bars, agent notes, MoviePy hints).

use crate::describe::{animation_desc, region};
use crate::export::{Formats, scene_png_name};
use crate::i18n::t;
use crate::model::{ElementKind, Project};
use std::fmt::Write;

pub fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let b = [c[0], *c.get(1).unwrap_or(&0), *c.get(2).unwrap_or(&0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&#39;"),
            '\n' => o.push_str("<br>"),
            c => o.push(c),
        }
    }
    o
}

/// Escape for an attribute value (newlines kept as real line breaks in tooltips).
fn attr(s: &str) -> String {
    esc(s).replace("<br>", "&#10;")
}

const CSS: &str = r#"
:root{--bg:#f4f4f2;--card:#fff;--ink:#222;--muted:#777;--accent:#f08014;--line:#ddd}
*{box-sizing:border-box}body{margin:0;font-family:"Noto Sans TC","Noto Sans CJK TC","Microsoft JhengHei","PingFang TC",system-ui,sans-serif;background:var(--bg);color:var(--ink);line-height:1.55}
header{background:#24262b;color:#eee;padding:22px 32px}header h1{margin:0 0 4px;font-size:26px}header .meta{color:#bbb;font-size:14px}
main{max-width:1500px;margin:0 auto;padding:20px 28px 60px}
h2{margin:0;font-size:21px}h3{font-size:16px;margin:18px 0 8px}
.card{background:var(--card);border:1px solid var(--line);border-radius:10px;padding:18px 20px;margin:18px 0;box-shadow:0 1px 3px rgba(0,0,0,.05)}
.timeline{display:flex;height:46px;border-radius:8px;overflow:hidden;border:1px solid var(--line)}
.timeline a{display:flex;align-items:center;justify-content:center;color:#222;text-decoration:none;font-size:13px;border-right:2px solid #fff;padding:0 6px;white-space:nowrap;overflow:hidden}
.timeline a:nth-child(odd){background:#ffe2c2}.timeline a:nth-child(even){background:#d9e6f7}.timeline a:hover{filter:brightness(.93)}
.scene-head{display:flex;gap:14px;align-items:baseline;flex-wrap:wrap}.scene-head .t{color:var(--muted);font-size:14px}
.notes{background:#fff8e6;border-left:4px solid #f5b400;padding:8px 12px;margin:10px 0;border-radius:4px}
.grid{display:grid;grid-template-columns:minmax(320px,1.15fr) minmax(300px,1fr);gap:20px;align-items:start}
@media(max-width:1000px){.grid{grid-template-columns:1fr}}
.frame{position:relative;width:100%;border:1px solid #bbb;background:#eee}.frame img{display:block;width:100%;height:100%}
.frame .box{position:absolute;border:2px solid transparent;cursor:pointer}.frame .box:hover,.frame .box.hl{border-color:var(--accent);background:rgba(240,128,20,.12)}
.gantt{font-size:12px}.gantt .row{display:grid;grid-template-columns:150px 1fr;gap:8px;align-items:center;margin:3px 0}
.gantt .lane{position:relative;height:16px;background:#f0f0f0;border-radius:3px}.gantt .bar{position:absolute;top:0;bottom:0;background:#8fb3e8;border-radius:3px}
.gantt .bar.anim{background:linear-gradient(90deg,#f0a050,#8fb3e8 30%)}.gantt .id{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
table{width:100%;border-collapse:collapse;font-size:13px}th,td{border-bottom:1px solid #eee;padding:6px 7px;text-align:left;vertical-align:top}th{white-space:nowrap}td:last-child{min-width:7em}
th{background:#fafafa;position:sticky;top:0}tr.hl td{background:#fff1e0}td.num{font-variant-numeric:tabular-nums;white-space:nowrap}
code,pre{font-family:ui-monospace,Consolas,monospace;font-size:12.5px}pre{background:#1e1f23;color:#e6e6e6;padding:12px 14px;border-radius:8px;overflow:auto}
.chip{display:inline-block;padding:1px 7px;border-radius:10px;background:#eee;font-size:12px;margin-right:4px}.sw{display:inline-block;width:12px;height:12px;border:1px solid #999;vertical-align:-2px;margin-right:3px}
.muted{color:var(--muted)}footer{color:var(--muted);font-size:13px;text-align:center;margin-top:30px}
"#;

const JS: &str = r#"
document.querySelectorAll('[data-el]').forEach(n=>{const id=n.dataset.el;
n.addEventListener('mouseenter',()=>document.querySelectorAll('[data-el="'+id+'"]').forEach(m=>m.classList.add('hl')));
n.addEventListener('mouseleave',()=>document.querySelectorAll('[data-el="'+id+'"]').forEach(m=>m.classList.remove('hl')));});
"#;

/// Build the page. `pngs[i]` = PNG bytes of scene `i` (may be empty → no image).
pub fn storyboard_html(p: &Project, pngs: &[Vec<u8>], f: &Formats) -> String {
    let (cw, ch) = (p.canvas.width as f32, p.canvas.height as f32);
    let total = p.total_duration().max(0.001);
    let mut h = String::new();
    let _ = write!(
        h,
        "<!doctype html>\n<html lang=\"{}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <title>{} — {}</title><style>{CSS}</style></head><body>\n",
        t("zh-Hant", "en"),
        esc(&p.name),
        t("分鏡", "storyboard")
    );
    let preset = if p.canvas.preset.is_empty() { t("自訂", "custom").to_string() } else { p.canvas.preset.clone() };
    let meta = tf!(
        "{}×{}（{}）· {} fps · 總長 {:.2} 秒 · {} 個場景 · 由 whitebox-video-storyboard {} 產生",
        "{}×{} ({}) · {} fps · {:.2} s total · {} scenes · generated by whitebox-video-storyboard {}",
        p.canvas.width,
        p.canvas.height,
        esc(&preset),
        p.fps,
        p.total_duration(),
        p.scenes.len(),
        env!("CARGO_PKG_VERSION")
    );
    let _ = writeln!(h, "<header><h1>{}</h1><div class=\"meta\">{meta}</div></header><main>", esc(&p.name));

    // timeline
    let _ = write!(
        h,
        "<div class=\"card\"><h3 style=\"margin-top:0\">{}</h3><div class=\"timeline\">",
        t("時間軸", "Timeline")
    );
    for (i, s) in p.scenes.iter().enumerate() {
        let t0 = p.scene_start(i);
        let _ = write!(
            h,
            "<a href=\"#{}\" style=\"width:{:.3}%\" title=\"{:.2}–{:.2}s\">#{} {} · {:.1}s</a>",
            esc(&s.id),
            s.duration / total * 100.0,
            t0,
            t0 + s.duration,
            i + 1,
            esc(&s.name),
            s.duration
        );
    }
    let _ = writeln!(
        h,
        "</div><p class=\"muted\" style=\"margin:8px 0 0\">{}</p></div>",
        t(
            "座標以目標解析度像素表示，原點在左上角，(x, y) 為元件左上角；時間「場景內」為相對於場景起點的秒數。轉場發生在場景開頭，不改變時間軸。滑鼠移到圖上方框或表格列可互相對照。",
            "Coordinates are pixels at the target resolution, origin top-left, (x, y) = the element's top-left corner; \"in scene\" times are seconds from the scene start. Transitions happen at the start of a scene and do not change the timeline. Hover a box on the image or a table row to cross-highlight."
        )
    );

    for (i, s) in p.scenes.iter().enumerate() {
        let t0 = p.scene_start(i);
        let head = tf!(
            "<h2>場景 {} · {}</h2><span class=\"t\">影片 {:.2}–{:.2} 秒（{:.2} 秒）· 背景 <span class=\"sw\" style=\"background:{}\"></span>{} · {} 個元件{}</span>",
            "<h2>Scene {} · {}</h2><span class=\"t\">video {:.2}–{:.2} s ({:.2} s) · background <span class=\"sw\" style=\"background:{}\"></span>{} · {} elements{}</span>",
            i + 1,
            esc(&s.name),
            t0,
            t0 + s.duration,
            s.duration,
            s.background.hex(),
            s.background.hex(),
            s.elements.len(),
            if f.png { format!(" · <code>{}</code>", scene_png_name(i, &s.id)) } else { String::new() }
        );
        let _ = writeln!(h, "<section class=\"card\" id=\"{}\"><div class=\"scene-head\">{head}</div>", esc(&s.id));
        if let Some(l) = crate::describe::transition_line(p, i) {
            let _ = write!(
                h,
                "<div class=\"notes\" style=\"background:#eaf1fd;border-color:#3d6fd0\"><b>{}</b>{}<br><code>{}</code></div>",
                t("轉場進入：", "Transition in: "),
                esc(&l).replace('`', ""),
                esc(s.transition.kind.moviepy())
            );
        }
        if !s.notes.trim().is_empty() {
            let _ = write!(
                h,
                "<div class=\"notes\"><b>{}</b>{}</div>",
                t("導演 / Agent 備註：", "Director / agent notes: "),
                esc(s.notes.trim())
            );
        }
        h.push_str("<div class=\"grid\"><div>");
        let _ = write!(h, "<div class=\"frame\" style=\"aspect-ratio:{} / {}\">", p.canvas.width, p.canvas.height);
        if let Some(png) = pngs.get(i).filter(|b| !b.is_empty()) {
            let _ = write!(h, "<img alt=\"{}\" src=\"data:image/png;base64,{}\">", esc(&s.name), base64(png));
        }
        for e in s.elements.iter().filter(|e| e.visible) {
            let _ = write!(
                h,
                "<div class=\"box\" data-el=\"{}\" title=\"{} · {}\n{}\" style=\"left:{:.3}%;top:{:.3}%;width:{:.3}%;height:{:.3}%\"></div>",
                esc(&e.id),
                esc(&e.id),
                esc(e.kind.label()),
                attr(&e.text),
                e.x / cw * 100.0,
                e.y / ch * 100.0,
                e.w / cw * 100.0,
                e.h / ch * 100.0
            );
        }
        let _ = write!(
            h,
            "</div></div><div><h3 style=\"margin-top:0\">{}</h3><div class=\"gantt\">",
            t("元件時間（場景內）", "Element timing (in scene)")
        );
        let d = s.duration.max(0.001);
        for e in s.elements.iter().rev() {
            let end = e.end_in(s.duration);
            let _ = write!(
                h,
                "<div class=\"row\" data-el=\"{}\"><div class=\"id\" title=\"{}\">{} <span class=\"muted\">{}</span></div><div class=\"lane\"><div class=\"bar{}\" style=\"left:{:.2}%;width:{:.2}%\" title=\"{:.2}–{:.2}s · {}\"></div></div></div>",
                esc(&e.id),
                esc(&e.id),
                esc(&e.id),
                esc(e.kind.label()),
                if e.animation == crate::model::Animation::None { "" } else { " anim" },
                e.start / d * 100.0,
                ((end - e.start).max(0.0)) / d * 100.0,
                e.start,
                end,
                e.animation.key()
            );
        }
        h.push_str("</div></div></div>\n");

        // element table
        h.push_str(t(
            "<h3>元件明細（由上層到下層）</h3><div style=\"overflow:auto\"><table><thead><tr><th>z</th><th>id / 類型</th><th>文字 / 內容</th><th>位置與大小</th><th>時間（場景內）</th><th>動畫</th><th>樣式</th><th>MoviePy 提示</th><th>備註</th></tr></thead><tbody>",
            "<h3>Elements (top to bottom)</h3><div style=\"overflow:auto\"><table><thead><tr><th>z</th><th>id / type</th><th>Text / content</th><th>Position &amp; size</th><th>Time (in scene)</th><th>Animation</th><th>Style</th><th>MoviePy hint</th><th>Notes</th></tr></thead><tbody>",
        ));
        for e in s.elements.iter().rev() {
            let info = e.kind.info();
            let mut content = esc(&e.text);
            if e.kind == ElementKind::Options {
                content = e
                    .options
                    .iter()
                    .enumerate()
                    .map(|(j, o)| {
                        format!(
                            "{}. {}{}",
                            (b'A' + (j as u8 % 26)) as char,
                            esc(o),
                            if e.answer == Some(j) { " ✔" } else { "" }
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("<br>");
            }
            if !e.src.is_empty() {
                content.push_str(&format!("<br><span class=\"chip\">src</span><code>{}</code>", esc(&e.src)));
            }
            let end = e.end_in(s.duration);
            let style = format!(
                "{}<span class=\"sw\" style=\"background:{}\"></span>{} px {}<br>{} <span class=\"sw\" style=\"background:{};opacity:{:.2}\"></span>{} · {:.0}%{}",
                if e.kind.has_text() {
                    ""
                } else {
                    t("<span class=\"muted\">（無文字）</span><br>", "<span class=\"muted\">(no text)</span><br>")
                },
                e.font_color.hex(),
                e.font_size,
                e.align.label(),
                t("底", "bg"),
                e.bg_color.hex(),
                e.bg_opacity.max(0.15),
                e.bg_color.hex(),
                e.bg_opacity * 100.0,
                if e.stroke_width > 0.0 {
                    tf!("<br>描邊 {} px {}", "<br>stroke {} px {}", e.stroke_width, e.stroke_color.hex())
                } else {
                    String::new()
                }
            );
            let _ = write!(
                h,
                "<tr data-el=\"{}\"><td class=\"num\">{}</td><td><code>{}</code><br>{} <span class=\"muted\">{}</span>{}</td><td>{}</td><td class=\"num\">x {} · y {}<br>{} × {}<br><span class=\"muted\">{}</span></td><td class=\"num\">{:.2}–{:.2}s<br><span class=\"muted\">{} {:.2}–{:.2}s</span></td><td><code>{}</code><br><span class=\"muted\">{}</span></td><td>{}</td><td><span class=\"muted\">{}</span></td><td>{}</td></tr>",
                esc(&e.id),
                e.z,
                esc(&e.id),
                e.kind.label(),
                if crate::i18n::is_en() { "" } else { info.en },
                if e.visible {
                    ""
                } else {
                    t("<br><span class=\"chip\">隱藏</span>", "<br><span class=\"chip\">hidden</span>")
                },
                content,
                e.x,
                e.y,
                e.w,
                e.h,
                esc(&region(e, cw, ch)),
                e.start,
                end,
                t("絕對", "abs"),
                t0 + e.start,
                t0 + end,
                e.animation.key(),
                animation_desc(e.animation),
                style,
                esc(info.moviepy),
                esc(&e.notes)
            );
        }
        h.push_str("</tbody></table></div></section>\n");
    }

    // MoviePy hints
    h.push_str(t(
        "<section class=\"card\"><h2>MoviePy v2 對應</h2><p>每個元件：依 type 建立 (w, h) 大小的 clip → <code>.with_position((x, y))</code> → <code>.with_start(start).with_duration(end - start)</code>；場景以 <code>CompositeVideoClip</code> 依 z 疊合，再用 <code>concatenate_videoclips</code> 串接；轉場在下一個場景開頭、疊在上一場景最後一格上。中文 <code>TextClip</code> 必須指定含中文字形的字型檔。</p>",
        "<section class=\"card\"><h2>MoviePy v2 mapping</h2><p>For each element: build a (w, h) clip for its type → <code>.with_position((x, y))</code> → <code>.with_start(start).with_duration(end - start)</code>; composite a scene with <code>CompositeVideoClip</code> in z order, then join scenes with <code>concatenate_videoclips</code>; a transition plays at the start of the next scene over the previous scene's last frame. Chinese <code>TextClip</code>s need a font file with CJK glyphs.</p>",
    ));
    for k in ElementKind::ALL {
        if !p.scenes.iter().any(|s| s.elements.iter().any(|e| e.kind == k)) {
            continue;
        }
        let i = k.info();
        let _ = write!(
            h,
            "<h3><code>{}</code> {}{}</h3><pre>{}</pre>",
            k.key(),
            k.label(),
            if crate::i18n::is_en() { String::new() } else { format!(" / {}", i.en) },
            esc(crate::guide::snippet(k)).replace("<br>", "\n")
        );
    }
    let mut files = vec![];
    for (key, _, _, file) in Formats::INFO {
        if f.get(key) {
            files.push(format!("<li><code>{}</code> — {}</li>", esc(file), Formats::label(key)));
        }
    }
    let _ = write!(h, "<h3>{}</h3><ul>{}</ul>", t("本次匯出的檔案", "Files in this export"), files.join(""));
    if f.script {
        h.push_str(t(
            "<p>快速合成佔位影片：<code>python render_moviepy.py layout.json -o draft.mp4 --preview</code></p>",
            "<p>Quick placeholder render: <code>python render_moviepy.py layout.json -o draft.mp4 --preview</code></p>",
        ));
    }
    h.push_str("</section><footer>whitebox-video-storyboard · https://github.com/stevenke1981/whitebox-video-storyboard</footer></main>");
    let _ = writeln!(h, "<script>{JS}</script></body></html>");
    h
}

#[cfg(test)]
mod tests {
    #[test]
    fn b64() {
        assert_eq!(super::base64(b""), "");
        assert_eq!(super::base64(b"f"), "Zg==");
        assert_eq!(super::base64(b"fo"), "Zm8=");
        assert_eq!(super::base64(b"foobar"), "Zm9vYmFy");
    }
}
