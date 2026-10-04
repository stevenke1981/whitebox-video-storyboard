//! Headless command-line interface (no display needed).

use crate::export::{ExportOptions, Formats, export_to, project_dir_name, render_scene};
use crate::i18n::{Lang, is_en, set_lang, t};
use crate::model::Project;
use crate::templates::{Template, build, project_from_templates};
use std::path::Path;

pub const HELP_ZH: &str = "whitebox-video-storyboard — 白模影片版面草稿工具

用法：
  whitebox-video-storyboard [--lang en|zh-TW]       啟動 GUI（可附上專案檔路徑）
  whitebox-video-storyboard <project.json>          以 GUI 開啟專案
  whitebox-video-storyboard --export <project.json> [out_base] [options]
        在 out_base（預設＝目前資料夾）裡建立 <專案檔名>_<YYYYMMDD_HHMMSS>/ 資料夾並匯出（不需要顯示器）。
        stdout 第一行是建立的資料夾路徑，其後是每個檔案。
        options: --formats <list>   要輸出的格式，逗號分隔或 all（預設 all）：
                                    png,overview,layout,project,guide,script,html,md,font
                 --no-subdir        直接寫入 out_base，不建立時間戳記子資料夾
                 --name <name>      子資料夾名稱前綴（預設＝專案檔名）
                 --no-font / --no-overview   從格式中移除字型 / 總覽圖
                 --no-annotations   PNG 不畫類型/時間標籤
                 --safe-guides      PNG 畫出 title-safe / action-safe 參考線
  whitebox-video-storyboard --render <project.json> <scene_no> <out.png>
        只輸出單一場景（1 起算）
  whitebox-video-storyboard --templates
        列出內建場景範本
  whitebox-video-storyboard --from-templates <list|all> <out.json> [--preset 16:9|9:16|1:1|4:5|4:3] [--name 專案名]
        用範本建立新專案，例如 intro,quiz,outro
  whitebox-video-storyboard --add-template <project.json> <template> [--at N] [-o out.json]
        在專案第 N 個場景後（預設最後）插入範本場景；未給 -o 時直接覆寫專案檔
  whitebox-video-storyboard --sample <out.json> [--preset 16:9|9:16|1:1|4:5|4:3]
        寫出內建範例專案
  whitebox-video-storyboard --help | --version

全域選項：
  --lang en|zh-TW   介面、範本預設文字與匯出文件（AGENT_GUIDE、Markdown、HTML）的語言；
                    預設讀環境變數 WVS_LANG，否則繁體中文
";

pub const HELP_EN: &str = "whitebox-video-storyboard — white-model video layout drafts

USAGE:
  whitebox-video-storyboard [--lang en|zh-TW]       start the GUI (optionally with a project path)
  whitebox-video-storyboard <project.json>          open a project in the GUI
  whitebox-video-storyboard --export <project.json> [out_base] [options]
        create <project name>_<YYYYMMDD_HHMMSS>/ inside out_base (default: current folder) and export
        into it (no display needed). The first stdout line is the created folder, then one line per file.
        options: --formats <list>   comma-separated formats or all (default all):
                                    png,overview,layout,project,guide,script,html,md,font
                 --no-subdir        write straight into out_base (no timestamped sub-folder)
                 --name <name>      sub-folder name prefix (default: project file name)
                 --no-font / --no-overview   drop the font / overview from the formats
                 --no-annotations   no type / timing labels on the PNGs
                 --safe-guides      draw title-safe / action-safe guides on the PNGs
  whitebox-video-storyboard --render <project.json> <scene_no> <out.png>
        render a single scene (1-based)
  whitebox-video-storyboard --templates
        list the built-in scene templates
  whitebox-video-storyboard --from-templates <list|all> <out.json> [--preset 16:9|9:16|1:1|4:5|4:3] [--name NAME]
        create a new project from templates, e.g. intro,quiz,outro
  whitebox-video-storyboard --add-template <project.json> <template> [--at N] [-o out.json]
        insert a template scene after scene N (default: at the end); overwrites the project unless -o is given
  whitebox-video-storyboard --sample <out.json> [--preset 16:9|9:16|1:1|4:5|4:3]
        write the built-in sample project
  whitebox-video-storyboard --help | --version

