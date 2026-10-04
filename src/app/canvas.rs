//! Central canvas: scaled white-model preview, selection, move/resize with snapping, drop target.

use super::paint::{View, c32, paint_prims};
use super::{App, Drag, DragMode, Guide, Handle};
use eframe::egui::{self, Color32, CursorIcon, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, pos2, vec2};
use wvs::draw::{DrawOptions, R, element_prims, safe_guides, scene_prims, unit};
use wvs::model::{ACTION_SAFE, ElementKind, TITLE_SAFE};

const HANDLE_PX: f32 = 7.0;
const SNAP_PX: f32 = 7.0;

fn handles(r: Rect) -> [(Handle, Pos2); 8] {
    let c = r.center();
    [
        (Handle::NW, r.left_top()),
        (Handle::N, pos2(c.x, r.top())),
        (Handle::NE, r.right_top()),
        (Handle::E, pos2(r.right(), c.y)),
        (Handle::SE, r.right_bottom()),
        (Handle::S, pos2(c.x, r.bottom())),
        (Handle::SW, r.left_bottom()),
        (Handle::W, pos2(r.left(), c.y)),
    ]
}

fn handle_cursor(h: Handle) -> CursorIcon {
    match h {
        Handle::N | Handle::S => CursorIcon::ResizeVertical,
        Handle::E | Handle::W => CursorIcon::ResizeHorizontal,
        Handle::NE | Handle::SW => CursorIcon::ResizeNeSw,
        Handle::NW | Handle::SE => CursorIcon::ResizeNwSe,
    }
}

impl App {
    fn view_for(&self, avail: Rect) -> View {
        let (cw, ch) = (self.project.canvas.width as f32, self.project.canvas.height as f32);
        let m = 28.0;
        let fit = ((avail.width() - 2.0 * m) / cw).min((avail.height() - 2.0 * m) / ch).max(0.01);
        let scale = fit * self.zoom;
        let size = vec2(cw * scale, ch * scale);
        let origin = avail.center() - size / 2.0;
        View { origin, scale }
    }

    /// Vertical (x) and horizontal (y) snap targets in canvas px.
    fn snap_targets(&self, exclude: &str) -> (Vec<f32>, Vec<f32>) {
        let (w, h) = (self.project.canvas.width as f32, self.project.canvas.height as f32);
        let mut xs = vec![0.0, w / 2.0, w];
        let mut ys = vec![0.0, h / 2.0, h];
        if self.show_safe {
            for m in [ACTION_SAFE, TITLE_SAFE] {
                xs.extend([w * m, w * (1.0 - m)]);
                ys.extend([h * m, h * (1.0 - m)]);
            }
        }
        for e in &self.scene().elements {
            if e.id == exclude || !e.visible || e.kind == ElementKind::Background {
                continue;
            }
            xs.extend([e.x, e.x + e.w / 2.0, e.x + e.w]);
            ys.extend([e.y, e.y + e.h / 2.0, e.y + e.h]);
        }
        (xs, ys)
    }

    /// Best snap offset for any of `edges` to any of `targets` within `tol`.
    fn best_snap(edges: &[f32], targets: &[f32], tol: f32) -> Option<(f32, f32)> {
        let mut best: Option<(f32, f32)> = None;
        for &e in edges {
            for &t in targets {
                let d = t - e;
                if d.abs() <= tol && best.is_none_or(|(bd, _)| d.abs() < bd.abs()) {
                    best = Some((d, t));
                }
            }
        }
        best
    }

