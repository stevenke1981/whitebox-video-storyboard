# AGENT_GUIDE — MoviePy 小測驗（範例）

> 由 whitebox-video-storyboard 0.2.0 自動產生。本文件告訴 AI Agent 如何依照白模草稿（`scene_*.png`）與 `layout.json`，
> 使用 **Python MoviePy v2**（`moviepy>=2`）把畫面配置成影片。
> Generated file — tells an agent how to build this video with MoviePy v2 from `layout.json`.

## 1. 你的任務 / Task

1. 讀取 `layout.json`（權威資料來源；PNG 只是視覺參考）。
2. 每個 scene 依序播放；每個 element 依 `z` 由小到大疊在該 scene 的畫面上。
3. 位置 `x,y` 為元件**左上角**像素座標，大小 `w,h`；時間 `start/end` 相對於該 scene 起點（秒）。
4. 灰色白模只是佔位：若 `src` 有素材路徑就換成真實素材；文字直接使用 `text`。
5. 依 `animation` 欄位加上動畫（對照第 5 節）。
6. 可以先直接執行 `python render_moviepy.py layout.json -o draft.mp4 --preview` 產生佔位版影片，再在其上替換素材、加上配音／音樂。

## 2. 檔案 / Files

| 檔案 | 用途 |
|---|---|
| `layout.json` | 完整版面資料（schema 見附錄） |
| `project.json` | 編輯器專案檔，可再用 whitebox-video-storyboard 開啟 |
| `scene_01_scene_1.png` | 場景 1 「開場」白模草稿圖 1920×1080 |
| `scene_02_scene_2.png` | 場景 2 「題目」白模草稿圖 1920×1080 |
| `scene_03_scene_3.png` | 場景 3 「結尾」白模草稿圖 1920×1080 |
| `storyboard_overview.png` | 所有場景縮圖總覽 |
| `storyboard.html` | 單一檔案的分鏡網頁（內嵌草稿圖、元件表、時間軸、MoviePy 提示） |
| `storyboard.md` | 純文字版面說明：不看圖也能理解每個元件的位置、大小、文字、時間與動畫 |
| `render_moviepy.py` | 讀取 layout.json 直接合成佔位影片的參考實作（moviepy>=2） |

## 3. 畫布與時間軸 / Canvas & timeline

- 解析度 **1920×1080**（16:9），**30 fps**，總長 **14.00 秒**，共 3 個場景。
- 座標原點在左上角；Title-safe = 內縮 10%（192, 108, 1536, 864），Action-safe = 內縮 5%。重要文字請放在 title-safe 內。

| # | scene id | 名稱 | 開始 | 結束 | 長度 | 背景 | 元件數 | 備註 |
|---|---|---|---|---|---|---|---|---|
| 1 | `scene_1` | 開場 | 0.00s | 4.00s | 4.00s | `#E8E8E8` | 6 | 開場：標題淡入，主持人自我介紹。 |
| 2 | `scene_2` | 題目 | 4.00s | 10.00s | 6.00s | `#E1E1E1` | 7 | 出題：選項依序出現，倒數 5 秒，進度條跑完。 |
| 3 | `scene_3` | 結尾 | 10.00s | 14.00s | 4.00s | `#ECECEC` | 7 | 結尾：公布答案、放精華片段、引導訂閱。 |

## 4. 元件類型 → MoviePy 對應 / Element type mapping

共通步驟（每個 element）：

```python
clip = build(el)                                  # 依 type 建立，大小 = (w, h)
clip = clip.with_position((el['x'], el['y']))     # 左上角
clip = clip.with_start(el['start']).with_duration(el['end'] - el['start'])
scene = CompositeVideoClip([bg] + clips_sorted_by_z, size=(W, H)).with_duration(scene['duration'])
video = concatenate_videoclips(scenes)
```

