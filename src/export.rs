//! Export: draft PNGs, layout.json, AGENT_GUIDE.md, render_moviepy.py, font.

use crate::draw::{DrawOptions, scene_prims};
use crate::model::{ACTION_SAFE, Project, TITLE_SAFE};
use crate::raster;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

pub static MOVIEPY_SCRIPT: &str = include_str!("../examples/render_moviepy.py");
pub const SCHEMA_ID: &str = "whitebox-video-storyboard/layout@1";

/// Which files an export writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Formats {
    /// `scene_XX_<id>.png` white-model draft per scene.
    pub png: bool,
    /// `storyboard_overview.png` contact sheet.
    pub overview: bool,
    /// `layout.json`.
    pub layout: bool,
    /// `project.json` (re-editable project file).
    pub project: bool,
    /// `AGENT_GUIDE.md`.
    pub guide: bool,
    /// `render_moviepy.py`.
    pub script: bool,
    /// `storyboard.html` (self-contained, images embedded).
    pub html: bool,
    /// `storyboard.md` (text-only description in Traditional Chinese).
    pub md: bool,
    /// `fonts/NotoSansCJKtc-Subset.otf` + licence.
    pub font: bool,
}

impl Default for Formats {
    fn default() -> Self {
        Formats {
            png: true,
            overview: true,
            layout: true,
            project: true,
            guide: true,
            script: true,
            html: true,
            md: true,
            font: true,
        }
    }
}

impl Formats {
    /// `(cli key, 中文 label, file name)` for every format, in display order.
    pub const INFO: [(&'static str, &'static str, &'static str); 9] = [
        ("png", "場景草稿 PNG", "scene_XX_*.png"),
        ("overview", "總覽圖", "storyboard_overview.png"),
        ("layout", "版面資料 layout.json", "layout.json"),
        ("project", "專案檔", "project.json"),
        ("guide", "Agent 指南", "AGENT_GUIDE.md"),
        ("script", "MoviePy 腳本", "render_moviepy.py"),
        ("html", "HTML 分鏡頁", "storyboard.html"),
        ("md", "Markdown 文字說明", "storyboard.md"),
        ("font", "中文字型", "fonts/NotoSansCJKtc-Subset.otf"),
    ];

    pub const NONE: Formats = Formats {
        png: false,
        overview: false,
        layout: false,
        project: false,
        guide: false,
        script: false,
        html: false,
        md: false,
        font: false,
    };

    pub fn get_mut(&mut self, key: &str) -> Option<&mut bool> {
        Some(match key {
            "png" => &mut self.png,
            "overview" => &mut self.overview,
            "layout" => &mut self.layout,
            "project" => &mut self.project,
            "guide" => &mut self.guide,
            "script" => &mut self.script,
            "html" => &mut self.html,
            "md" => &mut self.md,
            "font" => &mut self.font,
            _ => return None,
        })
    }

    pub fn get(&self, key: &str) -> bool {
        let mut c = *self;
        c.get_mut(key).map(|b| *b).unwrap_or(false)
    }

    /// Parse `png,html,md` / `all` (aliases: json→layout, markdown→md, agent→guide, py→script, fonts→font).
    pub fn parse(list: &str) -> Result<Formats, String> {
        let mut f = Formats::NONE;
        for raw in list.split(',').map(|s| s.trim().to_ascii_lowercase()).filter(|s| !s.is_empty()) {
            if raw == "all" {
                f = Formats::default();
                continue;
            }
            let key = match raw.as_str() {
                "json" | "layout.json" => "layout",
                "markdown" | "text" => "md",
                "agent" | "agent_guide" => "guide",
                "py" | "python" | "moviepy" => "script",
                "fonts" => "font",
                "sheet" => "overview",
                k => k,
            };
            *f.get_mut(key).ok_or_else(|| format!("未知格式 {raw:?}（可用：all,{}）", Self::keys().join(",")))? = true;
        }
        if f == Formats::NONE {
            return Err("至少要選一種輸出格式".into());
        }
        Ok(f)
    }

    pub fn keys() -> Vec<&'static str> {
        Self::INFO.iter().map(|i| i.0).collect()
    }

