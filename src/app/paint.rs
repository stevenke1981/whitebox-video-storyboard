//! egui backend for [`wvs::draw::Prim`] lists.

use eframe::egui::{self, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, pos2, vec2};
use egui::text::LayoutJob;
use wvs::draw::{Prim, R, Rgba, VAlign};
use wvs::model::Align;

/// Canvas (target pixels) → screen transform.
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub origin: Pos2,
    pub scale: f32,
}

impl View {
    pub fn pt(&self, x: f32, y: f32) -> Pos2 {
        pos2(self.origin.x + x * self.scale, self.origin.y + y * self.scale)
    }
    pub fn rect(&self, r: &R) -> Rect {
        Rect::from_min_size(self.pt(r.x, r.y), vec2(r.w * self.scale, r.h * self.scale))
    }
    pub fn to_canvas(self, p: Pos2) -> [f32; 2] {
        [(p.x - self.origin.x) / self.scale, (p.y - self.origin.y) / self.scale]
    }
}

pub fn c32(c: Rgba) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

fn ellipse_points(r: Rect, n: usize) -> Vec<Pos2> {
    let (c, rx, ry) = (r.center(), r.width() / 2.0, r.height() / 2.0);
    (0..=n)
        .map(|i| {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            pos2(c.x + rx * a.cos(), c.y + ry * a.sin())
        })
        .collect()
}

fn rounded_points(r: Rect, rad: f32) -> Vec<Pos2> {
    let rad = rad.min(r.width() / 2.0).min(r.height() / 2.0).max(0.0);
    if rad < 0.5 {
        return vec![r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
    }
    let mut v = vec![];
    let corners = [
        (pos2(r.right() - rad, r.top() + rad), -90.0_f32),
        (pos2(r.right() - rad, r.bottom() - rad), 0.0),
        (pos2(r.left() + rad, r.bottom() - rad), 90.0),
        (pos2(r.left() + rad, r.top() + rad), 180.0),
    ];
    for (c, a0) in corners {
        for k in 0..=6 {
            let a = (a0 + k as f32 * 15.0).to_radians();
            v.push(pos2(c.x + rad * a.cos(), c.y + rad * a.sin()));
        }
    }
    v.push(v[0]);
    v
}

pub fn text_galley(
    painter: &Painter,
    text: &str,
    size: f32,
    color: Color32,
    wrap: f32,
    align: Align,
) -> std::sync::Arc<egui::Galley> {
    let mut job = LayoutJob::simple(text.to_string(), FontId::proportional(size.max(1.0)), color, wrap.max(1.0));
    job.halign = match align {
        Align::Left => egui::Align::LEFT,
        Align::Center => egui::Align::Center,
        Align::Right => egui::Align::RIGHT,
    };
    painter.layout_job(job)
}

#[allow(clippy::too_many_arguments)]
fn paint_text(
    painter: &Painter,
    v: &View,
    r: &R,
    text: &str,
    size: f32,
    color: Rgba,
    align: Align,
    valign: VAlign,
    stroke: f32,
    stroke_color: Rgba,
) {
    let rect = v.rect(r);
    let size_s = size * v.scale;
    if size_s < 2.0 {
        // too small to read: hint with a grey bar
        let h = size_s.max(1.0);
        painter.rect_filled(
            Rect::from_center_size(rect.center(), vec2(rect.width() * 0.6, h)),
            0.0,
            c32(color).gamma_multiply(0.5),
        );
        return;
    }
    // Break lines with the same metrics as the PNG exporter (target resolution), so the
    // editor shows exactly the line breaks of the exported drafts; egui must not re-wrap.
    let lines = wvs::raster::Fonts::get().wrap(text, size.max(1.0), r.w.max(size)).join("\n");
    let g = text_galley(painter, &lines, size_s, c32(color), f32::INFINITY, align);
    let gs = g.rect.size();
    let x = match align {
        Align::Left => rect.left(),
        Align::Center => rect.center().x - gs.x / 2.0,
        Align::Right => rect.right() - gs.x,
    };
    let y = match valign {
        VAlign::Top => rect.top(),
        VAlign::Center => rect.center().y - gs.y / 2.0,
        VAlign::Bottom => rect.bottom() - gs.y,
    };
    let pos = pos2(x, y) - g.rect.min.to_vec2();
    let sw = stroke * v.scale;
    let clip = rect.expand2(vec2(sw + 2.0, size_s * 0.25 + sw)).intersect(painter.clip_rect());
    let p = painter.with_clip_rect(clip);
    if sw > 0.3 && stroke_color[3] > 0 {
        for k in 0..8 {
            let a = k as f32 / 8.0 * std::f32::consts::TAU;
            p.galley_with_override_text_color(pos + vec2(a.cos(), a.sin()) * sw, g.clone(), c32(stroke_color));
        }
    }
    p.galley(pos, g, c32(color));
}

pub fn paint_prims(painter: &Painter, prims: &[Prim], v: &View) {
    for prim in prims {
        match prim {
            Prim::Fill { r, color, radius, ellipse } => {
                let rect = v.rect(r);
                if *ellipse {
                    painter.add(Shape::ellipse_filled(rect.center(), rect.size() / 2.0, c32(*color)));
                } else {
                    painter.rect_filled(rect, radius * v.scale, c32(*color));
                }
            }
            Prim::Stroke { r, color, width, radius, ellipse, dashed } => {
                let rect = v.rect(&r.inset(width / 2.0));
                let stroke = Stroke::new((width * v.scale).max(1.0), c32(*color));
                if *dashed {
                    let pts = if *ellipse { ellipse_points(rect, 64) } else { rounded_points(rect, radius * v.scale) };
                    let dash = (width * v.scale * 5.0).max(4.0);
                    painter.extend(Shape::dashed_line(&pts, stroke, dash, dash * 0.7));
                } else if *ellipse {
                    painter.add(Shape::ellipse_stroke(rect.center(), rect.size() / 2.0, stroke));
                } else {
                    painter.rect_stroke(rect, radius * v.scale, stroke, StrokeKind::Middle);
                }
            }
            Prim::Line { a, b, color, width } => {
                painter.line_segment(
                    [v.pt(a[0], a[1]), v.pt(b[0], b[1])],
                    Stroke::new((width * v.scale).max(1.0), c32(*color)),
                );
            }
            Prim::Poly { pts, color } => {
                let pts = pts.iter().map(|p| v.pt(p[0], p[1])).collect();
                painter.add(Shape::convex_polygon(pts, c32(*color), Stroke::NONE));
            }
            Prim::Text { r, text, size, color, align, valign, stroke, stroke_color } => {
                paint_text(painter, v, r, text, *size, *color, *align, *valign, *stroke, *stroke_color)
            }
            Prim::Badge { r, text, size, fg, bg, corner } => {
                let s = (size * v.scale).max(8.0);
                let g = painter.layout_no_wrap(text.clone(), FontId::proportional(s), c32(*fg));
                let pad = s * 0.4;
                let size = vec2(g.size().x + pad * 2.0, s * 1.45);
                let b = wvs::draw::badge_rect(r, size.x / v.scale, size.y / v.scale, *corner);
                let br = v.rect(&b);
                painter.rect_filled(br, s * 0.3, c32(*bg));
                painter.galley(br.center() - g.size() / 2.0, g, c32(*fg));
            }
        }
    }
}