| type | 中文 | MoviePy 建議做法 | 本專案使用 |
|---|---|---|---|
| `title` | 標題 | TextClip(method='caption') over optional ColorClip | ✔ |
| `subheading` | 副標題 | TextClip(method='caption') over optional ColorClip | ✔ |
| `subtitle` | 字幕 | TextClip with stroke, bottom-centred, timed per line | ✔ |
| `text_card` | 字卡 | ColorClip card + TextClip, CompositeVideoClip | ✔ |
| `options` | 選項 | one ColorClip+TextClip row per option (A/B/C/D) | ✔ |
| `lower_third` | 下三分之一名牌 | ColorClip bar + accent + 2-line TextClip, slide in | ✔ |
| `cta_button` | CTA 按鈕 | rounded ColorClip/ImageClip + TextClip, pop animation | ✔ |
| `watermark` | 浮水印 | semi-transparent TextClip/ImageClip for the whole scene | ✔ |
| `media_placeholder` | 圖片/影片佔位 | ImageClip / VideoFileClip(src).resized((w,h)) | ✔ |
| `logo` | Logo | ImageClip(src).resized(height=h) | ✔ |
| `avatar_frame` | 頭像/人物框 | ImageClip/VideoFileClip(src) with circular mask | ✔ |
| `qr_code` | QR Code | ImageClip(qr.png) – generate with the `qrcode` package | ✔ |
| `sticker` | 貼圖/圖示 | ImageClip(icon.png) or TextClip(emoji font) | ✔ |
| `progress_bar` | 進度條 | VideoClip(frame_function) filling 0→100% over the element time | ✔ |
| `countdown` | 倒數計時 | one TextClip per second, counting down to 0 | ✔ |
| `callout_arrow` | 箭頭/標註 | RGBA ImageClip of an arrow (PIL) + TextClip label | ✔ |
| `shape` | 形狀 | ColorClip (rect) or RGBA ImageClip (circle/rounded) | ✔ |
| `background` | 背景 | ColorClip(size=canvas, color=bg_color) |  |

### `title`（標題 / Title）

```python
card = ColorClip((w, h), color=hex_rgb(el['bg_color'])).with_opacity(el['bg_opacity'])
txt = TextClip(font=FONT, text=el['text'], font_size=int(el['font_size']), color=el['font_color'],
               size=(w, h), method='caption', text_align=el['align'], horizontal_align=el['align'])
clip = CompositeVideoClip([card, txt], size=(w, h))
```

### `subheading`（副標題 / Subheading）

```python
card = ColorClip((w, h), color=hex_rgb(el['bg_color'])).with_opacity(el['bg_opacity'])
txt = TextClip(font=FONT, text=el['text'], font_size=int(el['font_size']), color=el['font_color'],
               size=(w, h), method='caption', text_align=el['align'], horizontal_align=el['align'])
clip = CompositeVideoClip([card, txt], size=(w, h))
```

### `subtitle`（字幕 / Subtitle）

```python
TextClip(font=FONT, text=el['text'], font_size=int(el['font_size']), color=el['font_color'],
         stroke_color=el['stroke_color'], stroke_width=int(el['stroke_width']),
         size=(w, h), method='caption', text_align='center')
# long narration: split into several subtitle clips, each with its own start/duration
```

### `text_card`（字卡 / Text card）

```python
card = ColorClip((w, h), color=hex_rgb(el['bg_color'])).with_opacity(el['bg_opacity'])
txt = TextClip(font=FONT, text=el['text'], font_size=int(el['font_size']), color=el['font_color'],
               size=(w, h), method='caption', text_align=el['align'], horizontal_align=el['align'])
clip = CompositeVideoClip([card, txt], size=(w, h))
```

### `options`（選項 / Options）

