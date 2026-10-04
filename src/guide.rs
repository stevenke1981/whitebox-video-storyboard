//! Generates `AGENT_GUIDE.md`: how an AI agent should turn layout.json into a
//! MoviePy (v2 API) composition.

use crate::export::scene_png_name;
use crate::model::{ElementKind, Project};
use std::fmt::Write;

pub static SCHEMA_MD: &str = include_str!("../docs/LAYOUT_SCHEMA.md");

fn md_escape(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ⏎ ")
}

/// MoviePy v2 snippet for one element type. Variables available in the snippet:
/// `el` (dict), `x, y, w, h`, `start, dur`, `FONT`.
pub fn snippet(kind: ElementKind) -> &'static str {
    use ElementKind::*;
    match kind {
        Background => "ColorClip((W, H), color=hex_rgb(el['bg_color'])).with_opacity(el['bg_opacity'])",
        Title | Subheading | TextCard => {
            "card = ColorClip((w, h), color=hex_rgb(el['bg_color'])).with_opacity(el['bg_opacity'])\n\
             txt = TextClip(font=FONT, text=el['text'], font_size=int(el['font_size']), color=el['font_color'],\n\
             \x20              size=(w, h), method='caption', text_align=el['align'], horizontal_align=el['align'])\n\
             clip = CompositeVideoClip([card, txt], size=(w, h))"
        }
        Subtitle => {
            "TextClip(font=FONT, text=el['text'], font_size=int(el['font_size']), color=el['font_color'],\n\
             \x20        stroke_color=el['stroke_color'], stroke_width=int(el['stroke_width']),\n\
             \x20        size=(w, h), method='caption', text_align='center')\n\
             # long narration: split into several subtitle clips, each with its own start/duration"
        }
        Options => {
            "rows = []\nrow_h = (h - 14 * (len(el['options']) - 1)) // len(el['options'])\n\
             for i, opt in enumerate(el['options']):\n\
             \x20   bg = ColorClip((w, row_h), color=(120, 200, 120) if el.get('answer') == i else hex_rgb(el['bg_color']))\n\
             \x20   t = TextClip(font=FONT, text=f\"{'ABCD'[i]}. {opt}\", font_size=int(el['font_size']), color=el['font_color'],\n\
             \x20                size=(w - 40, row_h), method='label', horizontal_align='left')\n\
             \x20   rows.append(CompositeVideoClip([bg, t.with_position((20, 0))], size=(w, row_h)).with_position((0, i * (row_h + 14))))\n\
             clip = CompositeVideoClip(rows, size=(w, h))   # optionally stagger rows with .with_start(i * 0.3)"
        }
        LowerThird => {
            "bar = ColorClip((w, h), color=hex_rgb(el['bg_color'])).with_opacity(el['bg_opacity'])\n\
             accent = ColorClip((16, h), color=(250, 180, 40))\n\
             name, _, role = el['text'].partition('\\n')\n\
             txt = TextClip(font=FONT, text=name + '\\n' + role, font_size=int(el['font_size']), color=el['font_color'],\n\
             \x20              size=(w - 60, h), method='label', horizontal_align='left')\n\
             clip = CompositeVideoClip([bar, accent, txt.with_position((40, 0))], size=(w, h))"
        }
        MediaPlaceholder => {
            "src = el['src']  # replace the grey placeholder with real footage when available\n\
             clip = (VideoFileClip(src, audio=False).subclipped(0, dur) if src.endswith(('.mp4', '.mov'))\n\
             \x20       else ImageClip(src)).resized((w, h))"
        }
        Logo => "ImageClip(el['src']).resized(height=h)  # keep aspect ratio; centre inside the box",
        AvatarFrame => {
            "face = VideoFileClip(el['src'], audio=False).resized((w, h))\n\
             mask = ImageClip(circle_mask(w, h), is_mask=True)   # numpy array 0..1, white circle\n\
             clip = face.with_mask(mask)"
        }
        ProgressBar => {
            "def frame(t):\n\
             \x20   img = np.zeros((h, w, 3), np.uint8); img[:] = hex_rgb(el['bg_color'])\n\
             \x20   img[:, : int(w * t / dur)] = hex_rgb(el['font_color'])\n\
             \x20   return img\n\
             clip = VideoClip(frame_function=frame, duration=dur)"
        }
        Countdown => {
            "n = int(el['text']); step = dur / (n + 1)\n\
             nums = [TextClip(font=FONT, text=str(n - i), font_size=int(el['font_size']), color=el['font_color'],\n\
             \x20                size=(w, h), method='label').with_start(i * step).with_duration(step) for i in range(n + 1)]\n\
             clip = CompositeVideoClip(nums, size=(w, h))"
        }
        Watermark => {
            "TextClip(font=FONT, text=el['text'], font_size=int(el['font_size']), color=el['font_color'],\n\
             \x20        size=(w, h), method='label', horizontal_align=el['align']).with_opacity(0.75)"
        }
        CtaButton => {
            "btn = ImageClip(rounded_rect_rgba(w, h, el['bg_color']))   # PIL rounded_rectangle -> numpy RGBA\n\
             txt = TextClip(font=FONT, text=el['text'], font_size=int(el['font_size']), color=el['font_color'], size=(w, h), method='label')\n\
             clip = CompositeVideoClip([btn, txt], size=(w, h))"
        }
        CalloutArrow => {
            "arrow = ImageClip(arrow_rgba(w, h, el['direction'], el['bg_color']))  # draw with PIL ImageDraw.line + polygon\n\
             label = TextClip(font=FONT, text=el['text'], font_size=int(el['font_size']), color=el['font_color'], size=(int(w * .58), h), method='label')\n\
             clip = CompositeVideoClip([arrow, label], size=(w, h))"
        }
        Shape => {
            "ColorClip((w, h), color=hex_rgb(el['bg_color'])).with_opacity(el['bg_opacity'])  # rect\n\
             # circle / rounded: draw RGBA with PIL (ellipse / rounded_rectangle) and wrap in ImageClip"
        }
        QrCode => {
            "import qrcode; qrcode.make(el['notes'] or el['text']).save('qr.png')\n\
             clip = ImageClip('qr.png').resized((w, h))"
        }
        Sticker => {
            "ImageClip(el['src']).resized((w, h)) if el['src'] else \\\n\
             TextClip(font=EMOJI_OR_CJK_FONT, text=el['text'], font_size=int(el['font_size']), color=el['font_color'], size=(w, h), method='label')"
        }
    }
}

