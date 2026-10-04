//! Offscreen rasteriser for [`crate::draw::Prim`] lists (tiny-skia + ab_glyph).
//! Used for the draft PNG export; needs no GPU or display.

use crate::draw::{Prim, R, Rgba, VAlign};
use crate::model::Align;
use ab_glyph::{Font, FontArc, GlyphId, PxScale, ScaleFont, point};
use std::sync::OnceLock;
use tiny_skia::{FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Rect, Stroke, StrokeDash, Transform};

/// Font chain: bundled CJK subset first, then an optional system font.
pub struct Fonts {
    chain: Vec<FontArc>,
}

impl Fonts {
    pub fn get() -> &'static Fonts {
        static F: OnceLock<Fonts> = OnceLock::new();
        F.get_or_init(|| {
            let mut chain = vec![FontArc::try_from_slice(crate::fonts::cjk_font()).expect("bundled font")];
            if let Some((bytes, idx)) = crate::fonts::load_system_fallback()
                && let Ok(f) = ab_glyph::FontVec::try_from_vec_and_index(bytes, idx)
            {
                chain.push(FontArc::new(f));
            }
            Fonts { chain }
        })
    }

    fn pick(&self, c: char) -> (usize, GlyphId) {
        for (i, f) in self.chain.iter().enumerate() {
            let g = f.glyph_id(c);
            if g.0 != 0 {
                return (i, g);
            }
        }
        (0, self.chain[0].glyph_id(c))
    }

    /// `size` is the em size in pixels (like CSS, Pillow/MoviePy and egui), while
    /// ab_glyph's `PxScale` is the ascent-to-descent height: convert.
    fn scale(&self, i: usize, size: f32) -> PxScale {
        let f = &self.chain[i];
        let upem = f.units_per_em().unwrap_or(1000.0).max(1.0);
        PxScale::from(size * f.height_unscaled() / upem)
    }

    fn advance(&self, c: char, size: f32) -> f32 {
        let (i, g) = self.pick(c);
        self.chain[i].as_scaled(self.scale(i, size)).h_advance(g)
    }

    pub fn measure(&self, s: &str, size: f32) -> f32 {
        s.chars().map(|c| self.advance(c, size)).sum()
    }

    fn line_height(&self, size: f32) -> f32 {
        let f = self.chain[0].as_scaled(self.scale(0, size));
        (f.ascent() - f.descent() + f.line_gap()).max(size)
    }

    fn ascent(&self, size: f32) -> f32 {
        self.chain[0].as_scaled(self.scale(0, size)).ascent()
    }

    /// Greedy line wrapping: break at spaces for Latin words, anywhere between CJK characters.
    pub fn wrap(&self, text: &str, size: f32, max_w: f32) -> Vec<String> {
        let mut lines = vec![];
        for para in text.split('\n') {
            let mut line = String::new();
            let mut w = 0.0;
            // tokens: runs of ASCII alphanumerics stay together
            let mut tokens: Vec<String> = vec![];
            for c in para.chars() {
                let wordy = c.is_ascii_alphanumeric() || "'-_.,:;!?)\"".contains(c);
                match tokens.last_mut() {
                    Some(t)
                        if wordy
                            && t.chars().last().is_some_and(|l| l.is_ascii_alphanumeric() || "'-_".contains(l)) =>
                    {
                        t.push(c)
                    }
                    _ => tokens.push(c.to_string()),
                }
            }
            for t in tokens {
                let tw = self.measure(&t, size);
                // kinsoku: closing punctuation never starts a line, let it hang instead
                let hang = t.chars().count() == 1 && NO_LINE_START.contains(t.as_str());
                if w + tw > max_w && !line.is_empty() && !hang {
                    lines.push(line.trim_end().to_string());
                    line = String::new();
                    w = 0.0;
                    if t == " " {
                        continue;
                    }
                }
                if tw > max_w && t.chars().count() > 1 {
                    for c in t.chars() {
                        let cw = self.advance(c, size);
                        if w + cw > max_w && !line.is_empty() {
                            lines.push(std::mem::take(&mut line));
                            w = 0.0;
                        }
                        line.push(c);
                        w += cw;
                    }
                } else {
                    line.push_str(&t);
                    w += tw;
                }
            }
            lines.push(line.trim_end().to_string());
        }
        lines
    }
}

const NO_LINE_START: &str = "，。、！？：；）」』】》〉,.!?:;)]}…ー～";