```python
rows = []
row_h = (h - 14 * (len(el['options']) - 1)) // len(el['options'])
for i, opt in enumerate(el['options']):
    bg = ColorClip((w, row_h), color=(120, 200, 120) if el.get('answer') == i else hex_rgb(el['bg_color']))
    t = TextClip(font=FONT, text=f"{'ABCD'[i]}. {opt}", font_size=int(el['font_size']), color=el['font_color'],
                 size=(w - 40, row_h), method='label', horizontal_align='left')
    rows.append(CompositeVideoClip([bg, t.with_position((20, 0))], size=(w, row_h)).with_position((0, i * (row_h + 14))))
clip = CompositeVideoClip(rows, size=(w, h))   # optionally stagger rows with .with_start(i * 0.3)
```

### `lower_third`（下三分之一名牌 / Lower third）

```python
bar = ColorClip((w, h), color=hex_rgb(el['bg_color'])).with_opacity(el['bg_opacity'])
accent = ColorClip((16, h), color=(250, 180, 40))
name, _, role = el['text'].partition('\n')
txt = TextClip(font=FONT, text=name + '\n' + role, font_size=int(el['font_size']), color=el['font_color'],
               size=(w - 60, h), method='label', horizontal_align='left')
clip = CompositeVideoClip([bar, accent, txt.with_position((40, 0))], size=(w, h))
```

### `cta_button`（CTA 按鈕 / CTA button）

```python
btn = ImageClip(rounded_rect_rgba(w, h, el['bg_color']))   # PIL rounded_rectangle -> numpy RGBA
txt = TextClip(font=FONT, text=el['text'], font_size=int(el['font_size']), color=el['font_color'], size=(w, h), method='label')
clip = CompositeVideoClip([btn, txt], size=(w, h))
```

### `watermark`（浮水印 / Watermark）

```python
TextClip(font=FONT, text=el['text'], font_size=int(el['font_size']), color=el['font_color'],
         size=(w, h), method='label', horizontal_align=el['align']).with_opacity(0.75)
```

### `media_placeholder`（圖片/影片佔位 / Media placeholder）

```python
src = el['src']  # replace the grey placeholder with real footage when available
clip = (VideoFileClip(src, audio=False).subclipped(0, dur) if src.endswith(('.mp4', '.mov'))
        else ImageClip(src)).resized((w, h))
```

### `logo`（Logo / Logo）

```python
ImageClip(el['src']).resized(height=h)  # keep aspect ratio; centre inside the box
```

### `avatar_frame`（頭像/人物框 / Avatar / presenter）

```python
face = VideoFileClip(el['src'], audio=False).resized((w, h))
mask = ImageClip(circle_mask(w, h), is_mask=True)   # numpy array 0..1, white circle
clip = face.with_mask(mask)
```

### `qr_code`（QR Code / QR code）

```python
import qrcode; qrcode.make(el['notes'] or el['text']).save('qr.png')
clip = ImageClip('qr.png').resized((w, h))
```

### `sticker`（貼圖/圖示 / Emoji / icon sticker）

```python
ImageClip(el['src']).resized((w, h)) if el['src'] else \
TextClip(font=EMOJI_OR_CJK_FONT, text=el['text'], font_size=int(el['font_size']), color=el['font_color'], size=(w, h), method='label')
```

### `progress_bar`（進度條 / Progress bar）

```python
def frame(t):
    img = np.zeros((h, w, 3), np.uint8); img[:] = hex_rgb(el['bg_color'])
    img[:, : int(w * t / dur)] = hex_rgb(el['font_color'])
    return img
clip = VideoClip(frame_function=frame, duration=dur)
```

### `countdown`（倒數計時 / Countdown）

```python
n = int(el['text']); step = dur / (n + 1)
nums = [TextClip(font=FONT, text=str(n - i), font_size=int(el['font_size']), color=el['font_color'],
                 size=(w, h), method='label').with_start(i * step).with_duration(step) for i in range(n + 1)]
clip = CompositeVideoClip(nums, size=(w, h))
```

### `callout_arrow`（箭頭/標註 / Callout arrow）

