// Hide the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

use eframe::egui;

/// Release builds use the GUI subsystem on Windows; re-attach to the parent console so
/// the command-line interface can print when started from a terminal.
#[cfg(windows)]
fn attach_parent_console() {
    unsafe extern "system" {
        fn AttachConsole(process_id: u32) -> i32;
    }
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    // SAFETY: plain Win32 call without pointers; failure (no parent console) is harmless.
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

fn icon() -> egui::IconData {
    // 64×64 white-model icon: light card with grey boxes.
    let n = 64usize;
    let mut rgba = vec![0u8; n * n * 4];
    for y in 0..n {
        for x in 0..n {
            let i = (y * n + x) * 4;
            let inside = (4..60).contains(&x) && (10..54).contains(&y);
            let title = (12..52).contains(&x) && (18..26).contains(&y);
            let body = (12..36).contains(&x) && (32..46).contains(&y);
            let btn = (40..52).contains(&x) && (38..46).contains(&y);
            let c: [u8; 4] = if title || body {
                [150, 150, 150, 255]
            } else if btn {
                [240, 128, 20, 255]
            } else if inside {
                [236, 236, 236, 255]
            } else {
                [0, 0, 0, 0]
            };
            rgba[i..i + 4].copy_from_slice(&c);
        }
    }
    egui::IconData { rgba, width: 64, height: 64 }
}

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    #[cfg(windows)]
    if !args.is_empty() {
        attach_parent_console();
    }
    if let Some(code) = wvs::cli::run(&args) {
        std::process::exit(code);
    }
    let open = args.first().cloned();
    let viewport = egui::ViewportBuilder::default()
        .with_title("白模影片版面草稿 Whitebox Video Storyboard")
        .with_inner_size([1480.0, 900.0])
        .with_min_inner_size([1000.0, 640.0])
        .with_app_id("whitebox-video-storyboard")
        .with_icon(icon());
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native("whitebox-video-storyboard", options, Box::new(move |cc| Ok(Box::new(app::App::new(cc, open)))))
}
