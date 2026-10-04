//! Headless command-line interface (no display needed).

use crate::export::{ExportOptions, Formats, export_to, project_dir_name, render_scene};
use crate::model::Project;
use std::path::Path;

pub const HELP: &str = "whitebox-video-storyboard — 白模影片版面草稿工具 / white-model video layout drafts

USAGE:
  whitebox-video-storyboard                         啟動 GUI（可附上專案檔路徑）
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
  whitebox-video-storyboard --sample <out.json> [--preset 16:9|9:16|1:1|4:5|4:3]
        寫出內建範例專案
  whitebox-video-storyboard --help | --version
";

fn load(path: &str) -> Result<(Project, String), String> {
    let s = std::fs::read_to_string(path).map_err(|e| format!("讀取 {path} 失敗: {e}"))?;
    Ok((Project::from_json(&s)?, s))
}

/// Returns `Some(exit_code)` when a CLI command was handled, `None` to start the GUI.
pub fn run(args: &[String]) -> Option<i32> {
    let first = args.first()?.as_str();
    let has = |f: &str| args.iter().any(|a| a == f);
    let res: Result<(), String> = match first {
        "-h" | "--help" => {
            println!("{HELP}");
            Ok(())
        }
        "-V" | "--version" => {
            println!("whitebox-video-storyboard {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        "--export" => (|| {
            let src = args
                .get(1)
                .filter(|a| !a.starts_with("--"))
                .ok_or("用法: --export <project.json> [out_base] [--formats ...]")?;
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
            let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
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
            eprintln!("匯出完成：{} 個檔案（{}）→ {}", rep.files.len(), opts.formats.list(), rep.dir.display());
            Ok(())
        })(),
        "--render" => (|| {
            let (src, n, out) = match (args.get(1), args.get(2).and_then(|s| s.parse::<usize>().ok()), args.get(3)) {
                (Some(a), Some(n), Some(o)) if n >= 1 => (a, n, o),
                _ => return Err("用法: --render <project.json> <scene_no> <out.png>".into()),
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
        "--sample" => (|| {
            let out = args.get(1).ok_or("用法: --sample <out.json> [--preset 9:16]")?;
            let preset = args
                .iter()
                .position(|a| a == "--preset")
                .and_then(|i| args.get(i + 1))
                .map(String::as_str)
                .unwrap_or("16:9");
            let p = crate::sample::sample_project(preset);
            std::fs::write(out, p.to_json()).map_err(|e| e.to_string())?;
            println!("{out}");
            Ok(())
        })(),
        s if s.starts_with('-') => Err(format!("未知參數 {s}\n\n{HELP}")),
        _ => return None, // a project path for the GUI
    };
    Some(match res {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("錯誤: {e}");
            1
        }
    })
}