    pub fn list(&self) -> String {
        Self::INFO.iter().filter(|i| self.get(i.0)).map(|i| i.0).collect::<Vec<_>>().join(",")
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportOptions {
    pub formats: Formats,
    /// Draw type/id/timing badges on the PNGs.
    pub annotations: bool,
    /// Draw title-/action-safe guides on the PNGs.
    pub safe_guides: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        ExportOptions { formats: Formats::default(), annotations: true, safe_guides: false }
    }
}

/// Folder-name-safe version of a project / file name (keeps CJK letters).
pub fn sanitize_name(name: &str) -> String {
    let s: String =
        name.trim().chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    let s = s.split('_').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("_");
    if s.is_empty() { "untitled".into() } else { s.chars().take(60).collect() }
}

/// Export folder base name for a project file path: its file stem, or `untitled`.
pub fn project_dir_name(path: Option<&Path>) -> String {
    path.and_then(|p| p.file_stem()).map(|s| sanitize_name(&s.to_string_lossy())).unwrap_or_else(|| "untitled".into())
}

/// `<name>_<YYYYMMDD_HHMMSS>` in local time.
pub fn timestamped_name(name: &str) -> String {
    format!("{}_{}", sanitize_name(name), chrono::Local::now().format("%Y%m%d_%H%M%S"))
}

/// A not-yet-existing `<base>/<name>_<timestamp>[_N]` directory path.
pub fn timestamped_dir(base: &Path, name: &str) -> PathBuf {
    let stem = timestamped_name(name);
    let mut dir = base.join(&stem);
    let mut n = 2;
    while dir.exists() {
        dir = base.join(format!("{stem}_{n}"));
        n += 1;
    }
    dir
}

#[derive(Debug, Default)]
pub struct ExportReport {
    /// The folder everything was written to.
    pub dir: PathBuf,
    pub files: Vec<PathBuf>,
}

pub fn scene_png_name(i: usize, id: &str) -> String {
    let id: String =
        id.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).collect();
    format!("scene_{:02}_{}.png", i + 1, id)
}

fn rect_json(margin: f32, w: f32, h: f32) -> Value {
    json!([
        (w * margin).round(),
        (h * margin).round(),
        (w * (1.0 - 2.0 * margin)).round(),
        (h * (1.0 - 2.0 * margin)).round()
    ])
}

/// Build the `layout.json` document (project + derived fields).
pub fn layout_json(p: &Project, font_rel: Option<&str>) -> Value {
    let mut p = p.clone();
    p.normalize();
    let (w, h) = (p.canvas.width as f32, p.canvas.height as f32);
    let mut scenes = vec![];
    for (i, s) in p.scenes.iter().enumerate() {
        let t0 = p.scene_start(i);
        let mut els = vec![];
        for e in &s.elements {
            let mut v = serde_json::to_value(e).expect("element json");
            let end = e.end_in(s.duration);
            let o = v.as_object_mut().unwrap();
            o.insert("end".into(), json!(round3(end)));
            o.insert("until_scene_end".into(), json!(e.end.is_none()));
            o.insert("abs_start".into(), json!(round3(t0 + e.start)));
            o.insert("abs_end".into(), json!(round3(t0 + end)));
            o.insert("center".into(), json!([e.x + e.w / 2.0, e.y + e.h / 2.0]));
            o.insert("moviepy".into(), json!(e.kind.info().moviepy));
            els.push(v);
        }
        scenes.push(json!({
            "index": i,
            "id": s.id,
            "name": s.name,
            "start": round3(t0),
            "end": round3(t0 + s.duration),
            "duration": s.duration,
            "background": s.background,
            "notes": s.notes,
            "draft_png": scene_png_name(i, &s.id),
            "elements": els,
        }));
    }
    json!({
        "schema": SCHEMA_ID,
        "generator": format!("whitebox-video-storyboard {}", env!("CARGO_PKG_VERSION")),
        "version": p.version,
        "project": p.name,
        "name": p.name,
        "canvas": { "width": p.canvas.width, "height": p.canvas.height, "preset": p.canvas.preset },
        "fps": p.fps,
        "total_duration": round3(p.total_duration()),
        "coordinate_system": "pixels in target resolution; origin top-left; x,y = element top-left corner; times in seconds; element start/end are relative to the scene start",
        "safe_area": {
            "action_margin": ACTION_SAFE,
            "title_margin": TITLE_SAFE,
            "action_safe_rect": rect_json(ACTION_SAFE, w, h),
            "title_safe_rect": rect_json(TITLE_SAFE, w, h),
        },
        "fonts": { "cjk": font_rel },
        "scenes": scenes,
    })
}

