//! Backend-independent white-model drawing.
//!
//! A scene is turned into a flat list of [`Prim`]itives in canvas pixel
//! coordinates. The GUI paints them with egui (scaled) and the exporter
//! rasterises them with tiny-skia + ab_glyph, so the editor and the exported
//! draft PNG always look the same.

use crate::model::{Align, Direction, Element, ElementKind, Project, Rgb, Scene, ShapeKind};

pub type Rgba = [u8; 4];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct R {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl R {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> R {
        R { x, y, w: w.max(0.0), h: h.max(0.0) }
    }
    pub fn of(e: &Element) -> R {
        R::new(e.x, e.y, e.w, e.h)
    }
    pub fn inset(&self, d: f32) -> R {
        let dx = d.min(self.w / 2.0);
        let dy = d.min(self.h / 2.0);
        R::new(self.x + dx, self.y + dy, self.w - 2.0 * dx, self.h - 2.0 * dy)
    }
    pub fn cx(&self) -> f32 {
        self.x + self.w / 2.0
    }
    pub fn cy(&self) -> f32 {
        self.y + self.h / 2.0
    }
    pub fn right(&self) -> f32 {
        self.x + self.w
    }
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }
    pub fn contains(&self, p: [f32; 2]) -> bool {
        p[0] >= self.x && p[0] <= self.right() && p[1] >= self.y && p[1] <= self.bottom()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VAlign {
    Top,
    Center,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corner {
    TopLeft,
    BottomRight,
    /// Centred on the top edge (scene-level notes such as the transition).
    TopCenter,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Prim {
    Fill {
        r: R,
        color: Rgba,
        radius: f32,
        ellipse: bool,
    },
    Stroke {
        r: R,
        color: Rgba,
        width: f32,
        radius: f32,
        ellipse: bool,
        dashed: bool,
    },
    Line {
        a: [f32; 2],
        b: [f32; 2],
        color: Rgba,
        width: f32,
    },
    Poly {
        pts: Vec<[f32; 2]>,
        color: Rgba,
    },
    Text {
        r: R,
        text: String,
        size: f32,
        color: Rgba,
        align: Align,
        valign: VAlign,
        stroke: f32,
        stroke_color: Rgba,
    },
    /// Small label chip anchored at a corner of `r` (size measured by the backend).
    Badge {
        r: R,
        text: String,
        size: f32,
        fg: Rgba,
        bg: Rgba,
        corner: Corner,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct DrawOptions {
    /// Draw type/id badges and timing chips (white-model annotations).
    pub annotations: bool,
    /// Draw elements whose `visible` flag is off (dimmed) instead of skipping them.
    pub show_hidden: bool,
}

impl Default for DrawOptions {
    fn default() -> Self {
        DrawOptions { annotations: true, show_hidden: false }
    }
}

pub fn rgba(c: Rgb, a: f32) -> Rgba {
    [c.0[0], c.0[1], c.0[2], (a.clamp(0.0, 1.0) * 255.0).round() as u8]
}

const OUTLINE: Rgba = [60, 60, 60, 210];

/// Unit size: 1.0 at a 1080 px short side.
pub fn unit(p: &Project) -> f32 {
    (p.canvas.width.min(p.canvas.height) as f32 / 1080.0).max(0.2)
}

/// All primitives for one scene (background first, then elements bottom to top).
pub fn scene_prims(project: &Project, scene: &Scene, opts: &DrawOptions) -> Vec<Prim> {
    let (cw, ch) = (project.canvas.width as f32, project.canvas.height as f32);
    let mut out = vec![Prim::Fill {
        r: R::new(0.0, 0.0, cw, ch),
        color: rgba(scene.background, 1.0),
        radius: 0.0,
        ellipse: false,
    }];
    let u = unit(project);
    for e in &scene.elements {
        if !e.visible && !opts.show_hidden {
            continue;
        }
        element_prims(e, scene.duration, u, opts, &mut out);
    }
    if opts.annotations && !scene.transition.is_none() {
        out.push(Prim::Badge {
            r: R::new(0.0, 0.0, cw, ch),
            text: tf!(
                "轉場進入：{} {:.1}s",
                "Transition in: {} {:.1}s",
                scene.transition.kind.key(),
                scene.transition.duration
            ),
            size: (14.0 * u).max(7.0),
            fg: [255, 255, 255, 255],
            bg: [50, 105, 200, 230],
            corner: Corner::TopCenter,
        });
    }
    out
}

fn darker(c: Rgb, f: f32) -> Rgb {
    Rgb(c.0.map(|v| (v as f32 * f) as u8))
}

/// Primitives for a single element.
pub fn element_prims(e: &Element, scene_duration: f32, u: f32, opts: &DrawOptions, out: &mut Vec<Prim>) {
    use ElementKind::*;
    let r = R::of(e);
    let mut fill = rgba(e.bg_color, e.bg_opacity);
    let mut fc = rgba(e.font_color, 1.0);
    if !e.visible {
        fill[3] /= 3;
        fc[3] /= 3;
    }
    if e.kind == Watermark {
        fc[3] = (fc[3] as f32 * 0.75) as u8;
    }
    let pad = (12.0 * u).min(r.w / 4.0).min(r.h / 4.0);
    let dash = |out: &mut Vec<Prim>, radius: f32, ellipse: bool| {
        out.push(Prim::Stroke { r, color: OUTLINE, width: (2.0 * u).max(1.0), radius, ellipse, dashed: true })
    };
    let text = |out: &mut Vec<Prim>, r: R, s: &str, size: f32, align: Align| {
        if !s.trim().is_empty() {
            out.push(Prim::Text {
                r,
                text: s.to_string(),
                size,
                color: fc,
                align,
                valign: VAlign::Center,
                stroke: e.stroke_width,
                stroke_color: rgba(e.stroke_color, fc[3] as f32 / 255.0),
            });
        }
    };
    match e.kind {
        Background => {
            out.push(Prim::Fill { r, color: fill, radius: 0.0, ellipse: false });
            if e.text != "背景" && e.text != "Background" {
                text(out, r.inset(pad), &e.text, e.font_size, e.align);
            }
        }
        Title | Subheading | Subtitle | TextCard | Watermark => {
            let radius = if e.kind == TextCard { 18.0 * u } else { 6.0 * u };
            if fill[3] > 0 {
                out.push(Prim::Fill { r, color: fill, radius, ellipse: false });
            }
            dash(out, radius, false);
            text(out, r.inset(pad), &e.text, e.font_size, e.align);
        }
        Options => {
            let n = e.options.len().max(1);
            let gap = 14.0 * u;
            let row_h = ((r.h - gap * (n as f32 - 1.0)) / n as f32).max(4.0);
            let size = e.font_size.min(row_h * 0.62);
            for i in 0..n {
                let rr = R::new(r.x, r.y + i as f32 * (row_h + gap), r.w, row_h);
                let correct = e.answer == Some(i);
                let radius = row_h.min(28.0 * u) / 2.0;
                let c = if correct { rgba(Rgb([120, 200, 120]), e.bg_opacity.max(0.6)) } else { fill };
                out.push(Prim::Fill { r: rr, color: c, radius, ellipse: false });
                out.push(Prim::Stroke {
                    r: rr,
                    color: if correct { [40, 140, 60, 255] } else { OUTLINE },
                    width: (2.0 * u).max(1.0),
                    radius,
                    ellipse: false,
                    dashed: !correct,
                });
                // letter bubble
                let d = row_h * 0.7;
                let bub = R::new(rr.x + (row_h - d) / 2.0, rr.y + (row_h - d) / 2.0, d, d);
                out.push(Prim::Fill { r: bub, color: rgba(darker(e.bg_color, 0.55), 1.0), radius: 0.0, ellipse: true });
                let letter = ((b'A' + (i as u8 % 26)) as char).to_string();
                out.push(Prim::Text {
                    r: bub,
                    text: letter,
                    size: size * 0.9,
                    color: [255, 255, 255, 255],
                    align: Align::Center,
                    valign: VAlign::Center,
                    stroke: 0.0,
                    stroke_color: [0, 0, 0, 0],
                });
                let label = e.options.get(i).map(String::as_str).unwrap_or(crate::i18n::t("選項", "Option"));
                let tr = R::new(rr.x + row_h + pad * 0.5, rr.y, rr.w - row_h - pad * 1.5, rr.h);
                text(out, tr, label, size, e.align);
            }
        }
        LowerThird => {
            out.push(Prim::Fill { r, color: fill, radius: 4.0 * u, ellipse: false });
            let acc = R::new(r.x, r.y, (16.0 * u).min(r.w / 6.0), r.h);
            out.push(Prim::Fill { r: acc, color: [250, 180, 40, 255], radius: 0.0, ellipse: false });
            dash(out, 4.0 * u, false);
            text(
                out,
                R::new(acc.right() + pad * 1.5, r.y, r.w - acc.w - pad * 2.5, r.h).inset(pad * 0.5),
                &e.text,
                e.font_size,
                e.align,
            );
        }
        MediaPlaceholder | Logo => {
            out.push(Prim::Fill { r, color: fill, radius: 0.0, ellipse: false });
            let lc = rgba(darker(e.bg_color, 0.78), 1.0);
            let w = (2.0 * u).max(1.0);
            out.push(Prim::Line { a: [r.x, r.y], b: [r.right(), r.bottom()], color: lc, width: w });
            out.push(Prim::Line { a: [r.right(), r.y], b: [r.x, r.bottom()], color: lc, width: w });
            out.push(Prim::Stroke { r, color: OUTLINE, width: w, radius: 0.0, ellipse: false, dashed: false });
            if e.kind == MediaPlaceholder {
                let s = r.w.min(r.h) * 0.16;
                let (cx, cy) = (r.cx(), r.cy());
                let disc = R::new(cx - s * 1.25, cy - s * 1.25, s * 2.5, s * 2.5);
                out.push(Prim::Fill { r: disc, color: [255, 255, 255, 200], radius: 0.0, ellipse: true });
                out.push(Prim::Poly {
                    pts: vec![[cx - s * 0.5, cy - s * 0.75], [cx + s * 0.85, cy], [cx - s * 0.5, cy + s * 0.75]],
                    color: lc,
                });
                let mut label = e.text.clone();
                if !e.src.is_empty() {
                    label = format!("{label}\n{}", short_path(&e.src));
                }
                let band = R::new(r.x, r.y + r.h * 0.72, r.w, r.h * 0.26);
                text(out, band.inset(pad * 0.5), &label, e.font_size.min(band.h * 0.45), Align::Center);
            } else {
                let tr = r.inset(pad);
                out.push(Prim::Fill {
                    r: R::new(r.cx() - tr.w * 0.35, r.cy() - e.font_size * 0.7, tr.w * 0.7, e.font_size * 1.4),
                    color: fill,
                    radius: 6.0 * u,
                    ellipse: false,
                });
                text(out, tr, &e.text, e.font_size, Align::Center);
            }
        }
        AvatarFrame => {
            let ellipse = e.shape == ShapeKind::Circle;
            let radius = if e.shape == ShapeKind::Rounded { r.w.min(r.h) * 0.12 } else { 0.0 };
            out.push(Prim::Fill { r, color: fill, radius, ellipse });
            let m = r.w.min(r.h);
            let sil = rgba(darker(e.bg_color, 0.7), fill[3] as f32 / 255.0);
            let head = R::new(r.cx() - m * 0.16, r.cy() - m * 0.3, m * 0.32, m * 0.32);
            out.push(Prim::Fill { r: head, color: sil, radius: 0.0, ellipse: true });
            let body = R::new(r.cx() - m * 0.3, r.cy() + m * 0.06, m * 0.6, m * 0.36);
            out.push(Prim::Fill { r: body, color: sil, radius: m * 0.18, ellipse: false });
            dash(out, radius, ellipse);
            let band = R::new(r.x, r.y + r.h * 0.78, r.w, r.h * 0.2);
            text(out, band, &e.text, e.font_size.min(band.h * 0.8), Align::Center);
        }
        ProgressBar => {
            let radius = r.h / 2.0;
            out.push(Prim::Fill { r, color: fill, radius, ellipse: false });
            out.push(Prim::Fill { r: R::new(r.x, r.y, r.w * 0.6, r.h), color: fc, radius, ellipse: false });
            dash(out, radius, false);
        }
        Countdown => {
            let ellipse = e.shape == ShapeKind::Circle;
            out.push(Prim::Fill { r, color: fill, radius: 12.0 * u, ellipse });
            out.push(Prim::Stroke {
                r: r.inset(4.0 * u),
                color: fc,
                width: (8.0 * u).max(2.0),
                radius: 12.0 * u,
                ellipse,
                dashed: false,
            });
            text(out, r.inset(pad), &e.text, e.font_size, Align::Center);
        }
        CtaButton => {
            let radius = match e.shape {
                ShapeKind::Rect => 0.0,
                _ => r.h / 2.0,
            };
            out.push(Prim::Fill { r, color: fill, radius, ellipse: e.shape == ShapeKind::Circle });
            dash(out, radius, e.shape == ShapeKind::Circle);
            text(out, r.inset(pad), &e.text, e.font_size, e.align);
        }
        CalloutArrow => {
            let col = rgba(e.bg_color, e.bg_opacity.max(0.15));
            let horizontal = matches!(e.direction, Direction::Left | Direction::Right);
            let (label, a, b) = match e.direction {
                Direction::Right => {
                    (R::new(r.x, r.y, r.w * 0.58, r.h), [r.x + r.w * 0.58, r.cy()], [r.right(), r.cy()])
                }
                Direction::Left => {
                    (R::new(r.x + r.w * 0.42, r.y, r.w * 0.58, r.h), [r.x + r.w * 0.42, r.cy()], [r.x, r.cy()])
                }
                Direction::Down => {
                    (R::new(r.x, r.y, r.w, r.h * 0.55), [r.cx(), r.y + r.h * 0.55], [r.cx(), r.bottom()])
                }
                Direction::Up => {
                    (R::new(r.x, r.y + r.h * 0.45, r.w, r.h * 0.55), [r.cx(), r.y + r.h * 0.45], [r.cx(), r.y])
                }
            };
            let thick = if horizontal { r.h * 0.12 } else { r.w * 0.12 }.max(3.0 * u);
            let head = thick * 2.6;
            let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt().max(1e-3);
            let (dx, dy) = ((b[0] - a[0]) / len, (b[1] - a[1]) / len);
            let hb = [b[0] - dx * head.min(len), b[1] - dy * head.min(len)];
            out.push(Prim::Line { a, b: hb, color: col, width: thick });
            out.push(Prim::Poly {
                pts: vec![
                    b,
                    [hb[0] - dy * head * 0.6, hb[1] + dx * head * 0.6],
                    [hb[0] + dy * head * 0.6, hb[1] - dx * head * 0.6],
                ],
                color: col,
            });
            out.push(Prim::Fill { r: label, color: col, radius: 10.0 * u, ellipse: false });
            out.push(Prim::Stroke {
                r: label,
                color: OUTLINE,
                width: (2.0 * u).max(1.0),
                radius: 10.0 * u,
                ellipse: false,
                dashed: true,
            });
            text(out, label.inset(pad), &e.text, e.font_size, Align::Center);
        }
        Shape => {
            let ellipse = e.shape == ShapeKind::Circle;
            let radius = if e.shape == ShapeKind::Rounded { r.w.min(r.h) * 0.15 } else { 0.0 };
            out.push(Prim::Fill { r, color: fill, radius, ellipse });
            dash(out, radius, ellipse);
            text(out, r.inset(pad), &e.text, e.font_size, e.align);
        }
        QrCode => {
            out.push(Prim::Fill { r, color: rgba(e.bg_color, e.bg_opacity.max(0.9)), radius: 0.0, ellipse: false });
            let m = r.w.min(r.h);
            let q = R::new(r.cx() - m / 2.0, r.cy() - m / 2.0, m, m).inset(m * 0.06);
            let cell = q.w / 25.0;
            let dark = rgba(e.font_color, 1.0);
            // deterministic pseudo-random modules
            let mut seed: u32 = e.id.bytes().fold(2166136261u32, |h, b| (h ^ b as u32).wrapping_mul(16777619));
            for gy in 0..25 {
                for gx in 0..25 {
                    let finder = (gx < 8 && gy < 8) || (gx >= 17 && gy < 8) || (gx < 8 && gy >= 17);
                    seed ^= seed << 13;
                    seed ^= seed >> 17;
                    seed ^= seed << 5;
                    if !finder && seed % 5 < 2 {
                        out.push(Prim::Fill {
                            r: R::new(q.x + gx as f32 * cell, q.y + gy as f32 * cell, cell, cell),
                            color: dark,
                            radius: 0.0,
                            ellipse: false,
                        });
                    }
                }
            }
            for (fx, fy) in [(0.0, 0.0), (18.0, 0.0), (0.0, 18.0)] {
                let o = R::new(q.x + fx * cell, q.y + fy * cell, 7.0 * cell, 7.0 * cell);
                out.push(Prim::Fill { r: o, color: dark, radius: 0.0, ellipse: false });
                out.push(Prim::Fill { r: o.inset(cell), color: rgba(e.bg_color, 1.0), radius: 0.0, ellipse: false });
                out.push(Prim::Fill { r: o.inset(cell * 2.0), color: dark, radius: 0.0, ellipse: false });
            }
            out.push(Prim::Stroke {
                r,
                color: OUTLINE,
                width: (2.0 * u).max(1.0),
                radius: 0.0,
                ellipse: false,
                dashed: true,
            });
        }
        Sticker => {
            if fill[3] > 0 {
                out.push(Prim::Fill { r, color: fill, radius: 0.0, ellipse: true });
            }
            dash(out, 0.0, true);
            text(out, r, &e.text, e.font_size, Align::Center);
        }
    }
    if opts.annotations {
        let size = (12.0 * u).max(6.5);
        out.push(Prim::Badge {
            r,
            text: format!("{} · {}", e.kind.label(), e.id),
            size,
            fg: [255, 255, 255, 255],
            bg: [40, 40, 48, 215],
            corner: Corner::TopLeft,
        });
        let end = e.end_in(scene_duration);
        let timed = e.start > 0.0 || end < scene_duration - 1e-3;
        if timed || e.animation != crate::model::Animation::None {
            let mut t = format!("{:.1}–{:.1}s", e.start, end);
            if e.animation != crate::model::Animation::None {
                t.push_str(&format!(" · {}", e.animation.key()));
            }
            out.push(Prim::Badge {
                r,
                text: t,
                size: size * 0.92,
                fg: [30, 30, 30, 255],
                bg: [255, 214, 90, 225],
                corner: Corner::BottomRight,
            });
        }
    }
}

/// Where a badge of size `w`×`h` sits for element rect `r`: inside the corner, or
/// just outside (above / below) when the element is too small to hold it.
pub fn badge_rect(r: &R, w: f32, h: f32, corner: Corner) -> R {
    let small = r.h < h * 2.2 || r.w < w * 0.8;
    match (corner, small) {
        (Corner::TopLeft, false) => R::new(r.x, r.y, w, h),
        (Corner::TopLeft, true) if r.y >= h => R::new(r.x, r.y - h, w, h),
        (Corner::TopLeft, true) => R::new(r.x, r.bottom(), w, h),
        (Corner::BottomRight, false) => R::new(r.right() - w, r.bottom() - h, w, h),
        (Corner::BottomRight, true) if r.y >= h => R::new(r.right() - w, r.y - h, w, h),
        (Corner::BottomRight, true) => R::new(r.right() - w, r.bottom(), w, h),
        (Corner::TopCenter, _) => R::new(r.cx() - w / 2.0, r.y, w, h),
    }
}

fn short_path(p: &str) -> String {
    p.rsplit(['/', '\\']).next().unwrap_or(p).to_string()
}

/// Title-safe (10 %) and action-safe (5 %) guide rectangles plus centre cross.
pub fn safe_guides(p: &Project) -> Vec<Prim> {
    let (w, h) = (p.canvas.width as f32, p.canvas.height as f32);
    let u = unit(p);
    let mut v = vec![];
    for (m, c) in [(crate::model::ACTION_SAFE, [0, 150, 255, 200]), (crate::model::TITLE_SAFE, [255, 80, 160, 200])] {
        v.push(Prim::Stroke {
            r: R::new(w * m, h * m, w * (1.0 - 2.0 * m), h * (1.0 - 2.0 * m)),
            color: c,
            width: (2.0 * u).max(1.0),
            radius: 0.0,
            ellipse: false,
            dashed: true,
        });
    }
    let c = [120, 120, 120, 160];
    v.push(Prim::Line {
        a: [w / 2.0, h / 2.0 - 30.0 * u],
        b: [w / 2.0, h / 2.0 + 30.0 * u],
        color: c,
        width: (2.0 * u).max(1.0),
    });
    v.push(Prim::Line {
        a: [w / 2.0 - 30.0 * u, h / 2.0],
        b: [w / 2.0 + 30.0 * u, h / 2.0],
        color: c,
        width: (2.0 * u).max(1.0),
    });
    v
}