GLOBAL OPTIONS:
  --lang en|zh-TW   language of template texts and exported docs (AGENT_GUIDE, Markdown, HTML);
                    defaults to the WVS_LANG environment variable, else Traditional Chinese
";

pub fn help() -> &'static str {
    t(HELP_ZH, HELP_EN)
}

/// Remove `--lang X` from `args`, falling back to `WVS_LANG`. Returns the explicit
/// language (if any) and sets it as the current language.
pub fn take_lang(args: &mut Vec<String>) -> Result<Option<Lang>, String> {
    let mut found = None;
    if let Some(i) = args.iter().position(|a| a == "--lang") {
        let v = args.get(i + 1).cloned().ok_or("--lang needs a value: en | zh-TW")?;
        found = Some(Lang::parse(&v)?);
        args.drain(i..i + 2);
    } else if let Some(i) = args.iter().position(|a| a.starts_with("--lang=")) {
        found = Some(Lang::parse(&args[i]["--lang=".len()..])?);
        args.remove(i);
    } else if let Ok(v) = std::env::var("WVS_LANG")
        && !v.trim().is_empty()
    {
        found = Lang::parse(&v).ok();
    }
    if let Some(l) = found {
        set_lang(l);
    }
    Ok(found)
}

fn load(path: &str) -> Result<(Project, String), String> {
    let s = std::fs::read_to_string(path).map_err(|e| tf!("讀取 {path} 失敗: {e}", "cannot read {path}: {e}"))?;
    Ok((Project::from_json(&s)?, s))
}

fn parse_templates(list: &str) -> Result<Vec<Template>, String> {
    if list.trim() == "all" {
        return Ok(Template::ALL.to_vec());
    }
    list.split(',')
        .filter(|s| !s.trim().is_empty())
        .map(|k| {
            Template::parse(k).ok_or_else(|| {
                let keys: Vec<_> = Template::ALL.iter().map(|t| t.key()).collect();
                tf!("未知範本 {k:?}（可用：{}）", "unknown template {k:?} (available: {})", keys.join(","))
            })
        })
        .collect()
}

