<div align="center">

<img src="src-tauri/icons/128x128.png" width="120" height="120" alt="XCopy" />

# XCopy

**Your clipboard, finally effortless.**

A lightweight Windows clipboard history tool: automatically records text, links, and images; summon it with a hotkey; search, filter, preview images, and store everything locally.

[![Build and Release](https://github.com/yingjunnan/XCopy/actions/workflows/build-and-release.yml/badge.svg)](https://github.com/yingjunnan/XCopy/actions)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Windows](https://img.shields.io/badge/platform-Windows%2010%2F11-0078D4.svg)](#download--install)

[Download](#download--install) · [Features](#features) · [Tech Stack](#tech-stack) · [Development](#development) · [Project Structure](#project-structure)

[中文](README.md) | English

</div>

---

## Features

- **🖥️ Global Hotkey Summon** — Pop it up with `Ctrl+Shift+V` (default), grab what you need, move on. Auto-hides on blur so it never interrupts your flow.
- **📋 Auto-Record Everything** — Text, links, and images are captured silently in the background and de-duplicated by content hash. No repeats, no clutter.
- **🔍 Instant Search & Categories** — All / Text / Link / Image tabs with live keyword filtering. Find any entry at a glance, even with hundreds of records.
- **🖼️ Image Preview** — View screenshots in a dedicated window with mouse-wheel zoom and drag-to-pan. Every detail, clearly visible.
- **🗄️ Lightweight Local Storage** — Built on SQLite with configurable retention (up to 100k entries and custom day limits); old entries auto-pruned.
- **⚙️ Configure As You Like** — Auto-start on boot, system-tray resident, and a fully customizable hotkey recorder — all preferences in one place.
- **📊 Storage At A Glance** — The settings panel shows real-time disk usage split between the database and stored images.

## Download & Install

Grab the latest `XCopy_*-setup.exe` from [Releases](https://github.com/yingjunnan/XCopy/releases) and run the installer.

- System requirements: Windows 10 / 11
- After install, it auto-starts on boot; press `Ctrl+Shift+V` to summon

## Usage

1. After installation, the app lives in the system tray and records your clipboard in the background.
2. Copy any text, link, or screenshot — entries are saved automatically.
3. Press `Ctrl+Shift+V` (changeable in Settings) to open the main window.
4. Click any entry to re-copy it; image entries offer a "view large" button.
5. Filter instantly via the search box or category tabs at the top.

> On the very first launch after install, the main window pops up once to signal the app is ready.

## Tech Stack

| Layer | Technology |
|-------|------------|
| Desktop framework | [Tauri 2](https://v2.tauri.app/) |
| Frontend | React 18 + TypeScript + Vite |
| Styling | Tailwind CSS + Framer Motion |
| Backend | Rust |
| Storage | SQLite (rusqlite) |
| Clipboard | arboard + Win32 API |
| Packaging | NSIS |

## Development

### Prerequisites

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/tools/install) (stable)
- [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) (Windows requires WebView2 and MSVC build tools)

### Run

```bash
# Install frontend dependencies
npm install

# Start dev mode (launches both Vite and the Tauri window)
npm run tauri dev
```

### Build

```bash
# Build the production installer (output in src-tauri/target/release/bundle/)
npm run tauri build
```

### Common Scripts

| Command | Description |
|---------|-------------|
| `npm run dev` | Start the Vite frontend dev server only |
| `npm run build` | Build the frontend into `dist/` |
| `npm run tauri dev` | Start Tauri in dev mode |
| `npm run tauri build` | Build the production installer |

## Project Structure

```
xcopy/
├── src/                      # Frontend source (React + TypeScript)
│   ├── App.tsx               # Root component (history/settings switching)
│   ├── preview.tsx           # Image preview window
│   ├── components/           # UI components
│   │   ├── ClipboardPanel.tsx
│   │   ├── ClipboardItem.tsx
│   │   ├── SettingsPanel.tsx
│   │   ├── SearchBar.tsx
│   │   ├── CategoryTabs.tsx
│   │   └── EmptyState.tsx
│   ├── hooks/                # React hooks
│   └── types/                # Type definitions
├── src-tauri/                # Rust backend source
│   └── src/
│       ├── lib.rs            # App entry, command registration, tray, hotkeys
│       ├── clipboard.rs      # Clipboard monitoring & capture
│       ├── db.rs             # SQLite database operations
│       ├── app_settings.rs   # Settings persistence & auto-start
│       ├── models.rs         # Data models
│       └── window_tracker.rs # Active window tracking (Win32)
├── .github/workflows/        # CI: auto build & publish releases
└── landing.html              # Project landing page (single file, open directly)
```

## Continuous Integration

On every push to the `main` branch, GitHub Actions automatically builds the NSIS installer on `windows-latest` and publishes a new Release:

- Workflow: `.github/workflows/build-and-release.yml`
- Build status: [![Build and Release](https://github.com/yingjunnan/XCopy/actions/workflows/build-and-release.yml/badge.svg)](https://github.com/yingjunnan/XCopy/actions)
- Releases: each push produces an independent release, tagged `v0.1.0-r<build-number>`

## Roadmap

- [ ] Pin / favorite clipboard entries
- [ ] Optional cross-device sync
- [ ] Rich text / code snippet support
- [ ] Dark mode

## Contributing

Issues and Pull Requests are welcome. Please ensure `npm run tauri dev` runs locally and your changes pass the `npm run build` type check.

## License

[MIT](LICENSE)

<div align="center">

Made with ❤️ using Tauri & React

</div>
