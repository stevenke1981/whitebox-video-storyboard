# Changelog

## 0.3.0 — 2026-10-05

- **場景範本庫 / Scene templates**：14 種內建範本（片頭、片尾 CTA、轉場卡 ×3、章節標題、問答、對比 A vs B、清單重點、訪談＋名牌、倒數、引言、商品展示、結尾名單），依畫布比例自動排版（橫式 / 直式）。GUI：「範本」選單、範本庫視窗（Ctrl+T，縮圖預覽）；CLI：`--templates`、`--from-templates <list|all> <out.json>`、`--add-template <project.json> <tpl> [--at N]`。
- **自訂範本**：把目前場景存成範本（`<設定資料夾>/whitebox-video-storyboard/templates/*.json`），可插入其他專案（自動縮放）或刪除。
- **場景轉場 / Transitions**：`none`、`crossfade`、`fade_black`、`slide_left`、`slide_up`、`wipe`、`zoom`，可設定長度；寫入 layout.json（`scene.transition`、`transition_policy`）、AGENT_GUIDE（新第 6 節）、HTML、Markdown、草稿圖標籤，並在 `render_moviepy.py` 實作（新參數 `--transition-frames`、`--no-transitions`）。
- 新動畫 `scroll_up`（名單向上捲動），MoviePy 腳本已實作。
- **English / 雙語**：GUI 語言切換（繁體中文 / English，記在 settings.json；`--lang` 可暫時覆寫）；範本與元件預設文字、AGENT_GUIDE / storyboard.md / storyboard.html / 草稿圖標籤 / CLI 訊息皆可輸出英文（`--lang en|zh-TW` 或 `WVS_LANG`）；新增完整英文說明 `README.en.md`。
- **字級改為 em 大小**：`font_size` 與 CSS / Pillow / MoviePy 意義一致，GUI、草稿 PNG 與 MoviePy 成品文字大小相同（之前 PNG 約小 31%）。舊版（v1）專案檔開啟時自動換算 ×0.69，專案格式版本升為 2。
- layout.json 新增 `language`、`transition_policy` 與每個場景的 `transition`。
- **Release**：下載檔改為 AES-256 加密 ZIP（四個平台；密碼存在 GitHub Actions secret，請向作者索取）；Release 只由最後一個 job 建立一次（不再重複 release notes），附 `SHA256SUMS.txt`；Actions 升級到 Node 24 版本（checkout v7、upload-artifact v7、download-artifact v8、setup-python v7、action-gh-release v3）。
- CI 新增範本 / 英文匯出 smoke test 與 14 範本轉場的 MoviePy 測試。

## 0.2.0 — 2026-10-04

- **匯出資料夾**：每次匯出自動建立 `<專案檔名或 untitled>_<YYYYMMDD_HHMMSS>` 資料夾（GUI 與 CLI），完成後顯示完整路徑，可「開啟資料夾」/「複製路徑」。CLI 可用 `--no-subdir` 直接寫入、`--name` 指定名稱，stdout 第一行為輸出資料夾。
- **匯出視窗**（Ctrl+E）：勾選 PNG、總覽圖、layout.json、project.json、AGENT_GUIDE、render_moviepy.py、HTML、Markdown、字型；草稿圖選項與輸出位置；設定會記住。
- **新格式**：`storyboard.html`（單一檔案分鏡網頁，內嵌草稿圖、元件表、時間軸、備註、MoviePy 提示）與 `storyboard.md`（繁中純文字版面說明，含 ASCII 版面速寫、九宮格位置、尺寸、時間、動畫、事件順序）。
- CLI `--formats png,overview,layout,project,guide,script,html,md,font|all`。
- **執行檔縮小**：Linux 18.7 MB → 12.0 MB（opt-level "s"、fat LTO、codegen-units 1、panic abort；內嵌字型 Brotli 壓縮；移除 egui 預設字型）。
- AGENT_GUIDE 的檔案表依實際匯出的格式產生。

## 0.1.0 — 2026-10-04

- 第一版：egui 白模版面編輯器、18 種元件、多場景、PNG / layout.json / AGENT_GUIDE / render_moviepy.py 匯出、無頭 CLI。
