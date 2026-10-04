//! The egui editor: palette (left), canvas (centre), properties & layers (right), scenes (bottom).

mod canvas;
mod paint;
mod panels;
mod settings;

use eframe::egui::{self, Color32, RichText};
use settings::Settings;
use std::path::PathBuf;
use std::sync::Arc;
use wvs::export::{ExportOptions, Formats, export_to, project_dir_name};
use wvs::model::{Element, ElementKind, Project, Scene};

pub const ACCENT: Color32 = Color32::from_rgb(240, 128, 20);
pub const OK_GREEN: Color32 = Color32::from_rgb(90, 200, 120);
pub const ERR_RED: Color32 = Color32::from_rgb(240, 90, 90);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handle {
    N,
    S,
    E,
    W,
    NE,
    NW,
    SE,
    SW,
}

#[derive(Clone, Debug)]
pub enum DragMode {
    Move,
    Resize(Handle),
}

#[derive(Clone, Debug)]
pub struct Drag {
    pub id: String,
    pub mode: DragMode,
    pub start: wvs::draw::R,
    pub pointer0: [f32; 2],
}

/// An active snap guide line (canvas px): vertical if `vertical`, else horizontal.
#[derive(Clone, Copy, Debug)]
pub struct Guide {
    pub vertical: bool,
    pub pos: f32,
}

pub struct App {
    pub project: Project,
    pub path: Option<PathBuf>,
    pub scene: usize,
    pub selected: Option<String>,
    undo: Vec<Project>,
    redo: Vec<Project>,
    last_change: f64,
    force_checkpoint: bool,
    history_jump: bool,
    pub dirty: bool,
    pub show_grid: bool,
    pub snap: bool,
    pub show_safe: bool,
    pub annotations: bool,
    pub grid: f32,
    pub zoom: f32,
    pub drag: Option<Drag>,
    pub guides: Vec<Guide>,
    pub status: Option<(bool, String)>,
    pub export_opts: ExportOptions,
    pub settings: Settings,
    pub show_export: bool,
    pub export_base: String,
    pub last_export: Option<(PathBuf, usize)>,
    pub show_help: bool,
    pub show_about: bool,
    pub confirm_discard: Option<PendingAction>,
}

#[derive(Clone, Copy, Debug)]
pub enum PendingAction {
    New,
    Sample,
    Open,
}

fn setup_fonts(ctx: &egui::Context) {
    // eframe's `default_fonts` feature is off to save ~1.1 MB: the bundled CJK font
    // covers Latin + CJK text and egui's small icon font covers the toolbar icons.
    let mut fonts = egui::FontDefinitions::empty();
    fonts.font_data.insert("noto_cjk_tc".into(), Arc::new(egui::FontData::from_static(wvs::fonts::cjk_font())));
    fonts.font_data.insert(
        "emoji-icon-font".into(),
        Arc::new(
            egui::FontData::from_static(epaint_default_fonts::EMOJI_ICON)
                .tweak(egui::FontTweak { scale: 0.90, ..Default::default() }),
        ),
    );
    let mut chain = vec!["noto_cjk_tc".to_string(), "emoji-icon-font".to_string()];
    // Last-resort system font for characters outside the bundled subset.
    if let Some((bytes, index)) = wvs::fonts::load_system_fallback() {
        let mut fd = egui::FontData::from_owned(bytes);
        fd.index = index;
        fonts.font_data.insert("system_cjk".into(), Arc::new(fd));
        chain.push("system_cjk".into());
    }
    fonts.families.insert(egui::FontFamily::Proportional, chain.clone());
    fonts.families.insert(egui::FontFamily::Monospace, chain);
    ctx.set_fonts(fonts);
}