    fn apply_drag(&mut self, pointer: [f32; 2], view: &View, shift: bool, no_snap: bool) {
        let Some(drag) = self.drag.clone() else { return };
        let dx = pointer[0] - drag.pointer0[0];
        let dy = pointer[1] - drag.pointer0[1];
        let s = drag.start;
        let tol = SNAP_PX / view.scale;
        let snap = self.snap && !no_snap;
        let grid = self.grid.max(1.0);
        let (xs, ys) = self.snap_targets(&drag.id);
        let mut guides = vec![];
        let mut r = s;
        match drag.mode {
            DragMode::Move => {
                r.x = s.x + dx;
                r.y = s.y + dy;
                if snap {
                    match Self::best_snap(&[r.x, r.x + r.w / 2.0, r.x + r.w], &xs, tol) {
                        Some((d, t)) => {
                            r.x += d;
                            guides.push(Guide { vertical: true, pos: t });
                        }
                        None if self.show_grid => r.x = (r.x / grid).round() * grid,
                        None => {}
                    }
                    match Self::best_snap(&[r.y, r.y + r.h / 2.0, r.y + r.h], &ys, tol) {
                        Some((d, t)) => {
                            r.y += d;
                            guides.push(Guide { vertical: false, pos: t });
                        }
                        None if self.show_grid => r.y = (r.y / grid).round() * grid,
                        None => {}
                    }
                }
            }
            DragMode::Resize(h) => {
                let (mut l, mut t, mut rr, mut b) = (s.x, s.y, s.right(), s.bottom());
                let west = matches!(h, Handle::W | Handle::NW | Handle::SW);
                let east = matches!(h, Handle::E | Handle::NE | Handle::SE);
                let north = matches!(h, Handle::N | Handle::NE | Handle::NW);
                let south = matches!(h, Handle::S | Handle::SE | Handle::SW);
                let snap1 = |v: f32, targets: &[f32], vertical: bool, guides: &mut Vec<Guide>| -> f32 {
                    if !snap {
                        return v;
                    }
                    if let Some((d, t)) = Self::best_snap(&[v], targets, tol) {
                        guides.push(Guide { vertical, pos: t });
                        v + d
                    } else if self.show_grid {
                        (v / grid).round() * grid
                    } else {
                        v
                    }
                };
                if west {
                    l = snap1(s.x + dx, &xs, true, &mut guides);
                }
                if east {
                    rr = snap1(s.right() + dx, &xs, true, &mut guides);
                }
                if north {
                    t = snap1(s.y + dy, &ys, false, &mut guides);
                }
                if south {
                    b = snap1(s.bottom() + dy, &ys, false, &mut guides);
                }
                let min = 8.0;
                if rr - l < min {
                    if west { l = rr - min } else { rr = l + min }
                }
                if b - t < min {
                    if north { t = b - min } else { b = t + min }
                }
                // Shift on a corner handle keeps the aspect ratio.
                if shift && (west || east) && (north || south) && s.h > 0.0 {
                    let aspect = s.w / s.h;
                    let w = rr - l;
                    let hh = w / aspect;
                    if north { t = b - hh } else { b = t + hh }
                }
                r = R::new(l, t, rr - l, b - t);
            }
        }
        self.guides = guides;
        if let Some(i) = self.scene().find(&drag.id) {
            let e = &mut self.scene_mut().elements[i];
            e.x = r.x.round();
            e.y = r.y.round();
            e.w = r.w.round().max(1.0);
            e.h = r.h.round().max(1.0);
        }
    }

    fn hit_test(&self, p: [f32; 2]) -> Option<String> {
        self.scene().elements.iter().rev().find(|e| e.visible && R::of(e).contains(p)).map(|e| e.id.clone())
    }