fn round3(v: f32) -> f64 {
    (v as f64 * 1000.0).round() / 1000.0
}

/// Render one scene to a pixmap.
pub fn render_scene(p: &Project, index: usize, opts: &ExportOptions) -> Result<tiny_skia::Pixmap, String> {
    let s = p.scenes.get(index).ok_or("scene index out of range")?;
    let mut prims = scene_prims(p, s, &DrawOptions { annotations: opts.annotations, show_hidden: false });
    if opts.safe_guides {
        prims.extend(crate::draw::safe_guides(p));
    }
    raster::render(&prims, p.canvas.width, p.canvas.height)
}

/// Contact sheet: all scenes as thumbnails with captions.
pub fn render_overview(p: &Project, scenes: &[tiny_skia::Pixmap]) -> Result<tiny_skia::Pixmap, String> {
    use crate::draw::{Prim, R, VAlign};
    let n = scenes.len().max(1);
    let cols = n.min(3);
    let rows = n.div_ceil(cols);
    let tw = 640.0_f32;
    let th = tw * p.canvas.height as f32 / p.canvas.width as f32;
    let (gap, cap, head) = (24.0, 56.0, 90.0);
    let w = (cols as f32 * (tw + gap) + gap) as u32;
    let h = (head + rows as f32 * (th + cap + gap) + gap) as u32;
    let mut pm = tiny_skia::Pixmap::new(w, h).ok_or("overview size")?;
    pm.fill(tiny_skia::Color::from_rgba8(250, 250, 250, 255));
    let title = format!(
        "{}  ·  {}×{}  ·  {} fps  ·  {} 個場景  ·  共 {:.1} 秒",
        p.name,
        p.canvas.width,
        p.canvas.height,
        p.fps,
        p.scenes.len(),
        p.total_duration()
    );
    let text = |pm: &mut tiny_skia::Pixmap, r: R, s: &str, size: f32, c: [u8; 4]| {
        raster::draw_prim(
            pm,
            &Prim::Text {
                r,
                text: s.into(),
                size,
                color: c,
                align: crate::model::Align::Left,
                valign: VAlign::Center,
                stroke: 0.0,
                stroke_color: [0; 4],
            },
        )
    };
    text(&mut pm, R::new(gap, 10.0, w as f32 - 2.0 * gap, head - 20.0), &title, 34.0, [30, 30, 30, 255]);
    for (i, sp) in scenes.iter().enumerate() {
        let (c, r) = (i % cols, i / cols);
        let x = gap + c as f32 * (tw + gap);
        let y = head + r as f32 * (th + cap + gap);
        let sx = tw / sp.width() as f32;
        pm.draw_pixmap(
            0,
            0,
            sp.as_ref(),
            &tiny_skia::PixmapPaint { quality: tiny_skia::FilterQuality::Bicubic, ..Default::default() },
            tiny_skia::Transform::from_row(sx, 0.0, 0.0, sx, x, y),
            None,
        );
        raster::draw_prim(
            &mut pm,
            &Prim::Stroke {
                r: R::new(x, y, tw, th),
                color: [90, 90, 90, 255],
                width: 2.0,
                radius: 0.0,
                ellipse: false,
                dashed: false,
            },
        );
        let s = &p.scenes[i];
        let t0 = p.scene_start(i);
        let cap_s = format!(
            "#{} {}  ·  {:.1}s（{:.1}–{:.1}s）· {} 個元件",
            i + 1,
            s.name,
            s.duration,
            t0,
            t0 + s.duration,
            s.elements.len()
        );
        text(&mut pm, R::new(x, y + th + 4.0, tw, cap - 8.0), &cap_s, 24.0, [40, 40, 40, 255]);
    }
    Ok(pm)
}