fn apply_theme(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = Color32::from_rgb(32, 34, 38);
    v.window_fill = Color32::from_rgb(40, 43, 48);
    v.extreme_bg_color = Color32::from_rgb(20, 21, 24);
    v.selection.bg_fill = Color32::from_rgb(150, 85, 25);
    v.selection.stroke = egui::Stroke::new(1.0, Color32::WHITE);
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ACCENT);
    v.slider_trailing_fill = true;
    ctx.set_visuals_of(egui::Theme::Dark, v);
    ctx.set_theme(egui::Theme::Dark);
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = egui::vec2(6.0, 5.0);
        s.spacing.button_padding = egui::vec2(6.0, 3.0);
    });
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, open: Option<String>) -> App {
        setup_fonts(&cc.egui_ctx);
        apply_theme(&cc.egui_ctx);
        let settings = Settings::load();
        let mut app = App {
            project: wvs::sample::sample_project("16:9"),
            path: None,
            scene: 0,
            selected: None,
            undo: vec![],
            redo: vec![],
            last_change: -10.0,
            force_checkpoint: false,
            history_jump: false,
            dirty: false,
            show_grid: true,
            snap: true,
            show_safe: true,
            annotations: true,
            grid: 40.0,
            zoom: 1.0,
            drag: None,
            guides: vec![],
            status: Some((true, "已載入範例專案。從左側拖曳元件到畫布開始編排。".into())),
            export_opts: settings.export.clone(),
            export_base: String::new(),
            settings,
            show_export: false,
            last_export: None,
            show_help: false,
            show_about: false,
            confirm_discard: None,
        };
        if let Some(p) = open {
            app.open_path(PathBuf::from(p));
        }
        app
    }

    // ------------------------------------------------------------------ helpers

    pub fn scene(&self) -> &Scene {
        &self.project.scenes[self.scene]
    }
    pub fn scene_mut(&mut self) -> &mut Scene {
        &mut self.project.scenes[self.scene]
    }
    pub fn selected_index(&self) -> Option<usize> {
        self.selected.as_ref().and_then(|id| self.scene().find(id))
    }
    pub fn selected_mut(&mut self) -> Option<&mut Element> {
        let i = self.selected_index()?;
        Some(&mut self.scene_mut().elements[i])
    }

    /// Start a new undo step even if the previous change was very recent.
    pub fn checkpoint(&mut self) {
        self.force_checkpoint = true;
    }

    pub fn set_status(&mut self, ok: bool, msg: impl Into<String>) {
        self.status = Some((ok, msg.into()));
    }

    pub fn undo(&mut self) {
        if let Some(p) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.project, p));
            self.history_jump = true;
            self.last_change = -10.0;
            self.clamp_scene();
        }
    }

    pub fn redo(&mut self) {
        if let Some(p) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.project, p));
            self.history_jump = true;
            self.last_change = -10.0;
            self.clamp_scene();
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    fn clamp_scene(&mut self) {
        if self.project.scenes.is_empty() {
            self.project.scenes.push(Scene::new("scene_1".into(), "場景 1".into()));
        }
        self.scene = self.scene.min(self.project.scenes.len() - 1);
    }

    pub fn add_element(&mut self, kind: ElementKind, center: Option<[f32; 2]>) {
        let (cw, ch) = (self.project.canvas.width as f32, self.project.canvas.height as f32);
        let c = center.unwrap_or([cw / 2.0, ch / 2.0]);
        let id = self.project.next_element_id(kind);
        let mut e = Element::new(kind, id.clone(), cw, ch, c[0], c[1]);
        e.x = e.x.clamp(0.0, (cw - e.w).max(0.0));
        e.y = e.y.clamp(0.0, (ch - e.h).max(0.0));
        if self.snap && kind != ElementKind::Background {
            e.x = (e.x / self.grid).round() * self.grid;
            e.y = (e.y / self.grid).round() * self.grid;
        }
        if kind == ElementKind::Background {
            e.locked = true;
            self.scene_mut().elements.insert(0, e);
        } else {
            self.scene_mut().elements.push(e);
        }
        self.selected = Some(id.clone());
        self.checkpoint();
        self.set_status(true, format!("已新增 {}（{}）", kind.info().zh, id));
    }

    pub fn delete_selected(&mut self) {
        if let Some(i) = self.selected_index() {
            let e = self.scene_mut().elements.remove(i);
            self.selected = None;
            self.checkpoint();
            self.set_status(true, format!("已刪除 {}", e.id));
        }
    }

    pub fn duplicate_selected(&mut self) {
        if let Some(i) = self.selected_index() {
            let mut e = self.scene().elements[i].clone();
            e.id = self.project.next_element_id(e.kind);
            e.x += 30.0;
            e.y += 30.0;
            e.locked = false;
            let id = e.id.clone();
            self.scene_mut().elements.insert(i + 1, e);
            self.selected = Some(id.clone());
            self.checkpoint();
            self.set_status(true, format!("已複製為 {id}"));
        }
    }

    /// Move the selected element in the stacking order: +1 up, -1 down, ±100 to top/bottom.
    pub fn move_layer(&mut self, delta: i32) {
        if let Some(i) = self.selected_index() {
            let n = self.scene().elements.len() as i32;
            let j = (i as i32 + delta).clamp(0, n - 1) as usize;
            if i != j {
                let e = self.scene_mut().elements.remove(i);
                self.scene_mut().elements.insert(j, e);
                self.checkpoint();
            }
        }
    }

    pub fn nudge(&mut self, dx: f32, dy: f32) {
        if let Some(e) = self.selected_mut()
            && !e.locked
        {
            e.x += dx;
            e.y += dy;
        }
    }

    // ------------------------------------------------------------------ files

    fn discard_ok(&mut self, action: PendingAction) -> bool {
        if self.dirty {
            self.confirm_discard = Some(action);
            false
        } else {
            true
        }
    }

    pub fn do_action(&mut self, action: PendingAction, force: bool) {
        if !force && !self.discard_ok(action) {
            return;
        }
        match action {
            PendingAction::New => {
                let preset = self.project.canvas.preset.clone();
                self.load_project(Project::new("未命名專案", &preset), None);
                self.set_status(true, "新專案");
            }
            PendingAction::Sample => {
                let preset = self.project.canvas.preset.clone();
                self.load_project(wvs::sample::sample_project(&preset), None);
                self.set_status(true, "已載入範例專案");
            }
            PendingAction::Open => {
                if let Some(p) = rfd::FileDialog::new().add_filter("專案 / layout JSON", &["json"]).pick_file() {
                    self.open_path(p);
                }
            }
        }
    }

    fn load_project(&mut self, p: Project, path: Option<PathBuf>) {
        self.project = p;
        self.path = path;
        self.scene = 0;
        self.selected = None;
        self.undo.clear();
        self.redo.clear();
        self.dirty = false;
        self.history_jump = true;
    }

    pub fn open_path(&mut self, path: PathBuf) {
        match std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|s| Project::from_json(&s)) {
            Ok(p) => {
                self.load_project(p, Some(path.clone()));
                self.set_status(true, format!("已開啟 {}", path.display()));
            }
            Err(e) => self.set_status(false, format!("開啟失敗：{e}")),
        }
    }

    pub fn save(&mut self, save_as: bool) {
        let path = match (&self.path, save_as) {
            (Some(p), false) => Some(p.clone()),
            _ => rfd::FileDialog::new().add_filter("專案 JSON", &["json"]).set_file_name("project.json").save_file(),
        };
        let Some(path) = path else { return };
        match std::fs::write(&path, self.project.to_json()) {
            Ok(()) => {
                self.path = Some(path.clone());
                self.dirty = false;
                self.set_status(true, format!("已儲存 {}", path.display()));
            }
            Err(e) => self.set_status(false, format!("儲存失敗：{e}")),
        }
    }

    /// Open the export dialog (Ctrl+E).
    pub fn export_dialog(&mut self) {
        if self.export_base.is_empty() {
            self.export_base = self.default_export_base().display().to_string();
        }
        self.show_export = true;
    }

    fn default_export_base(&self) -> PathBuf {
        if let Some(b) = self.settings.export_base.as_ref().filter(|b| b.is_dir()) {
            return b.clone();
        }
        if let Some(d) = self.path.as_ref().and_then(|p| p.parent()).filter(|d| d.is_dir()) {
            return d.to_path_buf();
        }
        if let Some(h) = settings::home_dir() {
            let docs = h.join("Documents");
            return if docs.is_dir() { docs } else { h };
        }
        PathBuf::from(".")
    }

    pub fn export_name(&self) -> String {
        project_dir_name(self.path.as_deref())
    }

    pub fn do_export(&mut self) {
        let base = PathBuf::from(self.export_base.trim());
        let raw = self.path.as_ref().filter(|_| !self.dirty).and_then(|p| std::fs::read_to_string(p).ok());
        match export_to(
            &self.project,
            raw.as_deref(),
            &base,
            &self.export_name(),
            self.settings.export_subdir,
            &self.export_opts,
        ) {
            Ok(rep) => {
                self.set_status(true, format!("匯出完成：{} 個檔案 → {}", rep.files.len(), rep.dir.display()));
                self.last_export = Some((rep.dir.clone(), rep.files.len()));
                self.settings.export = self.export_opts.clone();
                self.settings.export_base = Some(base);
                self.settings.save();
            }
            Err(e) => self.set_status(false, format!("匯出失敗：{e}")),
        }
    }

    fn export_window(&mut self, ctx: &egui::Context) {
        if !self.show_export {
            return;
        }
        let mut open = true;
        let mut do_export = false;
        egui::Window::new("匯出草稿").open(&mut open).resizable(false).collapsible(false).default_width(460.0).show(
            ctx,
            |ui| {
                ui.label(RichText::new("輸出格式").strong().color(ACCENT));
                egui::Grid::new("fmt_grid").num_columns(2).spacing([18.0, 4.0]).show(ui, |ui| {
                    for (n, (key, label, file)) in Formats::INFO.iter().enumerate() {
                        if let Some(b) = self.export_opts.formats.get_mut(key) {
                            ui.checkbox(b, *label).on_hover_text(*file);
                        }
                        if n % 2 == 1 {
                            ui.end_row();
                        }
                    }
                });
                ui.horizontal(|ui| {
                    if ui.small_button("全選").clicked() {
                        self.export_opts.formats = Formats::default();
                    }
                    if ui.small_button("全不選").clicked() {
                        self.export_opts.formats = Formats::NONE;
                    }
                    if ui
                        .small_button("只要給 Agent 的文字")
                        .on_hover_text("layout.json + AGENT_GUIDE.md + storyboard.md")
                        .clicked()
                    {
                        self.export_opts.formats = Formats { layout: true, guide: true, md: true, ..Formats::NONE };
                    }
                });
                ui.add_space(4.0);
                ui.label(RichText::new("草稿圖選項").strong().color(ACCENT));
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.export_opts.annotations, "類型 / 時間標籤");
                    ui.checkbox(&mut self.export_opts.safe_guides, "安全框參考線");
                });
                ui.add_space(4.0);
                ui.label(RichText::new("輸出位置").strong().color(ACCENT));
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.export_base).desired_width(330.0));
                    if ui.button("選擇…").clicked() {
                        let mut dlg = rfd::FileDialog::new().set_title("選擇匯出位置");
                        if !self.export_base.is_empty() {
                            dlg = dlg.set_directory(&self.export_base);
                        }
                        if let Some(d) = dlg.pick_folder() {
                            self.export_base = d.display().to_string();
                        }
                    }
                });
                ui.checkbox(&mut self.settings.export_subdir, "建立「專案名_日期_時間」子資料夾（建議）");
                let target = if self.settings.export_subdir {
                    PathBuf::from(&self.export_base).join(format!("{}_YYYYMMDD_HHMMSS", self.export_name()))
                } else {
                    PathBuf::from(&self.export_base)
                };
                ui.label(RichText::new(format!("將輸出到：{}", target.display())).small().weak());
                ui.separator();
                ui.horizontal(|ui| {
                    let ok = self.export_opts.formats != Formats::NONE && !self.export_base.trim().is_empty();
                    if ui.add_enabled(ok, egui::Button::new(RichText::new("⬇ 匯出").strong())).clicked() {
                        do_export = true;
                    }
                    if !ok {
                        ui.label(RichText::new("請至少選一種格式並指定位置").color(ERR_RED).small());
                    }
                });
                if let Some((dir, n)) = self.last_export.clone() {
                    ui.add_space(4.0);
                    ui.label(RichText::new(format!("✔ 已匯出 {n} 個檔案到：")).color(OK_GREEN));
                    ui.label(RichText::new(dir.display().to_string()).monospace());
                    ui.horizontal(|ui| {
                        if ui.button("開啟資料夾").clicked() {
                            settings::open_folder(&dir);
                        }
                        if ui.button("複製路徑").clicked() {
                            ui.ctx().copy_text(dir.display().to_string());
                        }
                    });
                }
            },
        );
        if do_export {
            self.do_export();
        }
        if !open {
            self.show_export = false;
            self.settings.export = self.export_opts.clone();
            self.settings.save();
        }
    }

    pub fn export_scene_png(&mut self) {
        let name = wvs::export::scene_png_name(self.scene, &self.scene().id);
        let Some(path) = rfd::FileDialog::new().add_filter("PNG", &["png"]).set_file_name(name).save_file() else {
            return;
        };
        let res = wvs::export::render_scene(&self.project, self.scene, &self.export_opts)
            .and_then(|pm| wvs::raster::save_png(&pm, &path));
        match res {
            Ok(()) => self.set_status(true, format!("已輸出 {}", path.display())),
            Err(e) => self.set_status(false, format!("輸出失敗：{e}")),
        }
    }

    // ------------------------------------------------------------------ input

    fn shortcuts(&mut self, ctx: &egui::Context) {
        use egui::{Key, KeyboardShortcut, Modifiers};
        let sc = |m, k| KeyboardShortcut::new(m, k);
        let (redo_a, redo_b, save, save_as, open, export, dup) = ctx.input_mut(|i| {
            (
                i.consume_shortcut(&sc(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z)),
                i.consume_shortcut(&sc(Modifiers::COMMAND, Key::Y)),
                i.consume_shortcut(&sc(Modifiers::COMMAND | Modifiers::SHIFT, Key::S)),
                i.consume_shortcut(&sc(Modifiers::COMMAND, Key::S)),
                i.consume_shortcut(&sc(Modifiers::COMMAND, Key::O)),
                i.consume_shortcut(&sc(Modifiers::COMMAND, Key::E)),
                i.consume_shortcut(&sc(Modifiers::COMMAND, Key::D)),
            )
        });
        let undo = ctx.input_mut(|i| i.consume_shortcut(&sc(Modifiers::COMMAND, Key::Z)));
        if redo_a || redo_b {
            self.redo();
        } else if undo {
            self.undo();
        }
        if save_as {
            self.save(true);
        } else if save {
            self.save(false);
        }
        if open {
            self.do_action(PendingAction::Open, false);
        }
        if export {
            self.export_dialog();
        }
        if dup {
            self.duplicate_selected();
        }
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let (del, up, down, left, right, shift, raise, lower) = ctx.input(|i| {
            (
                i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace),
                i.key_pressed(Key::ArrowUp),
                i.key_pressed(Key::ArrowDown),
                i.key_pressed(Key::ArrowLeft),
                i.key_pressed(Key::ArrowRight),
                i.modifiers.shift,
                i.key_pressed(Key::CloseBracket) || i.key_pressed(Key::PageUp),
                i.key_pressed(Key::OpenBracket) || i.key_pressed(Key::PageDown),
            )
        });
        if del {
            self.delete_selected();
        }
        let step = if shift { 10.0 } else { 1.0 };
        let (dx, dy) = ((right as i32 - left as i32) as f32 * step, (down as i32 - up as i32) as f32 * step);
        if dx != 0.0 || dy != 0.0 {
            self.nudge(dx, dy);
        }
        if raise {
            self.move_layer(1);
        }
        if lower {
            self.move_layer(-1);
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.selected = None;
        }
    }

    fn commit_history(&mut self, ctx: &egui::Context, before: Project) {
        if self.history_jump {
            self.history_jump = false;
            self.force_checkpoint = false;
            return;
        }
        if self.project != before {
            let now = ctx.input(|i| i.time);
            if self.force_checkpoint || now - self.last_change > 0.8 {
                self.undo.push(before);
                if self.undo.len() > 300 {
                    self.undo.remove(0);
                }
                self.redo.clear();
            }
            self.last_change = now;
            self.dirty = true;
        }
        self.force_checkpoint = false;
    }

    // ------------------------------------------------------------------ chrome

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("檔案", |ui| {
                if ui.button("新增專案").clicked() {
                    ui.close();
                    self.do_action(PendingAction::New, false);
                }
                if ui.button("載入範例專案").clicked() {
                    ui.close();
                    self.do_action(PendingAction::Sample, false);
                }
                if ui.button("開啟…            Ctrl+O").clicked() {
                    ui.close();
                    self.do_action(PendingAction::Open, false);
                }
                ui.separator();
                if ui.button("儲存              Ctrl+S").clicked() {
                    ui.close();
                    self.save(false);
                }
                if ui.button("另存新檔…   Ctrl+Shift+S").clicked() {
                    ui.close();
                    self.save(true);
                }
                ui.separator();
                if ui.button("匯出…（選擇格式：PNG / HTML / Markdown / layout.json …）  Ctrl+E").clicked()
                {
                    ui.close();
                    self.export_dialog();
                }
                if ui.button("只輸出目前場景 PNG…").clicked() {
                    ui.close();
                    self.export_scene_png();
                }
                ui.separator();
                if ui.button("結束").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("編輯", |ui| {
                if ui.add_enabled(self.can_undo(), egui::Button::new("復原    Ctrl+Z")).clicked() {
                    self.undo();
                    ui.close();
                }
                if ui.add_enabled(self.can_redo(), egui::Button::new("重做    Ctrl+Y")).clicked() {
                    self.redo();
                    ui.close();
                }
                ui.separator();
                let sel = self.selected_index().is_some();
                if ui.add_enabled(sel, egui::Button::new("複製元件    Ctrl+D")).clicked() {
                    self.duplicate_selected();
                    ui.close();
                }
                if ui.add_enabled(sel, egui::Button::new("刪除元件    Delete")).clicked() {
                    self.delete_selected();
                    ui.close();
                }
                ui.separator();
                if ui.add_enabled(sel, egui::Button::new("上移一層    ]")).clicked() {
                    self.move_layer(1);
                }
                if ui.add_enabled(sel, egui::Button::new("下移一層    [")).clicked() {
                    self.move_layer(-1);
                }
                if ui.add_enabled(sel, egui::Button::new("移到最上層")).clicked() {
                    self.move_layer(10_000);
                }
                if ui.add_enabled(sel, egui::Button::new("移到最下層")).clicked() {
                    self.move_layer(-10_000);
                }
            });
            ui.menu_button("檢視", |ui| {
                ui.checkbox(&mut self.show_grid, "顯示格線");
                ui.checkbox(&mut self.snap, "吸附（格線 / 安全框 / 其他元件）");
                ui.checkbox(&mut self.show_safe, "顯示 Title-safe / Action-safe");
                ui.checkbox(&mut self.annotations, "顯示類型 / 時間標籤");
            });
            ui.menu_button("說明", |ui| {
                if ui.button("操作說明…").clicked() {
                    self.show_help = true;
                    ui.close();
                }
                if ui.button("關於…").clicked() {
                    self.show_about = true;
                    ui.close();
                }
            });
        });
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.add_enabled(self.can_undo(), egui::Button::new("⟲ 復原")).clicked() {
                self.undo();
            }
            if ui.add_enabled(self.can_redo(), egui::Button::new("⟳ 重做")).clicked() {
                self.redo();
            }
            ui.separator();
            let sel = self.selected_index().is_some();
            if ui.add_enabled(sel, egui::Button::new("🗐 複製")).clicked() {
                self.duplicate_selected();
            }
            if ui.add_enabled(sel, egui::Button::new("🗑 刪除")).clicked() {
                self.delete_selected();
            }
            if ui.add_enabled(sel, egui::Button::new("⬆ 上移")).on_hover_text("上移一層 ( ] )").clicked() {
                self.move_layer(1);
            }
            if ui.add_enabled(sel, egui::Button::new("⬇ 下移")).on_hover_text("下移一層 ( [ )").clicked() {
                self.move_layer(-1);
            }
            ui.separator();
            ui.toggle_value(&mut self.show_grid, "格線");
            ui.toggle_value(&mut self.snap, "吸附");
            ui.toggle_value(&mut self.show_safe, "安全框");
            ui.toggle_value(&mut self.annotations, "標籤");
            ui.separator();
            ui.label("縮放");
            ui.add(egui::Slider::new(&mut self.zoom, 0.25..=3.0).show_value(false));
            if ui.small_button(format!("{:.0}%", self.zoom * 100.0)).on_hover_text("重設為符合視窗").clicked() {
                self.zoom = 1.0;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(RichText::new("⬇ 匯出草稿").strong().color(ACCENT))
                    .on_hover_text("Ctrl+E：選擇格式並匯出到「專案名_日期_時間」資料夾")
                    .clicked()
                {
                    self.export_dialog();
                }
                if ui.button("💾 儲存").clicked() {
                    self.save(false);
                }
            });
        });
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            match &self.status {
                Some((true, m)) => ui.label(RichText::new(m).color(OK_GREEN)),
                Some((false, m)) => ui.label(RichText::new(m).color(ERR_RED)),
                None => ui.label(RichText::new("就緒").weak()),
            };
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let file = self.path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "（未儲存）".into());
                ui.label(
                    RichText::new(format!(
                        "{}{} · {}×{} · {} fps · {} 場景 · 共 {:.1}s",
                        if self.dirty { "● " } else { "" },
                        file,
                        self.project.canvas.width,
                        self.project.canvas.height,
                        self.project.fps,
                        self.project.scenes.len(),
                        self.project.total_duration()
                    ))
                    .small()
                    .weak(),
                );
            });
        });
    }

    fn windows(&mut self, ctx: &egui::Context) {
        let mut open = self.show_help;
        egui::Window::new("操作說明").open(&mut open).resizable(false).collapsible(false).show(ctx, |ui| {
            egui::Grid::new("help").num_columns(2).spacing([16.0, 4.0]).show(ui, |ui| {
                for (k, v) in [
                    ("從左側元件庫拖曳到畫布", "新增元件（點一下則加在畫面中央）"),
                    ("拖曳元件", "移動（Alt 按住暫時不吸附）"),
                    ("拖曳選取框的 8 個控制點", "調整大小（Shift = 等比例）"),
                    ("方向鍵 / Shift+方向鍵", "微調 1px / 10px"),
                    ("Delete", "刪除元件"),
                    ("Ctrl+D", "複製元件"),
                    ("] / [", "上移 / 下移圖層"),
                    ("Ctrl+Z / Ctrl+Y", "復原 / 重做"),
                    ("Ctrl+S / Ctrl+O", "儲存 / 開啟專案"),
                    ("Ctrl+E", "匯出（PNG、HTML、Markdown、layout.json、AGENT_GUIDE.md、render_moviepy.py…）"),
                    ("Esc", "取消選取"),
                ] {
                    ui.label(RichText::new(k).strong());
                    ui.label(v);
                    ui.end_row();
                }
            });
        });
        self.show_help = open;
        let mut open = self.show_about;
        egui::Window::new("關於").open(&mut open).resizable(false).collapsible(false).show(ctx, |ui| {
            ui.heading(format!("Whitebox Video Storyboard {}", env!("CARGO_PKG_VERSION")));
            ui.label("白模影片版面草稿工具：拖曳字幕、標題、選項等元件，匯出草稿圖與 layout.json，讓 AI Agent 用 MoviePy 合成影片。");
            ui.hyperlink("https://github.com/stevenke1981/whitebox-video-storyboard");
            ui.label("© 2026 Ke Sheng Da — MIT License");
            ui.label("內附字型：Noto Sans CJK TC（子集），SIL Open Font License 1.1");
        });
        self.show_about = open;

        if let Some(action) = self.confirm_discard {
            let mut close = false;
            egui::Modal::new(egui::Id::new("confirm_discard")).show(ctx, |ui| {
                ui.heading("尚未儲存的變更");
                ui.label("目前的專案有未儲存的變更，要放棄嗎？");
                ui.horizontal(|ui| {
                    if ui.button("放棄變更並繼續").clicked() {
                        close = true;
                        self.confirm_discard = None;
                        self.do_action(action, true);
                    }
                    if ui.button("取消").clicked() {
                        close = true;
                    }
                });
            });
            if close {
                self.confirm_discard = None;
            }
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.clamp_scene();
        let before = self.project.clone();
        self.shortcuts(&ctx);

        egui::Panel::top("menu_bar").show(ui, |ui| self.menu_bar(ui));
        egui::Panel::top("toolbar").show(ui, |ui| self.toolbar(ui));
        egui::Panel::bottom("status_bar").show(ui, |ui| self.status_bar(ui));
        egui::Panel::bottom("scenes")
            .resizable(true)
            .default_size(150.0)
            .min_size(110.0)
            .show(ui, |ui| self.scenes_panel(ui));
        egui::Panel::left("palette").resizable(true).default_size(250.0).min_size(200.0).show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| self.palette(ui));
        });
        egui::Panel::right("inspector").resizable(true).default_size(330.0).min_size(260.0).show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| self.inspector(ui));
        });
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(Color32::from_rgb(22, 23, 26)).inner_margin(0))
            .show(ui, |ui| self.canvas(ui));
        self.windows(&ctx);
        self.export_window(&ctx);
        self.clamp_scene();
        self.commit_history(&ctx, before);
    }
}