fn paint(c: Rgba) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color_rgba8(c[0], c[1], c[2], c[3]);
    p.anti_alias = true;
    p
}

fn rect_path(r: &R, radius: f32, ellipse: bool) -> Option<tiny_skia::Path> {
    let rect = Rect::from_xywh(r.x, r.y, r.w.max(0.01), r.h.max(0.01))?;
    if ellipse {
        return PathBuilder::from_oval(rect);
    }
    let rad = radius.min(r.w / 2.0).min(r.h / 2.0);
    if rad <= 0.5 {
        return Some(PathBuilder::from_rect(rect));
    }
    let (x0, y0, x1, y1) = (r.x, r.y, r.right(), r.bottom());
    let k = rad * 0.4477; // (1 - 0.5523)
    let mut pb = PathBuilder::new();
    pb.move_to(x0 + rad, y0);
    pb.line_to(x1 - rad, y0);
    pb.cubic_to(x1 - k, y0, x1, y0 + k, x1, y0 + rad);
    pb.line_to(x1, y1 - rad);
    pb.cubic_to(x1, y1 - k, x1 - k, y1, x1 - rad, y1);
    pb.line_to(x0 + rad, y1);
    pb.cubic_to(x0 + k, y1, x0, y1 - k, x0, y1 - rad);
    pb.line_to(x0, y0 + rad);
    pb.cubic_to(x0, y0 + k, x0 + k, y0, x0 + rad, y0);
    pb.close();
    pb.finish()
}

/// Source-over blend of one pixel with coverage `cov` (0..1).
fn blend(pm: &mut Pixmap, x: i32, y: i32, c: Rgba, cov: f32, clip: &R) {
    if x < 0 || y < 0 || x >= pm.width() as i32 || y >= pm.height() as i32 {
        return;
    }
    let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
    if fx < clip.x || fy < clip.y || fx > clip.right() || fy > clip.bottom() {
        return;
    }
    let a = (c[3] as f32 / 255.0) * cov.clamp(0.0, 1.0);
    if a <= 0.0 {
        return;
    }
    let idx = (y as u32 * pm.width() + x as u32) as usize * 4;
    let data = pm.data_mut();
    // data is premultiplied RGBA
    for k in 0..3 {
        let src = c[k] as f32 * a;
        data[idx + k] = (src + data[idx + k] as f32 * (1.0 - a)).round().min(255.0) as u8;
    }
    let da = data[idx + 3] as f32 / 255.0;
    let out_a = a + da * (1.0 - a);
    data[idx + 3] = (out_a * 255.0).round() as u8;
    for k in 0..3 {
        data[idx + k] = data[idx + k].min(data[idx + 3]);
    }
}

