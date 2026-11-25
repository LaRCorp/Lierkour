<div align="center">
  <img src="src-tauri/icons/logo.png" width="96" height="96" alt="Lierkour Logo" />
  <h1>🎮🎬 Lierkour v2.3 — Keeper of Completed Media</h1>
  <p><strong>Local cross-platform catalog for movies, series, anime, and video games powered by Rust and Tauri 2</strong></p>

  <p>
    <a href="README.md"><img src="https://img.shields.io/badge/Язык-Русский-8B0000?style=flat-square" alt="Русский" /></a>
    <a href="README_EN.md"><img src="https://img.shields.io/badge/Language-English-39C5BB?style=flat-square" alt="English" /></a>
    <a href="https://github.com/LaRCorp/Lierkour/releases"><img src="https://img.shields.io/badge/version-2.3.0-6366f1?style=flat-square" alt="Version 2.3.0" /></a>
    <img src="https://img.shields.io/badge/Platform-Windows%20%7C%20Linux-0ea5e9?style=flat-square" alt="Windows | Linux" />
    <img src="https://img.shields.io/badge/Tauri%202-React%2019%20%7C%20Rust-8b5cf6?style=flat-square" alt="Tauri 2 + React 19 + Rust" />
    <a href="LICENSE"><img src="https://img.shields.io/badge/License-GPLv3-10b981?style=flat-square" alt="GPLv3 License" /></a>
    <a href="https://github.com/LaRCorp/LierEX-Q"><img src="https://img.shields.io/badge/Companion-LierEX--Q_v1.4%2B-f59e0b?style=flat-square" alt="LierEX-Q Extension" /></a>
    <a href="https://github.com/LaRCorp/LierTags"><img src="https://img.shields.io/badge/Companion-LierTags_v1.1%2B-ec4899?style=flat-square" alt="LierTags Companion" /></a>
    <a href="https://github.com/LaRCorp/LierSync"><img src="https://img.shields.io/badge/Companion-LierSync_v1.2%2B-39C5BB?style=flat-square" alt="LierSync Companion" /></a>
  </p>
</div>

**Lierkour** is a fast local desktop application built with **Tauri 2**, **React 19**, and **SQLite** for tracking your personal collection of movies, TV shows, anime, and video games — with full offline portability and zero cloud lock-in.

---

## Key Features

- **Movies, Series & Anime:** Hierarchical season cards, watch statuses, ratings (`1–10`), favorite quotes, screenshot galleries, and OMDb metadata lookup.
- **Video Games:** Platform & launcher tracking (Steam, GOG, Epic Games, Itch.io, Switch, PS5, etc.), playtime tracking, and game screenshot galleries.
- **Directed Title Relations:** Sequels, prequels, remakes, spin-offs, and franchise links.
- **Backups & Portability:** Full self-contained ZIP archives (`lierkour-backup.zip`), CSV & Markdown exports, and portable `run-portable.bat` / `run-portable.sh` launchers.
- **LIER Ecosystem Integration (v2.3):**
  - **Conflict-free Port Map:** Browser extension bridge runs on `127.0.0.1:8765`, Mobile Delta Sync runs on `0.0.0.0:8767` (freeing `:8766` for `LierTags`).
  - **LierSync (`:8768`):** One-click `lierkour-backup.zip` transfer to phone/PC plus sending posters or quotes straight to the shared clipboard.
  - **LierTags (`:8766`):** Send title metadata & artwork directly to soundtrack (OST) tagging or let LierTags search your Lierkour catalog.
  - **LierEX-Q (`:8765`):** 1-click import from Kinopoisk, IMDb, TMDb, Letterboxd, Steam, Epic Games, GOG, and Itch.io.

---

## Build from Source (Windows / Linux)

```bash
git clone https://github.com/LaRCorp/Lierkour.git
cd Lierkour

cd frontend
npm ci
npm run build
cd ..

cargo test --workspace
cargo build --release --features custom-protocol
```

---

## License

Released under **GNU General Public License v3.0 (GPLv3)** — see [LICENSE](LICENSE).

Copyright (c) 2026 LaRevole.

### Related Ecosystem Projects
- ⚡🔄 **[LaRCorp/LierSync](https://github.com/LaRCorp/LierSync)** — fast P2P file transfers, folder sync, and shared clipboard for PC & Android.
- 🌐 **[LaRCorp/LierEX-Q](https://github.com/LaRCorp/LierEX-Q)** — browser extension for extracting media cards, music tags, and pushing links/images to LierSync.
- 🎵🏷️ **[LaRCorp/LierTags](https://github.com/LaRCorp/LierTags)** — high-performance local audio tag & cover art editor built with Rust.