/// Encode a pixmap as PNG bytes.
pub fn png_bytes(pm: &tiny_skia::Pixmap) -> Result<Vec<u8>, String> {
    let mut buf = Vec::with_capacity((pm.width() * pm.height() * 4) as usize);
    for px in pm.pixels() {
        let c = px.demultiply();
        buf.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
    }
    let img = image::RgbaImage::from_raw(pm.width(), pm.height(), buf).ok_or("image buffer")?;
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png).map_err(|e| e.to_string())?;
    Ok(out.into_inner())
}

/// Write the selected formats into `out_dir` (created if missing).
pub fn export_all(
    p: &Project,
    source_json: Option<&str>,
    out_dir: &Path,
    opts: &ExportOptions,
) -> Result<ExportReport, String> {
    let f = opts.formats;
    if f == Formats::NONE {
        return Err("至少要選一種輸出格式".into());
    }
    let mut p = p.clone();
    p.normalize();
    std::fs::create_dir_all(out_dir).map_err(|e| format!("無法建立 {}: {e}", out_dir.display()))?;
    let mut rep = ExportReport { dir: out_dir.to_path_buf(), ..Default::default() };
    let write = |rep: &mut ExportReport, name: &str, data: &[u8]| -> Result<(), String> {
        let path = out_dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, data).map_err(|e| format!("寫入 {} 失敗: {e}", path.display()))?;
        rep.files.push(path);
        Ok(())
    };

    let font_rel = if f.font {
        let rel = format!("fonts/{}", crate::fonts::CJK_FONT_FILE);
        write(&mut rep, &rel, crate::fonts::cjk_font())?;
        write(&mut rep, "fonts/LICENSE-OFL.txt", crate::fonts::CJK_FONT_LICENSE.as_bytes())?;
        Some(rel)
    } else {
        None
    };

    // Scene images are needed for PNG, overview and the (self-contained) HTML page.
    let mut pms = vec![];
    let mut pngs: Vec<Vec<u8>> = vec![];
    if f.png || f.overview || f.html {
        for i in 0..p.scenes.len() {
            let pm = render_scene(&p, i, opts)?;
            if f.png || f.html {
                let bytes = png_bytes(&pm)?;
                if f.png {
                    write(&mut rep, &scene_png_name(i, &p.scenes[i].id), &bytes)?;
                }
                pngs.push(bytes);
            }
            pms.push(pm);
        }
    }
    if f.overview && !pms.is_empty() {
        let ov = render_overview(&p, &pms)?;
        write(&mut rep, "storyboard_overview.png", &png_bytes(&ov)?)?;
    }
    if f.layout {
        let layout = layout_json(&p, font_rel.as_deref());
        write(&mut rep, "layout.json", serde_json::to_string_pretty(&layout).unwrap().as_bytes())?;
    }
    if f.project {
        let project_json = source_json.map(str::to_string).unwrap_or_else(|| p.to_json());
        write(&mut rep, "project.json", project_json.as_bytes())?;
    }
    if f.guide {
        write(&mut rep, "AGENT_GUIDE.md", crate::guide::agent_guide(&p, font_rel.as_deref(), &f).as_bytes())?;
    }
    if f.script {
        write(&mut rep, "render_moviepy.py", MOVIEPY_SCRIPT.as_bytes())?;
    }
    if f.html {
        write(&mut rep, "storyboard.html", crate::html::storyboard_html(&p, &pngs, &f).as_bytes())?;
    }
    if f.md {
        write(&mut rep, "storyboard.md", crate::describe::storyboard_md(&p, &f).as_bytes())?;
    }
    Ok(rep)
}