/// Draw one line of text with its left edge at `x` and baseline at `baseline`.
#[allow(clippy::too_many_arguments)]
fn draw_line(pm: &mut Pixmap, fonts: &Fonts, s: &str, size: f32, x: f32, baseline: f32, c: Rgba, clip: &R) {
    let mut pen = x;
    for ch in s.chars() {
        let (i, gid) = fonts.pick(ch);
        let font = &fonts.chain[i];
        let scale = fonts.scale(i, size);
        let g = gid.with_scale_and_position(scale, point(pen, baseline));
        if let Some(og) = font.outline_glyph(g) {
            let b = og.px_bounds();
            og.draw(|gx, gy, cov| blend(pm, b.min.x as i32 + gx as i32, b.min.y as i32 + gy as i32, c, cov, clip));
        }
        pen += font.as_scaled(scale).h_advance(gid);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_text(
    pm: &mut Pixmap,
    r: &R,
    text: &str,
    size: f32,
    color: Rgba,
    align: Align,
    valign: VAlign,
    stroke: f32,
    stroke_color: Rgba,
) {
    let fonts = Fonts::get();
    let size = size.max(1.0);
    let lines = fonts.wrap(text, size, r.w.max(size));
    let lh = fonts.line_height(size) * 1.08;
    let total = lh * lines.len() as f32;
    let top = match valign {
        VAlign::Top => r.y,
        VAlign::Center => r.cy() - total / 2.0,
        VAlign::Bottom => r.bottom() - total,
    };
    // Allow descenders / strokes to spill slightly outside the box (but not far).
    let clip = R::new(
        r.x - stroke - 2.0,
        r.y - stroke - size * 0.25,
        r.w + 2.0 * stroke + 4.0,
        r.h + 2.0 * stroke + size * 0.5,
    );
    let asc = fonts.ascent(size) + (lh - fonts.line_height(size)) / 2.0;
    for (i, line) in lines.iter().enumerate() {
        let w = fonts.measure(line, size);
        let x = match align {
            Align::Left => r.x,
            Align::Center => r.cx() - w / 2.0,
            Align::Right => r.right() - w,
        };
        let base = top + i as f32 * lh + asc;
        if stroke > 0.0 && stroke_color[3] > 0 {
            let n = 16;
            for k in 0..n {
                let ang = k as f32 / n as f32 * std::f32::consts::TAU;
                draw_line(
                    pm,
                    fonts,
                    line,
                    size,
                    x + stroke * ang.cos(),
                    base + stroke * ang.sin(),
                    stroke_color,
                    &clip,
                );
            }
        }
        draw_line(pm, fonts, line, size, x, base, color, &clip);
    }
}

/// Rasterise primitives into a new `width`×`height` pixmap.
pub fn render(prims: &[Prim], width: u32, height: u32) -> Result<Pixmap, String> {
    let mut pm = Pixmap::new(width, height).ok_or("invalid image size")?;
    pm.fill(tiny_skia::Color::WHITE);
    for p in prims {
        draw_prim(&mut pm, p);
    }
    Ok(pm)
}

pub fn draw_prim(pm: &mut Pixmap, p: &Prim) {
    let t = Transform::identity();
    match p {
        Prim::Fill { r, color, radius, ellipse } => {
            if color[3] > 0
                && let Some(path) = rect_path(r, *radius, *ellipse)
            {
                pm.fill_path(&path, &paint(*color), FillRule::Winding, t, None);
            }
        }
        Prim::Stroke { r, color, width, radius, ellipse, dashed } => {
            if let Some(path) = rect_path(&r.inset(width / 2.0), *radius, *ellipse) {
                let mut s = Stroke { width: *width, line_join: LineJoin::Round, ..Default::default() };
                if *dashed {
                    s.dash = StrokeDash::new(vec![width * 5.0, width * 3.5], 0.0);
                }
                pm.stroke_path(&path, &paint(*color), &s, t, None);
            }
        }
        Prim::Line { a, b, color, width } => {
            let mut pb = PathBuilder::new();
            pb.move_to(a[0], a[1]);
            pb.line_to(b[0], b[1]);
            if let Some(path) = pb.finish() {
                let s = Stroke { width: *width, line_cap: LineCap::Butt, ..Default::default() };
                pm.stroke_path(&path, &paint(*color), &s, t, None);
            }
        }
        Prim::Poly { pts, color } => {
            let mut pb = PathBuilder::new();
            for (i, q) in pts.iter().enumerate() {
                if i == 0 { pb.move_to(q[0], q[1]) } else { pb.line_to(q[0], q[1]) }
            }
            pb.close();
            if let Some(path) = pb.finish() {
                pm.fill_path(&path, &paint(*color), FillRule::Winding, t, None);
            }
        }
        Prim::Text { r, text, size, color, align, valign, stroke, stroke_color } => {
            draw_text(pm, r, text, *size, *color, *align, *valign, *stroke, *stroke_color)
        }
        Prim::Badge { r, text, size, fg, bg, corner } => {
            let fonts = Fonts::get();
            let pad = size * 0.4;
            let w = fonts.measure(text, *size) + pad * 2.0;
            let h = size * 1.45;
            let br = crate::draw::badge_rect(r, w, h, *corner);
            if let Some(path) = rect_path(&br, size * 0.3, false) {
                pm.fill_path(&path, &paint(*bg), FillRule::Winding, t, None);
            }
            draw_text(pm, &br, text, *size, *fg, Align::Center, VAlign::Center, 0.0, [0; 4]);
        }
    }
}

/// Save a pixmap as an RGBA PNG.
pub fn save_png(pm: &Pixmap, path: &std::path::Path) -> Result<(), String> {
    let mut buf = Vec::with_capacity((pm.width() * pm.height() * 4) as usize);
    for px in pm.pixels() {
        let c = px.demultiply();
        buf.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
    }
    let img = image::RgbaImage::from_raw(pm.width(), pm.height(), buf).ok_or("image buffer")?;
    img.save(path).map_err(|e| tf!("寫入 {} 失敗: {e}", "failed to write {}: {e}", path.display()))
}