/// Returns `Some(exit_code)` when a CLI command was handled, `None` to start the GUI.
pub fn run(args: &[String]) -> Option<i32> {
    let mut args = args.to_vec();
    if let Err(e) = take_lang(&mut args) {
        eprintln!("error: {e}");
        return Some(2);
    }
    let args = &args;
    let first = args.first()?.as_str();
    let has = |f: &str| args.iter().any(|a| a == f);
    let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
    let res: Result<(), String> = match first {
        "-h" | "--help" => {
            println!("{}", help());
            Ok(())
        }
        "-V" | "--version" => {
            println!("whitebox-video-storyboard {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        "--export" => (|| {
            let src = args.get(1).filter(|a| !a.starts_with("--")).ok_or(t(
                "用法: --export <project.json> [out_base] [--formats ...]",
                "usage: --export <project.json> [out_base] [--formats ...]",
            ))?;
            // optional positional out_base (anything after src that is not a flag or a flag value)
            let takes_value = ["--formats", "--name"];
            let mut base = None;
            let mut i = 2;
            while i < args.len() {
                if takes_value.contains(&args[i].as_str()) {
                    i += 2;
                    continue;
                }
                if !args[i].starts_with("--") && base.is_none() {
                    base = Some(args[i].clone());
                }
                i += 1;
            }
            let (p, raw) = load(src)?;
            let mut formats = match value("--formats") {
                Some(list) => Formats::parse(&list)?,
                None => Formats::default(),
            };
            if has("--no-font") {
                formats.font = false;
            }
            if has("--no-overview") {
                formats.overview = false;
            }
            let opts =
                ExportOptions { formats, annotations: !has("--no-annotations"), safe_guides: has("--safe-guides") };
            let base = base.unwrap_or_else(|| ".".into());
            let name = value("--name").unwrap_or_else(|| project_dir_name(Some(Path::new(src))));
            let rep = export_to(&p, Some(&raw), Path::new(&base), &name, !has("--no-subdir"), &opts)?;
            println!("{}", rep.dir.display());
            for f in &rep.files {
                println!("{}", f.display());
            }
            eprintln!(
                "{}",
                tf!(
                    "匯出完成：{} 個檔案（{}）→ {}",
                    "Exported {} files ({}) → {}",
                    rep.files.len(),
                    opts.formats.list(),
                    rep.dir.display()
                )
            );
            Ok(())
        })(),
        "--render" => (|| {
            let (src, n, out) = match (args.get(1), args.get(2).and_then(|s| s.parse::<usize>().ok()), args.get(3)) {
                (Some(a), Some(n), Some(o)) if n >= 1 => (a, n, o),
                _ => {
                    return Err(t(
                        "用法: --render <project.json> <scene_no> <out.png>",
                        "usage: --render <project.json> <scene_no> <out.png>",
                    )
                    .into());
                }
            };
            let (p, _) = load(src)?;
            let opts = ExportOptions {
                safe_guides: has("--safe-guides"),
                annotations: !has("--no-annotations"),
                ..Default::default()
            };
            let pm = render_scene(&p, n - 1, &opts)?;
            crate::raster::save_png(&pm, Path::new(out))?;
            println!("{out}");
            Ok(())
        })(),
        "--templates" => {
            for tp in Template::ALL {
                println!("{:<16} {}  —  {}", tp.key(), tp.label(), tp.description());
            }
            Ok(())
        }
        "--from-templates" => (|| {
            let (list, out) = match (args.get(1), args.get(2)) {
                (Some(l), Some(o)) if !o.starts_with("--") => (l, o),
                _ => {
                    return Err(t(
                        "用法: --from-templates <list|all> <out.json> [--preset 9:16] [--name 名稱]",
                        "usage: --from-templates <list|all> <out.json> [--preset 9:16] [--name NAME]",
                    )
                    .into());
                }
            };
            let tpls = parse_templates(list)?;
            let preset = value("--preset").unwrap_or_else(|| "16:9".into());
            if crate::model::preset(&preset).is_none() {
                return Err(tf!("未知比例 {preset}", "unknown preset {preset}"));
            }
            let name = value("--name").unwrap_or_else(|| t("範本專案", "Template project").into());
            let p = project_from_templates(&tpls, &preset, &name);
            std::fs::write(out, p.to_json()).map_err(|e| e.to_string())?;
            println!("{out}");
            eprintln!("{}", tf!("已建立 {} 個場景（{}）", "Created {} scenes ({})", p.scenes.len(), preset));
            Ok(())
        })(),
        "--add-template" => (|| {
            let (src, key) = match (args.get(1), args.get(2)) {
                (Some(s), Some(k)) if !k.starts_with("--") => (s, k),
                _ => {
                    return Err(t(
                        "用法: --add-template <project.json> <template> [--at N] [-o out.json]",
                        "usage: --add-template <project.json> <template> [--at N] [-o out.json]",
                    )
                    .into());
                }
            };
            let (mut p, _) = load(src)?;
            let tpls = parse_templates(key)?;
            let start = match value("--at") {
                Some(n) => n.parse::<usize>().map_err(|_| "--at N")?.min(p.scenes.len()),
                None => p.scenes.len(),
            };
            for (k, tp) in tpls.into_iter().enumerate() {
                let s = build(tp, &p);
                p.scenes.insert(start + k, s);
            }
            p.normalize();
            let out = value("-o").unwrap_or_else(|| src.clone());
            std::fs::write(&out, p.to_json()).map_err(|e| e.to_string())?;
            println!("{out}");
            Ok(())
        })(),
        "--sample" => (|| {
            let out = args
                .get(1)
                .ok_or(t("用法: --sample <out.json> [--preset 9:16]", "usage: --sample <out.json> [--preset 9:16]"))?;
            let preset = value("--preset").unwrap_or_else(|| "16:9".into());
            let p = crate::sample::sample_project(&preset);
            std::fs::write(out, p.to_json()).map_err(|e| e.to_string())?;
            println!("{out}");
            Ok(())
        })(),
        s if s.starts_with('-') => Err(tf!("未知參數 {s}\n\n{}", "unknown option {s}\n\n{}", help())),
        _ => return None, // a project path for the GUI
    };
    Some(match res {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{}{e}", if is_en() { "error: " } else { "錯誤: " });
            1
        }
    })
}
