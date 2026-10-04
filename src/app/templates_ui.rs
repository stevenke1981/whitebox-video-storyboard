//! Scene template library window, user templates and the language menu.

use super::paint::{View, paint_prims};
use super::settings::templates_dir;
use super::{ACCENT, App};
use eframe::egui::{self, Color32, RichText, Sense, Stroke, StrokeKind, vec2};
use std::path::PathBuf;
use wvs::draw::{DrawOptions, scene_prims};
use wvs::i18n::{Lang, set_lang, t};
use wvs::model::{Project, Scene};
use wvs::templates::{Template, UserTemplate, build, file_stem};

const CARD_W: f32 = 168.0;

/// Paint a small preview of `scene`; returns the click response.
fn thumbnail(ui: &mut egui::Ui, p: &Project, scene: &Scene, max_w: f32, max_h: f32) -> egui::Response {
    let (cw, ch) = (p.canvas.width as f32, p.canvas.height as f32);
    let scale = (max_w / cw).min(max_h / ch);
    let size = vec2(cw * scale, ch * scale);
    let (outer, resp) = ui.allocate_exact_size(vec2(max_w, max_h), Sense::click());
    let rect = egui::Rect::from_center_size(outer.center(), size);
    let painter = ui.painter_at(rect);
    let prims = scene_prims(p, scene, &DrawOptions { annotations: false, show_hidden: false });
    paint_prims(&painter, &prims, &View { origin: rect.min, scale });
    let hovered = resp.hovered();
    ui.painter().rect_stroke(
        rect,
        2.0,
        Stroke::new(if hovered { 2.0 } else { 1.0 }, if hovered { ACCENT } else { Color32::from_gray(90) }),
        StrokeKind::Outside,
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn load_user_templates() -> Vec<(PathBuf, UserTemplate)> {
    let Some(dir) = templates_dir() else { return vec![] };
    let Ok(rd) = std::fs::read_dir(&dir) else { return vec![] };
    let mut v: Vec<(PathBuf, UserTemplate)> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .filter_map(|p| {
            let s = std::fs::read_to_string(&p).ok()?;
            let tpl: UserTemplate = serde_json::from_str(&s).ok()?;
            Some((p, tpl))
        })
        .collect();
    v.sort_by_key(|a| a.1.name.to_lowercase());
    v
}

impl App {
    /// Insert a scene after the current one and select it.
    pub fn insert_scene(&mut self, s: Scene) {
        let name = s.name.clone();
        let at = (self.scene + 1).min(self.project.scenes.len());
        self.project.scenes.insert(at, s);
        self.scene = at;
        self.selected = None;
        self.project.normalize();
        self.checkpoint();
        self.set_status(true, wvs::tf!("已插入場景 #{}「{}」", "Inserted scene #{} \"{}\"", at + 1, name));
    }

    pub fn insert_template(&mut self, tpl: Template) {
        let s = build(tpl, &self.project);
        self.insert_scene(s);
    }

    pub fn open_templates(&mut self) {
        self.user_templates = load_user_templates();
        self.show_templates = true;
    }

    pub fn save_user_template(&mut self) {
        let name =
            if self.tpl_name.trim().is_empty() { self.scene().name.clone() } else { self.tpl_name.trim().to_string() };
        let Some(dir) = templates_dir() else {
            self.set_status(false, t("找不到使用者設定資料夾", "Cannot find the user config folder"));
            return;
        };
        let tpl = UserTemplate::from_scene(&name, &self.project, self.scene());
        let path = dir.join(format!("{}.json", file_stem(&name)));
        let res = std::fs::create_dir_all(&dir)
            .and_then(|_| std::fs::write(&path, serde_json::to_string_pretty(&tpl).unwrap_or_default()));
        match res {
            Ok(()) => {
                self.set_status(
                    true,
                    wvs::tf!("已儲存範本「{}」→ {}", "Saved template \"{}\" → {}", name, path.display()),
                );
                self.tpl_name.clear();
            }
            Err(e) => self.set_status(false, wvs::tf!("儲存範本失敗：{}", "Saving the template failed: {}", e)),
        }
        self.user_templates = load_user_templates();
    }

    pub fn set_language(&mut self, l: Lang) {
        set_lang(l);
        self.settings.lang = Some(l);
        self.settings.save();
        self.set_status(
            true,
            t(
                "介面語言：繁體中文（新元件、範本與匯出文件會使用中文）",
                "Language: English (new elements, templates and exported docs will use English)",
            ),
        );
    }

    pub(super) fn language_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button(t("🌐 語言 Language", "🌐 Language 語言"), |ui| {
            let cur = wvs::i18n::lang();
            for l in Lang::ALL {
                if ui.radio(cur == l, l.native()).clicked() {
                    ui.close();
                    if cur != l {
                        self.set_language(l);
                    }
                }
            }
            ui.separator();
            ui.label(
                RichText::new(t(
                    "已輸入的文字不會被翻譯；新元件、範本與匯出文件會跟隨語言。",
                    "Existing texts are not translated; new elements, templates and exported docs follow the language.",
                ))
                .small()
                .weak(),
            );
        });
    }

    pub(super) fn templates_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button(t("範本", "Templates"), |ui| {
            if ui.button(t("🎬 範本庫…            Ctrl+T", "🎬 Template library…  Ctrl+T")).clicked() {
                ui.close();
                self.open_templates();
            }
            ui.separator();
            ui.label(RichText::new(t("插入到目前場景之後：", "Insert after the current scene:")).small().weak());
            for tpl in Template::ALL {
                if ui.button(tpl.label()).on_hover_text(tpl.description()).clicked() {
                    ui.close();
                    self.insert_template(tpl);
                }
            }
            ui.separator();
            if ui.button(t("💾 將目前場景存為範本…", "💾 Save current scene as template…")).clicked() {
                ui.close();
                self.tpl_name = self.scene().name.clone();
                self.open_templates();
            }
        });
    }

    pub(super) fn templates_window(&mut self, ctx: &egui::Context) {
        if !self.show_templates {
            return;
        }
        let mut open = true;
        let mut insert: Option<Template> = None;
        let mut insert_user: Option<usize> = None;
        let mut delete_user: Option<usize> = None;
        let mut save = false;
        let (cw, ch) = (self.project.canvas.width as f32, self.project.canvas.height as f32);
        let thumb_h = if ch > cw * 1.05 { 150.0 } else { 96.0 };
        egui::Window::new(t("🎬 場景範本庫", "🎬 Scene template library"))
            .open(&mut open)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .default_size([900.0, 700.0])
            .min_height(420.0)
            .resizable(true)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(wvs::tf!(
                        "點縮圖或「插入」把範本加到目前場景（#{}）之後。範本會依目前比例 {}（{}×{}）自動排版。",
                        "Click a thumbnail or \"Insert\" to add the template after the current scene (#{}). Templates adapt to the current aspect {} ({}×{}).",
                        self.scene + 1,
                        self.project.canvas.preset,
                        self.project.canvas.width,
                        self.project.canvas.height
                    ))
                    .weak(),
                );
                ui.add_space(4.0);
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.label(RichText::new(t("內建範本", "Built-in templates")).strong().color(ACCENT));
                    let cols = ((ui.available_width() / (CARD_W + 12.0)).floor() as usize).max(1);
                    egui::Grid::new("tpl_grid").spacing([12.0, 12.0]).show(ui, |ui| {
                        for (n, tpl) in Template::ALL.into_iter().enumerate() {
                            let scene = build(tpl, &self.project);
                            ui.vertical(|ui| {
                                ui.set_width(CARD_W);
                                if thumbnail(ui, &self.project, &scene, CARD_W, thumb_h).on_hover_text(tpl.description()).clicked() {
                                    insert = Some(tpl);
                                }
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(tpl.label()).strong());
                                    if !scene.transition.is_none() {
                                        ui.label(RichText::new("⇨").color(Color32::from_rgb(110, 170, 255)))
                                            .on_hover_text(wvs::tf!(
                                                "轉場：{} {:.1}s",
                                                "Transition: {} {:.1}s",
                                                scene.transition.kind.label(),
                                                scene.transition.duration
                                            ));
                                    }
                                });
                                ui.add(egui::Label::new(RichText::new(tpl.description()).small().weak()).wrap());
                                if ui.small_button(t("＋ 插入", "＋ Insert")).clicked() {
                                    insert = Some(tpl);
                                }
                            });
                            if n % cols == cols - 1 {
                                ui.end_row();
                            }
                        }
                    });
                    ui.add_space(10.0);
                    ui.separator();
                    ui.label(RichText::new(t("我的範本", "My templates")).strong().color(ACCENT));
                    ui.horizontal(|ui| {
                        ui.label(t("名稱", "Name"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.tpl_name)
                                .hint_text(t("範本名稱（預設＝場景名稱）", "Template name (default = scene name)"))
                                .desired_width(240.0),
                        );
                        if ui.button(t("💾 將目前場景存為範本", "💾 Save current scene as template")).clicked() {
                            save = true;
                        }
                    });
                    if let Some(d) = templates_dir() {
                        ui.label(RichText::new(wvs::tf!("儲存位置：{}", "Stored in: {}", d.display())).small().weak());
                    }
                    if self.user_templates.is_empty() {
                        ui.label(RichText::new(t("（尚無自訂範本）", "(no saved templates yet)")).weak());
                    } else {
                        egui::Grid::new("user_tpl_grid").spacing([12.0, 12.0]).show(ui, |ui| {
                            for (n, (_, ut)) in self.user_templates.iter().enumerate() {
                                let scene = ut.instantiate(&self.project);
                                ui.vertical(|ui| {
                                    ui.set_width(CARD_W);
                                    if thumbnail(ui, &self.project, &scene, CARD_W, thumb_h).clicked() {
                                        insert_user = Some(n);
                                    }
                                    ui.label(RichText::new(&ut.name).strong());
                                    ui.label(
                                        RichText::new(wvs::tf!(
                                            "{} 個元件・{}×{}",
                                            "{} elements · {}×{}",
                                            ut.scene.elements.len(),
                                            ut.canvas.width,
                                            ut.canvas.height
                                        ))
                                        .small()
                                        .weak(),
                                    );
                                    ui.horizontal(|ui| {
                                        if ui.small_button(t("＋ 插入", "＋ Insert")).clicked() {
                                            insert_user = Some(n);
                                        }
                                        if ui.small_button(t("🗑 刪除", "🗑 Delete")).clicked() {
                                            delete_user = Some(n);
                                        }
                                    });
                                });
                                if n % cols == cols - 1 {
                                    ui.end_row();
                                }
                            }
                        });
                    }
                });
            });
        self.show_templates = open;
        if let Some(tpl) = insert {
            self.insert_template(tpl);
        }
        if let Some(n) = insert_user
            && let Some((_, ut)) = self.user_templates.get(n)
        {
            let s = ut.instantiate(&self.project);
            self.insert_scene(s);
        }
        if let Some(n) = delete_user
            && let Some((path, ut)) = self.user_templates.get(n).cloned()
        {
            match std::fs::remove_file(&path) {
                Ok(()) => self.set_status(true, wvs::tf!("已刪除範本「{}」", "Deleted template \"{}\"", ut.name)),
                Err(e) => self.set_status(false, wvs::tf!("刪除失敗：{}", "Delete failed: {}", e)),
            }
            self.user_templates = load_user_templates();
        }
        if save {
            self.save_user_template();
        }
    }
}