```python
arrow = ImageClip(arrow_rgba(w, h, el['direction'], el['bg_color']))  # draw with PIL ImageDraw.line + polygon
label = TextClip(font=FONT, text=el['text'], font_size=int(el['font_size']), color=el['font_color'], size=(int(w * .58), h), method='label')
clip = CompositeVideoClip([arrow, label], size=(w, h))
```

### `shape`（形狀 / Shape）

```python
ColorClip((w, h), color=hex_rgb(el['bg_color'])).with_opacity(el['bg_opacity'])  # rect
# circle / rounded: draw RGBA with PIL (ellipse / rounded_rectangle) and wrap in ImageClip
```

## 5. 動畫提示 / Animation hints

| animation | 中文 | MoviePy v2 |
|---|---|---|
| `none` | 無 | 直接出現 |
| `fade_in` | 淡入 | `clip.with_effects([vfx.CrossFadeIn(0.5)])` |
| `fade_out` | 淡出 | `clip.with_effects([vfx.CrossFadeOut(0.5)])` |
| `fade_in_out` | 淡入淡出 | `[vfx.CrossFadeIn(0.5), vfx.CrossFadeOut(0.5)]` |
| `slide_up` | 由下滑入 | `clip.with_position(lambda t: (x, y + off * (1 - ease(t / 0.5))))` + CrossFadeIn |
| `slide_left` | 由右滑入 | `clip.with_position(lambda t: (x + off * (1 - ease(t / 0.5)), y))` + CrossFadeIn |
| `pop` | 彈出放大 | `clip.resized(lambda t: 0.5 + 0.5 * ease(t / 0.35))`，位置同步修正以保持中心不動 |
| `typewriter` | 打字機 | 逐字建立 TextClip，或用 mask 由左至右擦出（`mask.transform(...)`） |

`ease = lambda p: 1 - (1 - min(1, max(0, p))) ** 3`。CrossFadeIn/Out 作用在 mask 上，適合疊加層；`vfx.FadeIn` 是從黑色淡入，只適合整個場景。

## 6. 中文字型 / CJK font

`TextClip(font=...)` 必須指定**含中文字形的字型檔路徑**，否則中文會變成方塊（tofu）。

- 本匯出未附字型，請用 `--font` 或環境變數 `WVS_FONT` 指定。
- 其他可用路徑：Windows `C:/Windows/Fonts/msjh.ttc`（微軟正黑體）、macOS `/System/Library/Fonts/PingFang.ttc`、Linux `/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc`。
- `.ttc` 字型集合在 Pillow/MoviePy 中預設使用第 0 個字面（face index 0）。
- MoviePy 的 `method='caption'` 會自動換行；若要精準控制中英混排換行，可先自行斷行再用 `method='label'`（`render_moviepy.py` 的 `wrap_text()` 即此做法）。
- MoviePy 2.x 對 ascent 很大的字型（如 Noto CJK）用固定 `size=(w, h)` 時字尾可能被裁掉；可給較高的 TextClip 再自行垂直置中（見 `render_moviepy.py` 的 `_text_once()`）。

## 7. 逐場景元件清單 / Scene-by-scene elements

### 場景 1 — 開場（`scene_1`，0.00s → 4.00s，長 4.00s）

![scene_1](scene_01_scene_1.png)

備註：開場：標題淡入，主持人自我介紹。

| z | id | type | 文字 / 內容 | x, y, w, h | 時間（場景內） | 動畫 | 樣式 | 備註 |
|---|---|---|---|---|---|---|---|---|
| 0 | `logo_1` | `logo` | LOGO | 96, 64, 220, 110 | 0.00–4.00s | `none` | 40px #464646 / bg #CDCDCD α1.00 |  |
| 1 | `title_1` | `title` | 三分鐘學會 MoviePy 排版 | 360, 330, 1200, 160 | 0.30–4.00s | `fade_in` | 100px #141414 / bg #C8C8C8 α0.00 |  |
| 2 | `subheading_1` | `subheading` | 白模草稿 → layout.json → 自動合成影片 | 460, 500, 1000, 90 | 0.80–4.00s | `slide_up` | 54px #323232 / bg #C8C8C8 α0.00 |  |
| 3 | `avatar_frame_1` | `avatar_frame` | 主持人 | 1500, 700, 300, 300 | 0.00–4.00s | `none` | 36px #3C3C3C / bg #C3C3C3 α1.00 |  |
| 4 | `lower_third_1` | `lower_third` | Ke Sheng Da ⏎ 影片創作者 · 自動化剪輯 | 96, 820, 720, 140 | 1.00–3.80s | `slide_left` | 44px #FFFFFF / bg #282828 α0.88 |  |
| 5 | `watermark_1` | `watermark` | @my_channel | 1464, 64, 360, 60 | 0.00–4.00s | `none` | 34px #6E6E6E / bg #000000 α0.00 |  |

