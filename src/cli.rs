//! Headless command-line interface (no display needed).

use crate::export::{ExportOptions, export_all, render_scene};
use crate::model::Project;
use std::path::Path;

pub const HELP: &str = "whitebox-video-storyboard — 白模影片版面草稿工具 / white-model video layout drafts

USAGE:
  whitebox-video-storyboard                         啟動 GUI（可附上專案檔路徑）
  whitebox-video-storyboard <project.json>          以 GUI 開啟專案
  whitebox-video-storyboard --export <project.json> <out_dir> [options]
        匯出 scene_XX_*.png、storyboard_overview.png、layout.json、project.json、
        AGENT_GUIDE.md、render_moviepy.py、fonts/（不需要顯示器）
        options: --no-font  不複製內附字型
                 --no-annotations  PNG 不畫類型/時間標籤
                 --safe-guides     PNG 畫出 title-safe / action-safe 參考線
                 --no-overview     不輸出總覽圖
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
            let (src, out) = match (args.get(1), args.get(2)) {
                (Some(a), Some(b)) => (a, b),
                _ => return Err("用法: --export <project.json> <out_dir>".into()),
            };
            let (p, raw) = load(src)?;
            let opts = ExportOptions {
                copy_font: !has("--no-font"),
                annotations: !has("--no-annotations"),
                safe_guides: has("--safe-guides"),
                overview: !has("--no-overview"),
            };
            let rep = export_all(&p, Some(&raw), Path::new(out), &opts)?;
            for f in &rep.files {
                println!("{}", f.display());
            }
            eprintln!("匯出完成：{} 個檔案 → {out}", rep.files.len());
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