/// Export into `<base>/<name>_<timestamp>/` (or straight into `base` when `subdir` is false).
pub fn export_to(
    p: &Project,
    source_json: Option<&str>,
    base: &Path,
    name: &str,
    subdir: bool,
    opts: &ExportOptions,
) -> Result<ExportReport, String> {
    // Absolute path so the caller can show / open exactly where files landed.
    let base = std::path::absolute(base).unwrap_or_else(|_| base.to_path_buf());
    let dir = if subdir { timestamped_dir(&base, name) } else { base };
    export_all(p, source_json, &dir, opts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_sample() {
        let p = crate::sample::sample_project("16:9");
        let dir = std::env::temp_dir().join(format!("wvs-test-{}", std::process::id()));
        let mut opts = ExportOptions::default();
        opts.formats.font = false;
        let rep = export_all(&p, None, &dir, &opts).unwrap();
        assert!(rep.files.iter().any(|f| f.ends_with("storyboard.html")));
        assert!(rep.files.iter().any(|f| f.ends_with("storyboard.md")));
        assert!(rep.files.iter().any(|f| f.ends_with("layout.json")));
        let img = image::open(dir.join(scene_png_name(0, "scene_1"))).unwrap();
        assert_eq!((img.width(), img.height()), (1920, 1080));
        let layout: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("layout.json")).unwrap()).unwrap();
        assert_eq!(layout["scenes"].as_array().unwrap().len(), 3);
        // layout.json can be opened again as a project
        let q = Project::from_json(&std::fs::read_to_string(dir.join("layout.json")).unwrap()).unwrap();
        assert_eq!(q.scenes[1].elements.len(), p.scenes[1].elements.len());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn formats_parse_and_subset() {
        assert_eq!(Formats::parse("all").unwrap(), Formats::default());
        let f = Formats::parse("png, markdown,json").unwrap();
        assert!(f.png && f.md && f.layout && !f.html && !f.font);
        assert!(Formats::parse("bogus").is_err());
        assert!(Formats::parse("").is_err());
        let p = crate::sample::sample_project("16:9");
        let base = std::env::temp_dir().join(format!("wvs-test-fmt-{}", std::process::id()));
        let opts = ExportOptions { formats: Formats::parse("md,html").unwrap(), ..Default::default() };
        let rep = export_to(&p, None, &base, "我的 專案!", true, &opts).unwrap();
        let name = rep.dir.file_name().unwrap().to_string_lossy().to_string();
        assert!(name.starts_with("我的_專案_"), "{name}");
        let mut files: Vec<_> =
            rep.files.iter().map(|f| f.file_name().unwrap().to_string_lossy().to_string()).collect();
        files.sort();
        assert_eq!(files, ["storyboard.html", "storyboard.md"]);
        // a second export in the same second gets a distinct folder
        let rep2 = export_to(&p, None, &base, "我的 專案!", true, &opts).unwrap();
        assert_ne!(rep.dir, rep2.dir);
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn dir_names() {
        assert_eq!(project_dir_name(None), "untitled");
        assert_eq!(project_dir_name(Some(Path::new("/a/b/quiz video.json"))), "quiz_video");
        assert_eq!(sanitize_name("  ??  "), "untitled");
    }
}