### 場景 2 — 題目（`scene_2`，4.00s → 10.00s，長 6.00s）

![scene_2](scene_02_scene_2.png)

備註：出題：選項依序出現，倒數 5 秒，進度條跑完。

| z | id | type | 文字 / 內容 | x, y, w, h | 時間（場景內） | 動畫 | 樣式 | 備註 |
|---|---|---|---|---|---|---|---|---|
| 0 | `text_card_1` | `text_card` | Q1. 下列哪一個是 ⏎ MoviePy v2 設定位置的寫法？ | 96, 140, 800, 560 | 0.00–6.00s | `fade_in` | 58px #1E1E1E / bg #F5F5F5 α0.92 |  |
| 1 | `options_1` | `options` | A. clip.set_pos(...) / B. clip.with_position(...) ✔ / C. clip.position = ... / D. clip.move(...) | 1040, 140, 780, 560 | 0.50–6.00s | `slide_left` | 44px #1E1E1E / bg #F0F0F0 α0.95 |  |
| 2 | `countdown_1` | `countdown` | 5 | 1640, 760, 170, 170 | 1.00–6.00s | `none` | 90px #141414 / bg #EBEBEB α0.95 |  |
| 3 | `callout_arrow_1` | `callout_arrow` | 倒數中！ | 1260, 760, 360, 130 | 1.00–6.00s | `pop` | 40px #141414 / bg #FFD23C α1.00 |  |
| 4 | `subtitle_1` | `subtitle` | 請在倒數結束前，選出正確答案 | 260, 900, 1400, 90 | 0.00–6.00s | `none` | 52px #FFFFFF / bg #000000 α0.45 / 描邊 3px #000000 |  |
| 5 | `progress_bar_1` | `progress_bar` |  | 0, 1050, 1920, 30 | 0.00–6.00s | `none` | 24px #FAB428 / bg #5A5A5A α0.60 |  |
| 6 | `sticker_1` | `sticker` | ？ | 900, 20, 120, 120 | 0.00–6.00s | `none` | 96px #FABE1E / bg #EBEBEB α0.00 |  |

### 場景 3 — 結尾（`scene_3`，10.00s → 14.00s，長 4.00s）

![scene_3](scene_03_scene_3.png)

備註：結尾：公布答案、放精華片段、引導訂閱。

| z | id | type | 文字 / 內容 | x, y, w, h | 時間（場景內） | 動畫 | 樣式 | 備註 |
|---|---|---|---|---|---|---|---|---|
| 0 | `media_placeholder_1` | `media_placeholder` | 精華片段 B-roll [src: assets/broll.mp4] | 96, 120, 960, 540 | 0.00–4.00s | `none` | 44px #464646 / bg #B9B9B9 α1.00 |  |
| 1 | `title_2` | `title` | 答案：B | 1120, 160, 700, 130 | 0.00–4.00s | `pop` | 96px #141414 / bg #C8C8C8 α0.00 |  |
| 2 | `subheading_2` | `subheading` | v2 統一用 with_* 方法回傳新的 clip | 1120, 300, 700, 140 | 0.00–4.00s | `none` | 46px #323232 / bg #C8C8C8 α0.00 |  |
| 3 | `cta_button_1` | `cta_button` | 立即訂閱 ▶ | 1120, 500, 440, 110 | 1.00–4.00s | `pop` | 50px #FFFFFF / bg #DC3C3C α1.00 |  |
| 4 | `qr_code_1` | `qr_code` | QR | 1600, 470, 220, 220 | 0.00–4.00s | `none` | 32px #141414 / bg #FFFFFF α1.00 |  |
| 5 | `shape_1` | `shape` |  | 96, 720, 960, 12 | 0.00–4.00s | `none` | 36px #3C3C3C / bg #FAB428 α1.00 |  |
| 6 | `subtitle_2` | `subtitle` | 我們下一集見！ | 260, 900, 1400, 90 | 0.00–4.00s | `none` | 52px #FFFFFF / bg #000000 α0.45 / 描邊 3px #000000 |  |

