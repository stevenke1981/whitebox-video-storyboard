//! Side panels: component palette + canvas settings, inspector + layers, scene strip.

use super::App;
use super::paint::{View, paint_prims};
use eframe::egui::{self, Color32, RichText, Sense, Stroke, StrokeKind, vec2};
use wvs::draw::{DrawOptions, scene_prims};
use wvs::model::{Align, Animation, Direction, ElementKind, PRESETS, Scene, ShapeKind};

fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(title).strong().color(super::ACCENT));
}

impl App {
    pub(super) fn palette(&mut self, ui: &mut egui::Ui) {
        section(ui, "元件庫");
        ui.label(RichText::new("拖曳到畫布，或點一下加在畫面中央").small().weak());
        let mut clicked = None;
        for group in ["文字", "互動", "媒體", "版面"] {
            ui.add_space(2.0);
            ui.label(RichText::new(group).small().color(Color32::from_gray(150)));
            for kind in ElementKind::ALL.iter().copied().filter(|k| k.info().group == group) {
                let info = kind.info();
                let id = egui::Id::new(("palette", kind.key()));
                let r = ui.dnd_drag_source(id, kind, |ui| {
                    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
                    let hovered = resp.hovered();
                    let p = ui.painter();
                    p.rect_filled(rect, 4.0, if hovered { Color32::from_gray(70) } else { Color32::from_gray(52) });
                    let icon = egui::Rect::from_min_size(rect.min + vec2(4.0, 4.0), vec2(22.0, 22.0));
                    p.rect_filled(icon, 3.0, Color32::from_gray(185));
                    p.text(
                        icon.center(),
                        egui::Align2::CENTER_CENTER,
                        info.icon,
                        egui::FontId::proportional(14.0),
                        Color32::from_gray(40),
                    );
                    p.text(
                        rect.min + vec2(34.0, 15.0),
                        egui::Align2::LEFT_CENTER,
                        info.zh,
                        egui::FontId::proportional(14.0),
                        Color32::from_gray(235),
                    );
                    p.text(
                        rect.right_center() - vec2(6.0, 0.0),
                        egui::Align2::RIGHT_CENTER,
                        info.en,
                        egui::FontId::proportional(10.5),
                        Color32::from_gray(150),
                    );
                    resp
                });
                if r.inner.clicked() {
                    clicked = Some(kind);
                }
                r.response.on_hover_text(format!("{} / {}\nMoviePy：{}", info.zh, info.en, info.moviepy));
            }
        }
        if let Some(k) = clicked {
            self.add_element(k, None);
        }

        ui.separator();
        section(ui, "畫布");
        let cur = self.project.canvas.preset.clone();
        let label = PRESETS
            .iter()
            .find(|p| p.key == cur)
            .map(|p| p.label.to_string())
            .unwrap_or_else(|| format!("自訂 {}×{}", self.project.canvas.width, self.project.canvas.height));
        egui::ComboBox::from_id_salt("preset").selected_text(label).width(ui.available_width() - 8.0).show_ui(
            ui,
            |ui| {
                for p in PRESETS {
                    if ui.selectable_label(cur == p.key, p.label).clicked() && cur != p.key {
                        self.project.resize_canvas(p.width, p.height, p.key);
                        self.checkpoint();
                    }
                }
            },
        );
        ui.horizontal(|ui| {
            let (mut w, mut h) = (self.project.canvas.width, self.project.canvas.height);
            ui.label("寬");
            let rw = ui.add(egui::DragValue::new(&mut w).range(16..=8192));
            ui.label("高");
            let rh = ui.add(egui::DragValue::new(&mut h).range(16..=8192));
            if (rw.lost_focus() || rw.drag_stopped() || rh.lost_focus() || rh.drag_stopped())
                && (w, h) != (self.project.canvas.width, self.project.canvas.height)
            {
                self.project.resize_canvas(w, h, "");
                self.checkpoint();
            }
        });
        ui.horizontal(|ui| {
            ui.label("FPS");
            ui.add(egui::DragValue::new(&mut self.project.fps).range(1..=120));
            ui.label("專案名");
        });
        ui.text_edit_singleline(&mut self.project.name);
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_grid, "格線");
            ui.add(egui::DragValue::new(&mut self.grid).range(5.0..=400.0).suffix(" px"));
        });
        ui.checkbox(&mut self.snap, "吸附（Alt 暫停）");
        ui.checkbox(&mut self.show_safe, "Title / Action safe");
        ui.checkbox(&mut self.annotations, "類型 / 時間標籤");
    }

    pub(super) fn inspector(&mut self, ui: &mut egui::Ui) {
        // ---- scene settings
        let has_sel = self.selected_index().is_some();
        egui::CollapsingHeader::new(RichText::new(format!("場景設定 #{}", self.scene + 1)).strong())
            .id_salt(("scene_settings", self.scene))
            .default_open(!has_sel)
            .show(ui, |ui| {
                let s = self.scene_mut();
                egui::Grid::new("scene_grid").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                    ui.label("名稱");
                    ui.text_edit_singleline(&mut s.name);
                    ui.end_row();
                    ui.label("ID");
                    ui.text_edit_singleline(&mut s.id);
                    ui.end_row();
                    ui.label("長度");
                    ui.add(
                        egui::DragValue::new(&mut s.duration)
                            .range(0.1..=3600.0)
                            .speed(0.1)
                            .suffix(" 秒")
                            .max_decimals(2),
                    );
                    ui.end_row();
                    ui.label("背景色");
                    ui.color_edit_button_srgb(&mut s.background.0);
                    ui.end_row();
                });
                ui.label("備註（給 Agent）");
                ui.add(egui::TextEdit::multiline(&mut s.notes).desired_rows(2).desired_width(f32::INFINITY));
            });
        ui.separator();

        // ---- element properties
        let scene_dur = self.scene().duration;
        let canvas = (self.project.canvas.width as f32, self.project.canvas.height as f32);
        let mut taken_ids: Vec<String> =
            self.project.scenes.iter().flat_map(|s| s.elements.iter().map(|e| e.id.clone())).collect();
        let sel_id = self.selected.clone();
        if let Some(e) = self.selected_mut() {
            taken_ids.retain(|i| Some(i) != sel_id.as_ref());
            let info = e.kind.info();
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{} {}", info.icon, info.zh)).heading());
                ui.label(RichText::new(info.en).weak());
            });
            ui.label(RichText::new(format!("MoviePy：{}", info.moviepy)).small().weak());
            ui.add_space(4.0);
            let mut new_id = e.id.clone();
            egui::Grid::new("el_grid").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                ui.label("ID");
                let r = ui.text_edit_singleline(&mut new_id);
                if r.lost_focus() {
                    let clean: String =
                        new_id.trim().chars().map(|c| if c.is_whitespace() { '_' } else { c }).collect();
                    if !clean.is_empty() && !taken_ids.contains(&clean) {
                        e.id = clean;
                    }
                }
                ui.end_row();
                ui.label("名稱");
                ui.text_edit_singleline(&mut e.name);
                ui.end_row();
                ui.label("位置 x / y");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut e.x).speed(1.0).range(-8192.0..=16384.0));
                    ui.add(egui::DragValue::new(&mut e.y).speed(1.0).range(-8192.0..=16384.0));
                });
                ui.end_row();
                ui.label("大小 w / h");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut e.w).speed(1.0).range(1.0..=16384.0));
                    ui.add(egui::DragValue::new(&mut e.h).speed(1.0).range(1.0..=16384.0));
                });
                ui.end_row();
                ui.label("");
                ui.horizontal(|ui| {
                    if ui.small_button("水平置中").clicked() {
                        e.x = ((canvas.0 - e.w) / 2.0).round();
                    }
                    if ui.small_button("垂直置中").clicked() {
                        e.y = ((canvas.1 - e.h) / 2.0).round();
                    }
                    if ui.small_button("滿版").clicked() {
                        e.x = 0.0;
                        e.y = 0.0;
                        e.w = canvas.0;
                        e.h = canvas.1;
                    }
                });
                ui.end_row();
            });

            if e.kind.has_text() {
                section(ui, "文字");
                let hint = match e.kind {
                    ElementKind::LowerThird => "第一行姓名、第二行職稱",
                    ElementKind::Countdown => "起始數字，例如 10",
                    ElementKind::Sticker => "emoji / 圖示名稱",
                    ElementKind::Options => "（選項在下方編輯）",
                    _ => "內容（Enter 換行）",
                };
                if e.kind != ElementKind::Options {
                    ui.add(
                        egui::TextEdit::multiline(&mut e.text)
                            .hint_text(hint)
                            .desired_rows(2)
                            .desired_width(f32::INFINITY),
                    );
                }
                egui::Grid::new("text_grid").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                    ui.label("字級");
                    ui.add(egui::DragValue::new(&mut e.font_size).range(4.0..=1000.0).suffix(" px"));
                    ui.end_row();
                    ui.label("文字顏色");
                    ui.color_edit_button_srgb(&mut e.font_color.0);
                    ui.end_row();
                    ui.label("對齊");
                    ui.horizontal(|ui| {
                        for a in Align::ALL {
                            ui.selectable_value(&mut e.align, a, a.zh());
                        }
                    });
                    ui.end_row();
                    ui.label("描邊");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut e.stroke_width).range(0.0..=40.0).suffix(" px"));
                        ui.color_edit_button_srgb(&mut e.stroke_color.0);
                    });
                    ui.end_row();
                });
            } else if e.kind == ElementKind::ProgressBar {
                section(ui, "樣式");
                ui.horizontal(|ui| {
                    ui.label("進度顏色");
                    ui.color_edit_button_srgb(&mut e.font_color.0);
                });
            }

            if e.kind == ElementKind::Options {
                section(ui, "選項");
                let mut remove = None;
                let mut swap = None;
                let n = e.options.len();
                for i in 0..n {
                    ui.horizontal(|ui| {
                        let letter = (b'A' + (i as u8 % 26)) as char;
                        let correct = e.answer == Some(i);
                        if ui
                            .selectable_label(correct, format!("{letter}"))
                            .on_hover_text("標記為正確答案 / 強調")
                            .clicked()
                        {
                            e.answer = if correct { None } else { Some(i) };
                        }
                        ui.add(
                            egui::TextEdit::singleline(&mut e.options[i]).desired_width(ui.available_width() - 70.0),
                        );
                        if ui.small_button("▲").clicked() && i > 0 {
                            swap = Some(i);
                        }
                        if ui.small_button("✖").clicked() {
                            remove = Some(i);
                        }
                    });
                }
                if let Some(i) = swap {
                    e.options.swap(i - 1, i);
                }
                if let Some(i) = remove {
                    e.options.remove(i);
                    if e.answer == Some(i) {
                        e.answer = None;
                    }
                }
                if ui.button("＋ 新增選項").clicked() {
                    let letter = (b'A' + (e.options.len() as u8 % 26)) as char;
                    e.options.push(format!("選項 {letter}"));
                }
            }

            section(ui, "外觀");
            egui::Grid::new("look_grid").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                ui.label("底色");
                ui.horizontal(|ui| {
                    ui.color_edit_button_srgb(&mut e.bg_color.0);
                    ui.add(egui::Slider::new(&mut e.bg_opacity, 0.0..=1.0).text("不透明度"));
                });
                ui.end_row();
                if matches!(
                    e.kind,
                    ElementKind::Shape | ElementKind::AvatarFrame | ElementKind::Countdown | ElementKind::CtaButton
                ) {
                    ui.label("形狀");
                    ui.horizontal(|ui| {
                        for s in ShapeKind::ALL {
                            ui.selectable_value(&mut e.shape, s, s.zh());
                        }
                    });
                    ui.end_row();
                }
                if e.kind == ElementKind::CalloutArrow {
                    ui.label("箭頭方向");
                    egui::ComboBox::from_id_salt("dir").selected_text(e.direction.zh()).show_ui(ui, |ui| {
                        for d in Direction::ALL {
                            ui.selectable_value(&mut e.direction, d, d.zh());
                        }
                    });
                    ui.end_row();
                }
                if matches!(
                    e.kind,
                    ElementKind::MediaPlaceholder
                        | ElementKind::Logo
                        | ElementKind::AvatarFrame
                        | ElementKind::QrCode
                        | ElementKind::Sticker
                        | ElementKind::Background
                ) {
                    ui.label("素材路徑 src");
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut e.src).hint_text("assets/xxx.png").desired_width(150.0));
                        if ui.small_button("…").clicked()
                            && let Some(p) = rfd::FileDialog::new().pick_file()
                        {
                            e.src = p.display().to_string();
                        }
                    });
                    ui.end_row();
                }
            });

            section(ui, "時間與動畫");
            egui::Grid::new("time_grid").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                ui.label("開始");
                ui.add(
                    egui::DragValue::new(&mut e.start).range(0.0..=scene_dur).speed(0.05).suffix(" 秒").max_decimals(2),
                );
                ui.end_row();
                ui.label("結束");
                ui.horizontal(|ui| {
                    let mut until_end = e.end.is_none();
                    if ui.checkbox(&mut until_end, "到場景結束").changed() {
                        e.end = if until_end { None } else { Some(scene_dur) };
                    }
                    if let Some(end) = e.end.as_mut() {
                        ui.add(
                            egui::DragValue::new(end).range(0.0..=scene_dur).speed(0.05).suffix(" 秒").max_decimals(2),
                        );
                    }
                });
                ui.end_row();
                ui.label("動畫");
                egui::ComboBox::from_id_salt("anim")
                    .selected_text(format!("{}  {}", e.animation.zh(), e.animation.key()))
                    .show_ui(ui, |ui| {
                        for a in Animation::ALL {
                            ui.selectable_value(&mut e.animation, a, format!("{}  {}", a.zh(), a.key()));
                        }
                    });
                ui.end_row();
                ui.label("圖層 z");
                ui.label(format!("{}", e.z));
                ui.end_row();
                ui.label("狀態");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut e.visible, "顯示");
                    ui.checkbox(&mut e.locked, "鎖定");
                });
                ui.end_row();
            });
            ui.label("備註（給 Agent 的指示）");
            ui.add(
                egui::TextEdit::multiline(&mut e.notes)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY)
                    .hint_text("例如：配合旁白第 2 句出現"),
            );
        } else {
            ui.label(RichText::new("未選取元件。點選畫布上的元件以編輯屬性。").weak());
        }

        // ---- layers
        ui.separator();
        section(ui, "圖層（上 = 最上層）");
        ui.horizontal(|ui| {
            let sel = self.selected_index().is_some();
            if ui.add_enabled(sel, egui::Button::new("置頂")).clicked() {
                self.move_layer(10_000);
            }
            if ui.add_enabled(sel, egui::Button::new("⬆")).clicked() {
                self.move_layer(1);
            }
            if ui.add_enabled(sel, egui::Button::new("⬇")).clicked() {
                self.move_layer(-1);
            }
            if ui.add_enabled(sel, egui::Button::new("置底")).clicked() {
                self.move_layer(-10_000);
            }
            if ui.add_enabled(sel, egui::Button::new("🗐")).on_hover_text("複製").clicked() {
                self.duplicate_selected();
            }
            if ui.add_enabled(sel, egui::Button::new("🗑")).on_hover_text("刪除").clicked() {
                self.delete_selected();
            }
        });
        let n = self.scene().elements.len();
        let mut select = None;
        for i in (0..n).rev() {
            let e = &mut self.project.scenes[self.scene].elements[i];
            let is_sel = self.selected.as_deref() == Some(e.id.as_str());
            ui.horizontal(|ui| {
                ui.checkbox(&mut e.visible, "").on_hover_text("顯示");
                if ui.selectable_label(e.locked, "鎖").on_hover_text("鎖定（不可拖曳）").clicked() {
                    e.locked = !e.locked;
                }
                let txt = format!("{} {}  ·  {}", e.kind.info().icon, e.display_name(), e.id);
                let mut rt = RichText::new(txt);
                if !e.visible {
                    rt = rt.weak();
                }
                if ui.selectable_label(is_sel, rt).clicked() {
                    select = Some(e.id.clone());
                }
            });
        }
        if let Some(id) = select {
            self.selected = Some(id);
        }
    }

    pub(super) fn scenes_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("場景 / 鏡頭").strong().color(super::ACCENT));
            if ui.button("＋ 新增場景").clicked() {
                let id = self.project.next_scene_id();
                let mut s = Scene::new(id, format!("場景 {}", self.project.scenes.len() + 1));
                s.background = self.scene().background;
                self.project.scenes.insert(self.scene + 1, s);
                self.scene += 1;
                self.selected = None;
                self.checkpoint();
            }
            if ui.button("🗐 複製場景").clicked() {
                let mut s = self.scene().clone();
                s.id = self.project.next_scene_id();
                s.name = format!("{}（複製）", s.name);
                for e in &mut s.elements {
                    e.id = format!("{}_{}", e.id, s.id);
                }
                self.project.scenes.insert(self.scene + 1, s);
                self.scene += 1;
                self.selected = None;
                self.checkpoint();
            }
            if ui.add_enabled(self.project.scenes.len() > 1, egui::Button::new("🗑 刪除場景")).clicked() {
                self.project.scenes.remove(self.scene);
                self.scene = self.scene.saturating_sub(1);
                self.selected = None;
                self.checkpoint();
            }
            if ui.add_enabled(self.scene > 0, egui::Button::new("◀ 前移")).clicked() {
                self.project.scenes.swap(self.scene, self.scene - 1);
                self.scene -= 1;
                self.checkpoint();
            }
            if ui.add_enabled(self.scene + 1 < self.project.scenes.len(), egui::Button::new("後移 ▶")).clicked() {
                self.project.scenes.swap(self.scene, self.scene + 1);
                self.scene += 1;
                self.checkpoint();
            }
            ui.label(RichText::new(format!("總長 {:.1} 秒", self.project.total_duration())).weak());
        });
        let avail_h = ui.available_height() - 4.0;
        let (cw, ch) = (self.project.canvas.width as f32, self.project.canvas.height as f32);
        let thumb_h = (avail_h - 40.0).clamp(40.0, 220.0);
        let thumb_w = thumb_h * cw / ch;
        let mut go = None;
        egui::ScrollArea::horizontal().auto_shrink([false, false]).show(ui, |ui| {
            ui.horizontal(|ui| {
                for i in 0..self.project.scenes.len() {
                    ui.vertical(|ui| {
                        ui.set_width(thumb_w.max(110.0));
                        let (rect, resp) = ui.allocate_exact_size(vec2(thumb_w, thumb_h), Sense::click());
                        let p = ui.painter_at(rect);
                        let view = View { origin: rect.min, scale: thumb_w / cw };
                        let prims = scene_prims(
                            &self.project,
                            &self.project.scenes[i],
                            &DrawOptions { annotations: false, show_hidden: false },
                        );
                        paint_prims(&p, &prims, &view);
                        let cur = i == self.scene;
                        p.rect_stroke(
                            rect,
                            2.0,
                            Stroke::new(
                                if cur { 3.0 } else { 1.0 },
                                if cur { super::ACCENT } else { Color32::from_gray(90) },
                            ),
                            StrokeKind::Outside,
                        );
                        if resp.clicked() {
                            go = Some(i);
                        }
                        let s = &mut self.project.scenes[i];
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("#{} {}", i + 1, s.name)).small().strong());
                            ui.add(
                                egui::DragValue::new(&mut s.duration)
                                    .range(0.1..=3600.0)
                                    .speed(0.1)
                                    .suffix("s")
                                    .max_decimals(1),
                            );
                        });
                    });
                    ui.add_space(6.0);
                }
            });
        });
        if let Some(i) = go
            && i != self.scene
        {
            self.scene = i;
            self.selected = None;
        }
    }
}