    pub(super) fn canvas(&mut self, ui: &mut egui::Ui) {
        let avail = ui.available_rect_before_wrap();
        let resp = ui.allocate_rect(avail, Sense::click_and_drag());
        let view = self.view_for(avail);
        let painter = ui.painter_at(avail);
        let (cw, ch) = (self.project.canvas.width as f32, self.project.canvas.height as f32);
        let canvas_rect = view.rect(&R::new(0.0, 0.0, cw, ch));
        let (shift, alt) = ui.input(|i| (i.modifiers.shift, i.modifiers.alt));

        // ---------------- interaction
        let hover = resp.hover_pos();
        if resp.drag_started() {
            self.checkpoint();
            let press = ui.input(|i| i.pointer.press_origin()).or(resp.interact_pointer_pos());
            if let Some(press) = press {
                let pc = view.to_canvas(press);
                let mut started = false;
                if let Some(i) = self.selected_index() {
                    let e = &self.scene().elements[i];
                    if !e.locked {
                        let sr = view.rect(&R::of(e));
                        if let Some((h, _)) =
                            handles(sr).into_iter().find(|(_, hp)| hp.distance(press) <= HANDLE_PX + 2.0)
                        {
                            self.drag = Some(Drag {
                                id: e.id.clone(),
                                mode: DragMode::Resize(h),
                                start: R::of(e),
                                pointer0: pc,
                            });
                            started = true;
                        }
                    }
                }
                if !started {
                    // Prefer the already-selected element when it is under the pointer.
                    let sel_hit = self
                        .selected_index()
                        .filter(|&i| R::of(&self.scene().elements[i]).contains(pc))
                        .map(|i| self.scene().elements[i].id.clone());
                    match sel_hit.or_else(|| self.hit_test(pc)) {
                        Some(id) => {
                            let i = self.scene().find(&id).unwrap();
                            let e = &self.scene().elements[i];
                            if !e.locked {
                                self.drag =
                                    Some(Drag { id: id.clone(), mode: DragMode::Move, start: R::of(e), pointer0: pc });
                            }
                            self.selected = Some(id);
                        }
                        None => self.selected = None,
                    }
                }
            }
        }
        if resp.dragged()
            && self.drag.is_some()
            && let Some(p) = resp.interact_pointer_pos()
        {
            self.apply_drag(view.to_canvas(p), &view, shift, alt);
        }
        if resp.drag_stopped() {
            self.drag = None;
            self.guides.clear();
        }
        if resp.clicked()
            && let Some(p) = resp.interact_pointer_pos()
        {
            self.selected = self.hit_test(view.to_canvas(p));
        }
        if resp.double_clicked()
            && let Some(p) = resp.interact_pointer_pos()
            && self.hit_test(view.to_canvas(p)).is_none()
        {
            self.zoom = 1.0;
        }
        // Ctrl + wheel zoom
        if resp.hovered() {
            let (zoom_delta, cmd) = ui.input(|i| (i.zoom_delta(), i.modifiers.command));
            if zoom_delta != 1.0 && cmd {
                self.zoom = (self.zoom * zoom_delta).clamp(0.25, 3.0);
            }
        }
        // drop from palette
        if let Some(kind) = resp.dnd_release_payload::<ElementKind>()
            && let Some(p) = ui.input(|i| i.pointer.interact_pos())
        {
            let c = view.to_canvas(p);
            if c[0] >= 0.0 && c[1] >= 0.0 && c[0] <= cw && c[1] <= ch {
                self.add_element(*kind, Some(c));
            } else {
                self.add_element(*kind, None);
            }
        }

        // ---------------- painting
        painter.rect_filled(canvas_rect.translate(vec2(5.0, 6.0)), 2.0, Color32::from_black_alpha(120));
        let opts = DrawOptions { annotations: self.annotations, show_hidden: true };
        let prims = scene_prims(&self.project, self.scene(), &opts);
        let cpainter = painter.with_clip_rect(canvas_rect.intersect(avail));
        paint_prims(&cpainter, &prims, &view);

        if self.show_grid {
            let step = self.grid.max(1.0);
            if step * view.scale >= 6.0 {
                let gc = Color32::from_rgba_unmultiplied(80, 120, 200, 40);
                let mut x = step;
                while x < cw {
                    let a = view.pt(x, 0.0);
                    cpainter.line_segment([a, pos2(a.x, canvas_rect.bottom())], Stroke::new(1.0, gc));
                    x += step;
                }
                let mut y = step;
                while y < ch {
                    let a = view.pt(0.0, y);
                    cpainter.line_segment([a, pos2(canvas_rect.right(), a.y)], Stroke::new(1.0, gc));
                    y += step;
                }
            }
        }
        if self.show_safe {
            paint_prims(&cpainter, &safe_guides(&self.project), &view);
            let u = unit(&self.project);
            let lbl = |m: f32, s: &str, c: Color32| {
                cpainter.text(
                    view.pt(cw * m + 6.0 * u, ch * m + 4.0 * u),
                    egui::Align2::LEFT_TOP,
                    s,
                    egui::FontId::proportional(11.0),
                    c,
                );
            };
            lbl(ACTION_SAFE, "action-safe 5%", Color32::from_rgb(0, 150, 255));
            lbl(TITLE_SAFE, "title-safe 10%", Color32::from_rgb(255, 80, 160));
        }
        painter.rect_stroke(canvas_rect, 0.0, Stroke::new(1.0, Color32::from_gray(90)), StrokeKind::Outside);

        // hover outline
        if self.drag.is_none()
            && let Some(hp) = hover
            && let Some(id) = self.hit_test(view.to_canvas(hp))
            && Some(&id) != self.selected.as_ref()
            && let Some(i) = self.scene().find(&id)
        {
            let r = view.rect(&R::of(&self.scene().elements[i]));
            cpainter.rect_stroke(r, 0.0, Stroke::new(1.0, Color32::from_rgb(120, 180, 255)), StrokeKind::Outside);
        }

        // selection
        if let Some(i) = self.selected_index() {
            let e = &self.scene().elements[i];
            let r = view.rect(&R::of(e));
            let col = if e.locked { Color32::from_rgb(160, 160, 160) } else { super::ACCENT };
            painter.rect_stroke(r, 0.0, Stroke::new(2.0, col), StrokeKind::Outside);
            if !e.locked {
                let mut cursor = None;
                for (h, p) in handles(r) {
                    let hr = Rect::from_center_size(p, vec2(HANDLE_PX * 1.6, HANDLE_PX * 1.6));
                    painter.rect_filled(hr, 1.5, Color32::WHITE);
                    painter.rect_stroke(hr, 1.5, Stroke::new(1.5, col), StrokeKind::Inside);
                    if let Some(hp) = hover
                        && hp.distance(p) <= HANDLE_PX + 2.0
                    {
                        cursor = Some(handle_cursor(h));
                    }
                }
                if let Some(Drag { mode: DragMode::Resize(h), .. }) = &self.drag {
                    cursor = Some(handle_cursor(*h));
                }
                if let Some(c) = cursor {
                    ui.ctx().set_cursor_icon(c);
                } else if hover.is_some_and(|hp| r.contains(hp)) {
                    ui.ctx().set_cursor_icon(if self.drag.is_some() { CursorIcon::Grabbing } else { CursorIcon::Grab });
                }
            }
            // geometry readout
            let txt = format!("{} · x{} y{} · {}×{}", e.id, e.x, e.y, e.w, e.h);
            let g = painter.layout_no_wrap(txt, egui::FontId::proportional(11.0), Color32::WHITE);
            let at = pos2(r.left(), (r.bottom() + 6.0).min(avail.bottom() - g.size().y - 4.0));
            painter.rect_filled(Rect::from_min_size(at, g.size()).expand(3.0), 3.0, Color32::from_black_alpha(190));
            painter.galley(at, g, Color32::WHITE);
        }

        // snap guides
        for g in &self.guides {
            let stroke = Stroke::new(1.0, Color32::from_rgb(255, 60, 200));
            let (a, b) = if g.vertical {
                (pos2(view.pt(g.pos, 0.0).x, canvas_rect.top()), pos2(view.pt(g.pos, 0.0).x, canvas_rect.bottom()))
            } else {
                (pos2(canvas_rect.left(), view.pt(0.0, g.pos).y), pos2(canvas_rect.right(), view.pt(0.0, g.pos).y))
            };
            painter.add(Shape::line_segment([a, b], stroke));
        }

        // drop preview
        if let Some(kind) = resp.dnd_hover_payload::<ElementKind>()
            && let Some(p) = ui.input(|i| i.pointer.hover_pos())
        {
            let c = view.to_canvas(p);
            let e = wvs::model::Element::new(*kind, "preview".into(), cw, ch, c[0], c[1]);
            let mut prims = vec![];
            element_prims(
                &e,
                1.0,
                unit(&self.project),
                &DrawOptions { annotations: false, show_hidden: true },
                &mut prims,
            );
            paint_prims(&cpainter, &prims, &view);
            cpainter.rect_stroke(view.rect(&R::of(&e)), 0.0, Stroke::new(2.0, super::ACCENT), StrokeKind::Outside);
        }

        // pointer coordinates
        if let Some(hp) = hover {
            let c = view.to_canvas(hp);
            if c[0] >= 0.0 && c[1] >= 0.0 && c[0] <= cw && c[1] <= ch {
                painter.text(
                    avail.right_bottom() - vec2(8.0, 6.0),
                    egui::Align2::RIGHT_BOTTOM,
                    format!("x {:.0}  y {:.0}", c[0], c[1]),
                    egui::FontId::monospace(11.0),
                    c32([200, 200, 200, 255]),
                );
            }
        }
        // scene caption
        let sc = self.scene();
        painter.text(
            avail.left_top() + vec2(10.0, 8.0),
            egui::Align2::LEFT_TOP,
            format!(
                "場景 {} / {} · {} · {:.1}s · {}×{} · {:.0}%",
                self.scene + 1,
                self.project.scenes.len(),
                sc.name,
                sc.duration,
                cw,
                ch,
                view.scale * 100.0
            ),
            egui::FontId::proportional(12.0),
            Color32::from_gray(170),
        );
    }
}