## 8. 最小範例 / Minimal MoviePy v2 example

```python
import json
from moviepy import ColorClip, TextClip, CompositeVideoClip, concatenate_videoclips, vfx

L = json.load(open('layout.json', encoding='utf-8'))
W, H = L['canvas']['width'], L['canvas']['height']
FONT = 'C:/Windows/Fonts/msjh.ttc'
hex_rgb = lambda s: tuple(int(s[i:i + 2], 16) for i in (1, 3, 5))

scenes = []
for sc in L['scenes']:
    clips = [ColorClip((W, H), color=hex_rgb(sc['background'])).with_duration(sc['duration'])]
    for el in sorted(sc['elements'], key=lambda e: e['z']):
        if not el['visible'] or not el['text']:
            continue
        w, h = int(el['w']), int(el['h'])
        c = TextClip(font=FONT, text=el['text'], font_size=int(el['font_size']), color=el['font_color'],
                     size=(w, h), method='caption', text_align=el['align'])
        if el['animation'] == 'fade_in':
            c = c.with_effects([vfx.CrossFadeIn(0.5)])
        clips.append(c.with_position((el['x'], el['y'])).with_start(el['start']).with_duration(el['end'] - el['start']))
    scenes.append(CompositeVideoClip(clips, size=(W, H)).with_duration(sc['duration']))

concatenate_videoclips(scenes).write_videofile('out.mp4', fps=L['fps'], codec='libx264', audio=False)
```

完整、涵蓋所有元件類型的實作請參考同目錄的 `render_moviepy.py`。

## 9. 檢查清單 / Checklist

- [ ] 輸出解析度 = 1920×1080、fps = 30、總長 ≈ 14.00s
- [ ] 每個場景的中間影格與對應 `scene_*.png` 的版面一致（位置、大小、層級）
- [ ] 中文沒有變成方塊（字型路徑正確）
- [ ] 文字都在 title-safe 範圍內、沒有被裁切
- [ ] `visible: false` 的元件沒有出現
- [ ] 有 `src` 的佔位已換成真實素材

## 附錄 / Appendix

## layout.json schema (v1)

All coordinates are **pixels in the target video resolution**, origin at the
**top-left** of the frame; `x, y` is the element's top-left corner. Times are
seconds. Colours are `"#RRGGBB"` strings.

### Top level

| key | type | description |
|---|---|---|
| `schema` | string | always `"whitebox-video-storyboard/layout@1"` |
| `generator` | string | app name + version |
| `project` | string | project name |
| `canvas` | object | `{ "width": 1920, "height": 1080, "preset": "16:9" }` |
| `fps` | int | frames per second |
| `total_duration` | float | sum of all scene durations |
| `safe_area` | object | `action_margin` (0.05), `title_margin` (0.10) and the resulting rects `action_safe_rect` / `title_safe_rect` as `[x, y, w, h]` |
| `fonts` | object | `{ "cjk": "fonts/NotoSansCJKtc-Subset.otf" }` path relative to layout.json (or `null`) |
| `coordinate_system` | string | human-readable reminder of the conventions above |
| `scenes` | array | ordered list of scenes / shots (played back-to-back) |