pub fn agent_guide(p: &Project, font_rel: Option<&str>) -> String {
    let mut s = String::new();
    let (w, h) = (p.canvas.width, p.canvas.height);
    let _ = writeln!(s, "# AGENT_GUIDE — {}\n", p.name);
    let _ = writeln!(
        s,
        "> 由 whitebox-video-storyboard {} 自動產生。本文件告訴 AI Agent 如何依照白模草稿（`scene_*.png`）與 `layout.json`，\n\
         > 使用 **Python MoviePy v2**（`moviepy>=2`）把畫面配置成影片。\n\
         > Generated file — tells an agent how to build this video with MoviePy v2 from `layout.json`.\n",
        env!("CARGO_PKG_VERSION")
    );

    let _ = writeln!(s, "## 1. 你的任務 / Task\n");
    let _ = writeln!(
        s,
        "1. 讀取 `layout.json`（權威資料來源；PNG 只是視覺參考）。\n\
         2. 每個 scene 依序播放；每個 element 依 `z` 由小到大疊在該 scene 的畫面上。\n\
         3. 位置 `x,y` 為元件**左上角**像素座標，大小 `w,h`；時間 `start/end` 相對於該 scene 起點（秒）。\n\
         4. 灰色白模只是佔位：若 `src` 有素材路徑就換成真實素材；文字直接使用 `text`。\n\
         5. 依 `animation` 欄位加上動畫（對照第 5 節）。\n\
         6. 可以先直接執行 `python render_moviepy.py layout.json -o draft.mp4 --preview` 產生佔位版影片，再在其上替換素材、加上配音／音樂。\n"
    );

    let _ = writeln!(s, "## 2. 檔案 / Files\n");
    let _ = writeln!(s, "| 檔案 | 用途 |\n|---|---|");
    let _ = writeln!(s, "| `layout.json` | 完整版面資料（schema 見附錄） |");
    let _ = writeln!(s, "| `project.json` | 編輯器專案檔，可再用 whitebox-video-storyboard 開啟 |");
    for (i, sc) in p.scenes.iter().enumerate() {
        let _ = writeln!(
            s,
            "| `{}` | 場景 {} 「{}」白模草稿圖 {}×{} |",
            scene_png_name(i, &sc.id),
            i + 1,
            md_escape(&sc.name),
            w,
            h
        );
    }
    let _ = writeln!(s, "| `storyboard_overview.png` | 所有場景縮圖總覽 |");
    let _ = writeln!(s, "| `render_moviepy.py` | 讀取 layout.json 直接合成佔位影片的參考實作（moviepy>=2） |");
    if let Some(f) = font_rel {
        let _ = writeln!(s, "| `{f}` | 內附中文字型（Noto Sans CJK TC 子集，SIL OFL 1.1），供 TextClip 使用 |");
    }
    s.push('\n');

    let _ = writeln!(s, "## 3. 畫布與時間軸 / Canvas & timeline\n");
    let _ = writeln!(
        s,
        "- 解析度 **{w}×{h}**（{}），**{} fps**，總長 **{:.2} 秒**，共 {} 個場景。\n\
         - 座標原點在左上角；Title-safe = 內縮 10%（{}, {}, {}, {}），Action-safe = 內縮 5%。重要文字請放在 title-safe 內。\n",
        if p.canvas.preset.is_empty() { "自訂" } else { &p.canvas.preset },
        p.fps,
        p.total_duration(),
        p.scenes.len(),
        (w as f32 * 0.1).round(),
        (h as f32 * 0.1).round(),
        (w as f32 * 0.8).round(),
        (h as f32 * 0.8).round(),
    );
    let _ = writeln!(
        s,
        "| # | scene id | 名稱 | 開始 | 結束 | 長度 | 背景 | 元件數 | 備註 |\n|---|---|---|---|---|---|---|---|---|"
    );
    for (i, sc) in p.scenes.iter().enumerate() {
        let t0 = p.scene_start(i);
        let _ = writeln!(
            s,
            "| {} | `{}` | {} | {:.2}s | {:.2}s | {:.2}s | `{}` | {} | {} |",
            i + 1,
            sc.id,
            md_escape(&sc.name),
            t0,
            t0 + sc.duration,
            sc.duration,
            sc.background.hex(),
            sc.elements.len(),
            md_escape(&sc.notes)
        );
    }
    s.push('\n');

    let _ = writeln!(s, "## 4. 元件類型 → MoviePy 對應 / Element type mapping\n");
    let _ = writeln!(
        s,
        "共通步驟（每個 element）：\n\n```python\n\
         clip = build(el)                                  # 依 type 建立，大小 = (w, h)\n\
         clip = clip.with_position((el['x'], el['y']))     # 左上角\n\
         clip = clip.with_start(el['start']).with_duration(el['end'] - el['start'])\n\
         scene = CompositeVideoClip([bg] + clips_sorted_by_z, size=(W, H)).with_duration(scene['duration'])\n\
         video = concatenate_videoclips(scenes)\n```\n"
    );
    let used: Vec<ElementKind> = ElementKind::ALL
        .iter()
        .copied()
        .filter(|k| p.scenes.iter().any(|s| s.elements.iter().any(|e| e.kind == *k)))
        .collect();
    let _ = writeln!(s, "| type | 中文 | MoviePy 建議做法 | 本專案使用 |\n|---|---|---|---|");
    for k in ElementKind::ALL {
        let i = k.info();
        let _ = writeln!(
            s,
            "| `{}` | {} | {} | {} |",
            k.key(),
            i.zh,
            md_escape(i.moviepy),
            if used.contains(&k) { "✔" } else { "" }
        );
    }
    s.push('\n');
    for k in &used {
        let i = k.info();
        let _ = writeln!(s, "### `{}`（{} / {}）\n\n```python\n{}\n```\n", k.key(), i.zh, i.en, snippet(*k));
    }

    let _ = writeln!(s, "## 5. 動畫提示 / Animation hints\n");
    let _ = writeln!(
        s,
        "| animation | 中文 | MoviePy v2 |\n|---|---|---|\n\
         | `none` | 無 | 直接出現 |\n\
         | `fade_in` | 淡入 | `clip.with_effects([vfx.CrossFadeIn(0.5)])` |\n\
         | `fade_out` | 淡出 | `clip.with_effects([vfx.CrossFadeOut(0.5)])` |\n\
         | `fade_in_out` | 淡入淡出 | `[vfx.CrossFadeIn(0.5), vfx.CrossFadeOut(0.5)]` |\n\
         | `slide_up` | 由下滑入 | `clip.with_position(lambda t: (x, y + off * (1 - ease(t / 0.5))))` + CrossFadeIn |\n\
         | `slide_left` | 由右滑入 | `clip.with_position(lambda t: (x + off * (1 - ease(t / 0.5)), y))` + CrossFadeIn |\n\
         | `pop` | 彈出放大 | `clip.resized(lambda t: 0.5 + 0.5 * ease(t / 0.35))`，位置同步修正以保持中心不動 |\n\
         | `typewriter` | 打字機 | 逐字建立 TextClip，或用 mask 由左至右擦出（`mask.transform(...)`） |\n\n\
         `ease = lambda p: 1 - (1 - min(1, max(0, p))) ** 3`。CrossFadeIn/Out 作用在 mask 上，適合疊加層；`vfx.FadeIn` 是從黑色淡入，只適合整個場景。\n"
    );

    let _ = writeln!(s, "## 6. 中文字型 / CJK font\n");
    let _ = writeln!(
        s,
        "`TextClip(font=...)` 必須指定**含中文字形的字型檔路徑**，否則中文會變成方塊（tofu）。\n\n\
         - 本匯出{}\n\
         - 其他可用路徑：Windows `C:/Windows/Fonts/msjh.ttc`（微軟正黑體）、macOS `/System/Library/Fonts/PingFang.ttc`、Linux `/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc`。\n\
         - `.ttc` 字型集合在 Pillow/MoviePy 中預設使用第 0 個字面（face index 0）。\n\
         - MoviePy 的 `method='caption'` 會自動換行；若要精準控制中英混排換行，可先自行斷行再用 `method='label'`（`render_moviepy.py` 的 `wrap_text()` 即此做法）。\n\
         - MoviePy 2.x 對 ascent 很大的字型（如 Noto CJK）用固定 `size=(w, h)` 時字尾可能被裁掉；可給較高的 TextClip 再自行垂直置中（見 `render_moviepy.py` 的 `_text_once()`）。\n",
        match font_rel {
            Some(f) =>
                format!("已附上 `{f}`，`render_moviepy.py` 會自動使用（也可用 `--font` 或環境變數 `WVS_FONT` 覆寫）。"),
            None => "未附字型，請用 `--font` 或環境變數 `WVS_FONT` 指定。".to_string(),
        }
    );

    let _ = writeln!(s, "## 7. 逐場景元件清單 / Scene-by-scene elements\n");
    for (i, sc) in p.scenes.iter().enumerate() {
        let t0 = p.scene_start(i);
        let _ = writeln!(
            s,
            "### 場景 {} — {}（`{}`，{:.2}s → {:.2}s，長 {:.2}s）\n\n![{}]({})\n",
            i + 1,
            sc.name,
            sc.id,
            t0,
            t0 + sc.duration,
            sc.duration,
            sc.id,
            scene_png_name(i, &sc.id)
        );
        if !sc.notes.is_empty() {
            let _ = writeln!(s, "備註：{}\n", sc.notes);
        }
        if sc.elements.is_empty() {
            let _ = writeln!(s, "（無元件）\n");
            continue;
        }
        let _ = writeln!(
            s,
            "| z | id | type | 文字 / 內容 | x, y, w, h | 時間（場景內） | 動畫 | 樣式 | 備註 |\n|---|---|---|---|---|---|---|---|---|"
        );
        for e in &sc.elements {
            let mut content = e.text.clone();
            if !e.options.is_empty() {
                content = e
                    .options
                    .iter()
                    .enumerate()
                    .map(|(j, o)| {
                        format!("{}. {}{}", (b'A' + j as u8) as char, o, if e.answer == Some(j) { " ✔" } else { "" })
                    })
                    .collect::<Vec<_>>()
                    .join(" / ");
            }
            if !e.src.is_empty() {
                content = format!("{content} [src: {}]", e.src);
            }
            let style = format!(
                "{}px {} / bg {} α{:.2}{}",
                e.font_size,
                e.font_color.hex(),
                e.bg_color.hex(),
                e.bg_opacity,
                if e.stroke_width > 0.0 {
                    format!(" / 描邊 {}px {}", e.stroke_width, e.stroke_color.hex())
                } else {
                    String::new()
                }
            );
            let _ = writeln!(
                s,
                "| {} | `{}` | `{}` | {} | {}, {}, {}, {} | {:.2}–{:.2}s | `{}` | {} | {} |",
                e.z,
                e.id,
                e.kind.key(),
                md_escape(&content),
                e.x,
                e.y,
                e.w,
                e.h,
                e.start,
                e.end_in(sc.duration),
                e.animation.key(),
                style,
                md_escape(&format!("{}{}", if e.visible { "" } else { "（隱藏，略過）" }, e.notes))
            );
        }
        s.push('\n');
    }

    let _ = writeln!(s, "## 8. 最小範例 / Minimal MoviePy v2 example\n");
    let _ = writeln!(
        s,
        "```python\nimport json\nfrom moviepy import ColorClip, TextClip, CompositeVideoClip, concatenate_videoclips, vfx\n\n\
         L = json.load(open('layout.json', encoding='utf-8'))\nW, H = L['canvas']['width'], L['canvas']['height']\n\
         FONT = '{}'\n\
         hex_rgb = lambda s: tuple(int(s[i:i + 2], 16) for i in (1, 3, 5))\n\n\
         scenes = []\nfor sc in L['scenes']:\n\
         \x20   clips = [ColorClip((W, H), color=hex_rgb(sc['background'])).with_duration(sc['duration'])]\n\
         \x20   for el in sorted(sc['elements'], key=lambda e: e['z']):\n\
         \x20       if not el['visible'] or not el['text']:\n\
         \x20           continue\n\
         \x20       w, h = int(el['w']), int(el['h'])\n\
         \x20       c = TextClip(font=FONT, text=el['text'], font_size=int(el['font_size']), color=el['font_color'],\n\
         \x20                    size=(w, h), method='caption', text_align=el['align'])\n\
         \x20       if el['animation'] == 'fade_in':\n\
         \x20           c = c.with_effects([vfx.CrossFadeIn(0.5)])\n\
         \x20       clips.append(c.with_position((el['x'], el['y'])).with_start(el['start']).with_duration(el['end'] - el['start']))\n\
         \x20   scenes.append(CompositeVideoClip(clips, size=(W, H)).with_duration(sc['duration']))\n\n\
         concatenate_videoclips(scenes).write_videofile('out.mp4', fps=L['fps'], codec='libx264', audio=False)\n```\n\n\
         完整、涵蓋所有元件類型的實作請參考同目錄的 `render_moviepy.py`。\n",
        font_rel.unwrap_or("C:/Windows/Fonts/msjh.ttc")
    );

    let _ = writeln!(s, "## 9. 檢查清單 / Checklist\n");
    let _ = writeln!(
        s,
        "- [ ] 輸出解析度 = {w}×{h}、fps = {}、總長 ≈ {:.2}s\n\
         - [ ] 每個場景的中間影格與對應 `scene_*.png` 的版面一致（位置、大小、層級）\n\
         - [ ] 中文沒有變成方塊（字型路徑正確）\n\
         - [ ] 文字都在 title-safe 範圍內、沒有被裁切\n\
         - [ ] `visible: false` 的元件沒有出現\n\
         - [ ] 有 `src` 的佔位已換成真實素材\n",
        p.fps,
        p.total_duration()
    );

    let _ = writeln!(s, "## 附錄 / Appendix\n");
    s.push_str(SCHEMA_MD);
    s
}
