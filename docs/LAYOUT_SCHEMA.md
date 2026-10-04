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