### Scene

| key | type | description |
|---|---|---|
| `index` | int | 0-based order |
| `id` | string | e.g. `scene_1` |
| `name` | string | display name |
| `start` / `end` | float | absolute time of the scene in the final video |
| `duration` | float | seconds |
| `background` | colour | base colour of the frame (a full-frame `ColorClip`) |
| `notes` | string | director / agent notes |
| `draft_png` | string | file name of the white-model draft image (exists only if the `png` format was exported) |
| `elements` | array | elements sorted by `z` (bottom → top) |

### Element

| key | type | description |
|---|---|---|
| `id` | string | unique in the project, e.g. `title_1` |
| `name` | string | display name |
| `type` | string | see the type table below |
| `x`, `y`, `w`, `h` | float | box in pixels (top-left + size) |
| `center` | `[float, float]` | derived box centre |
| `text` | string | main text (`\n` = line break). For `lower_third` line 1 = name, line 2 = title. For `countdown` the start number. For `sticker` the emoji / icon name |
| `font_size` | float | pixels |
| `font_color` | colour | text colour (also the fill colour of `progress_bar`, the ring of `countdown`, dark modules of `qr_code`) |
| `bg_color` / `bg_opacity` | colour / 0‥1 | box fill; opacity 0 = no box |
| `align` | `left` \| `center` \| `right` | horizontal text alignment (text is vertically centred) |
| `stroke_color` / `stroke_width` | colour / px | text outline (subtitles) |
| `start` / `end` | float | visible interval **relative to the scene start** (`end` already resolved) |
| `until_scene_end` | bool | `true` if `end` follows the scene duration |
| `abs_start` / `abs_end` | float | same interval in absolute video time |
| `animation` | string | `none`, `fade_in`, `fade_out`, `fade_in_out`, `slide_up`, `slide_left`, `pop`, `typewriter` |
| `z` | int | stacking order, 0 = bottom |
| `options` | string[] | rows of an `options` element |
| `answer` | int? | index of the correct / highlighted option |
| `shape` | `rect` \| `rounded` \| `circle` | for `shape`, `avatar_frame`, `countdown`, `cta_button` |
| `direction` | `left` \| `right` \| `up` \| `down` | arrow direction of `callout_arrow` |
| `src` | string | optional asset path (image/video/logo/icon), relative to layout.json |
| `notes` | string | free-form instructions for the agent |
| `visible` / `locked` | bool | editor flags (`visible: false` elements should be skipped) |
| `moviepy` | string | short hint of the MoviePy construction for this type |

### Element types

| type | 中文 | meaning |
|---|---|---|
| `background` | 背景 | full-frame colour / image layer |
| `title` | 標題 | main headline |
| `subheading` | 副標題 | secondary headline |
| `subtitle` | 字幕 | spoken-line caption, usually bottom centre with outline |
| `text_card` | 字卡 | card with key points |
| `options` | 選項 | multiple-choice list (A/B/C/D) |
| `lower_third` | 下三分之一名牌 | name / title strap |
| `media_placeholder` | 圖片/影片佔位 | B-roll, screenshot or image slot |
| `logo` | Logo | brand logo slot |
| `avatar_frame` | 頭像/人物框 | presenter / face-cam frame |
| `progress_bar` | 進度條 | fills 0 → 100 % over the element's time |
| `countdown` | 倒數計時 | counts down from `text` to 0 over the element's time |
| `watermark` | 浮水印 | semi-transparent channel mark |
| `cta_button` | CTA 按鈕 | call-to-action button |
| `callout_arrow` | 箭頭/標註 | arrow + label pointing at something |
| `shape` | 形狀 | decorative rect / rounded rect / circle |
| `qr_code` | QR Code | QR code slot |
| `sticker` | 貼圖/圖示 | emoji / icon sticker |

Unknown future types should be rendered as a labelled box (`shape` behaviour).
