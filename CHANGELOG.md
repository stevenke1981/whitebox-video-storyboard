# Changelog

## 0.2.0 — 2026-10-04

- **匯出資料夾**：每次匯出自動建立 `<專案檔名或 untitled>_<YYYYMMDD_HHMMSS>` 資料夾（GUI 與 CLI），完成後顯示完整路徑，可「開啟資料夾」/「複製路徑」。CLI 可用 `--no-subdir` 直接寫入、`--name` 指定名稱，stdout 第一行為輸出資料夾。
- **匯出視窗**（Ctrl+E）：勾選 PNG、總覽圖、layout.json、project.json、AGENT_GUIDE、render_moviepy.py、HTML、Markdown、字型；草稿圖選項與輸出位置；設定會記住。
- **新格式**：`storyboard.html`（單一檔案分鏡網頁，內嵌草稿圖、元件表、時間軸、備註、MoviePy 提示）與 `storyboard.md`（繁中純文字版面說明，含 ASCII 版面速寫、九宮格位置、尺寸、時間、動畫、事件順序）。
- CLI `--formats png,overview,layout,project,guide,script,html,md,font|all`。
- **執行檔縮小**：Linux 18.7 MB → 12.0 MB（opt-level "s"、fat LTO、codegen-units 1、panic abort；內嵌字型 Brotli 壓縮；移除 egui 預設字型）。
- AGENT_GUIDE 的檔案表依實際匯出的格式產生。

## 0.1.0 — 2026-10-04

- 第一版：egui 白模版面編輯器、18 種元件、多場景、PNG / layout.json / AGENT_GUIDE / render_moviepy.py 匯出、無頭 CLI。
