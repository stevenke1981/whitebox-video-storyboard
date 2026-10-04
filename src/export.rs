//! Export: draft PNGs, layout.json, AGENT_GUIDE.md, render_moviepy.py, font.

use crate::draw::{DrawOptions, scene_prims};
use crate::model::{ACTION_SAFE, Project, TITLE_SAFE};
use crate::raster;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

pub static MOVIEPY_SCRIPT: &str = include_str!("../examples/render_moviepy.py");
pub const SCHEMA_ID: &str = "whitebox-video-storyboard/layout@1";

#[derive(Clone, Debug)]
pub struct ExportOptions {
    /// Copy the bundled CJK font to `out/fonts/` (used by render_moviepy.py).
    pub copy_font: bool,
    /// Draw type/id/timing badges on the PNGs.
    pub annotations: bool,
    /// Draw title-/action-safe guides on the PNGs.
    pub safe_guides: bool,
    /// Also write a contact sheet with all scenes.
    pub overview: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        ExportOptions { copy_font: true, annotations: true, safe_guides: false, overview: true }
    }
}

#[derive(Debug, Default)]
pub struct ExportReport {
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

/// Write everything into `out_dir` (created if missing).
pub fn export_all(
    p: &Project,
    source_json: Option<&str>,
    out_dir: &Path,
    opts: &ExportOptions,
) -> Result<ExportReport, String> {
    let mut p = p.clone();
    p.normalize();
    std::fs::create_dir_all(out_dir).map_err(|e| format!("無法建立 {}: {e}", out_dir.display()))?;
    let mut rep = ExportReport::default();
    let write = |rep: &mut ExportReport, name: &str, data: &[u8]| -> Result<(), String> {
        let path = out_dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, data).map_err(|e| format!("寫入 {} 失敗: {e}", path.display()))?;
        rep.files.push(path);
        Ok(())
    };

    let font_rel = if opts.copy_font {
        let rel = format!("fonts/{}", crate::fonts::CJK_FONT_FILE);
        write(&mut rep, &rel, crate::fonts::CJK_FONT)?;
        write(&mut rep, "fonts/LICENSE-OFL.txt", crate::fonts::CJK_FONT_LICENSE.as_bytes())?;
        Some(rel)
    } else {
        None
    };

    let mut pms = vec![];
    for i in 0..p.scenes.len() {
        let pm = render_scene(&p, i, opts)?;
        let path = out_dir.join(scene_png_name(i, &p.scenes[i].id));
        raster::save_png(&pm, &path)?;
        rep.files.push(path);
        pms.push(pm);
    }
    if opts.overview && !pms.is_empty() {
        let ov = render_overview(&p, &pms)?;
        let path = out_dir.join("storyboard_overview.png");
        raster::save_png(&ov, &path)?;
        rep.files.push(path);
    }

    let layout = layout_json(&p, font_rel.as_deref());
    write(&mut rep, "layout.json", serde_json::to_string_pretty(&layout).unwrap().as_bytes())?;
    let project_json = source_json.map(str::to_string).unwrap_or_else(|| p.to_json());
    write(&mut rep, "project.json", project_json.as_bytes())?;
    write(&mut rep, "AGENT_GUIDE.md", crate::guide::agent_guide(&p, font_rel.as_deref()).as_bytes())?;
    write(&mut rep, "render_moviepy.py", MOVIEPY_SCRIPT.as_bytes())?;
    Ok(rep)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_sample() {
        let p = crate::sample::sample_project("16:9");
        let dir = std::env::temp_dir().join(format!("wvs-test-{}", std::process::id()));
        let rep = export_all(&p, None, &dir, &ExportOptions { copy_font: false, ..Default::default() }).unwrap();
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
}
