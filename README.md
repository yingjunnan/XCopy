<div align="center">

<img src="src-tauri/icons/128x128.png" width="120" height="120" alt="XCopy" />

# XCopy

**剪贴板,从未如此顺手。**

一款轻量的 Windows 剪贴板历史工具:自动记录文本、链接与图片,按快捷键即时唤起,秒搜分类、图片预览、本地存储。

[![Build and Release](https://github.com/yingjunnan/XCopy/actions/workflows/build-and-release.yml/badge.svg)](https://github.com/yingjunnan/XCopy/actions)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Windows](https://img.shields.io/badge/platform-Windows%2010%2F11-0078D4.svg)](#下载安装)

[下载安装](#下载安装) · [功能特性](#功能特性) · [技术栈](#技术栈) · [本地开发](#本地开发) · [项目结构](#项目结构)

English | [中文](README.md)

</div>

---

## 功能特性

- **🖥️ 全局快捷键唤起** — 默认 `Ctrl+Shift+V` 即时弹出,选完即走。失焦自动隐藏,不打断你的工作流。
- **📋 自动记录一切** — 文本、链接、图片统统自动捕获,后台静默运行并按内容哈希去重,不重复不啰嗦。
- **🔍 秒搜 + 分类** — 全部 / 文本 / 链接 / 图片 四类标签,关键词即时过滤,百条记录也能一眼找到。
- **🖼️ 图片预览** — 独立窗口查看大图,支持鼠标滚轮缩放与拖拽平移,截图细节看得清清楚楚。
- **🗄️ 轻量本地存储** — 基于 SQLite,可自定义保留条数(最多 10 万条)与保留天数,超出自动清理。
- **⚙️ 随手可配** — 开机自启、系统托盘常驻、全局快捷键可自定义录制,所有偏好集中一处。
- **📊 存储一目了然** — 设置面板实时显示数据库与图片各自占用的磁盘空间。

## 下载安装

前往 [Releases](https://github.com/yingjunnan/XCopy/releases) 下载最新的 `XCopy_*-setup.exe` 安装包,双击安装即可。

- 系统要求:Windows 10 / 11
- 安装后默认开机自启,按 `Ctrl+Shift+V` 唤起

## 使用方式

1. 安装后,应用常驻系统托盘,后台自动记录剪贴板。
2. 复制任意文本、链接或截图,记录会自动保存。
3. 按 `Ctrl+Shift+V`(可在设置中修改)唤起主窗口。
4. 点击任意条目即可重新复制;图片条目可点击"查看大图"。
5. 通过顶部搜索框或分类标签快速筛选。

> 首次启动安装后,主窗口会自动弹出一次,提示应用已就绪。

## 技术栈

| 层 | 技术 |
|----|------|
| 桌面框架 | [Tauri 2](https://v2.tauri.app/) |
| 前端 | React 18 + TypeScript + Vite |
| 样式 | Tailwind CSS + Framer Motion |
| 后端 | Rust |
| 存储 | SQLite (rusqlite) |
| 剪贴板 | arboard + Win32 API |
| 打包 | NSIS |

## 本地开发

### 环境要求

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/tools/install) (stable)
- [Tauri 2 前置依赖](https://v2.tauri.app/start/prerequisites/)(Windows 需要 WebView2 与 MSVC 构建工具)

### 运行

```bash
# 安装前端依赖
npm install

# 启动开发模式(同时拉起 Vite 与 Tauri 窗口)
npm run tauri dev
```

### 构建

```bash
# 构建生产安装包(产物位于 src-tauri/target/release/bundle/)
npm run tauri build
```

### 常用脚本

| 命令 | 说明 |
|------|------|
| `npm run dev` | 仅启动前端 Vite 开发服务器 |
| `npm run build` | 构建前端到 `dist/` |
| `npm run tauri dev` | 启动 Tauri 开发模式 |
| `npm run tauri build` | 构建生产安装包 |

## 项目结构

```
xcopy/
├── src/                      # 前端源码 (React + TypeScript)
│   ├── App.tsx               # 应用根组件(历史/设置视图切换)
│   ├── preview.tsx           # 图片预览窗口
│   ├── components/           # UI 组件
│   │   ├── ClipboardPanel.tsx
│   │   ├── ClipboardItem.tsx
│   │   ├── SettingsPanel.tsx
│   │   ├── SearchBar.tsx
│   │   ├── CategoryTabs.tsx
│   │   └── EmptyState.tsx
│   ├── hooks/                # React hooks
│   └── types/                # 类型定义
├── src-tauri/                # Rust 后端源码
│   └── src/
│       ├── lib.rs            # 应用入口、命令注册、托盘、快捷键
│       ├── clipboard.rs      # 剪贴板监控与捕获
│       ├── db.rs             # SQLite 数据库操作
│       ├── app_settings.rs   # 设置读写与自启注册
│       ├── models.rs         # 数据模型
│       └── window_tracker.rs # 当前窗口追踪 (Win32)
├── .github/workflows/        # CI: 自动构建并发布 release
└── landing.html              # 项目宣传页(单文件,双击可开)
```

## 持续集成

每次推送到 `main` 分支,GitHub Actions 会自动在 `windows-latest` 上构建 NSIS 安装包并发布一个新的 Release:

- 工作流:`.github/workflows/build-and-release.yml`
- 构建状态:[![Build and Release](https://github.com/yingjunnan/XCopy/actions/workflows/build-and-release.yml/badge.svg)](https://github.com/yingjunnan/XCopy/actions)
- 发布产物:每次推送产生独立 release,tag 形如 `v0.1.0-r<构建序号>`

## 路线图

- [ ] 剪贴板条目置顶 / 收藏
- [ ] 跨设备同步(可选)
- [ ] 富文本 / 代码片段支持
- [ ] 暗色模式

## 贡献

欢迎提交 Issue 与 Pull Request。请确保本地 `npm run tauri dev` 可正常运行,且改动通过 `npm run build` 类型检查。

## 许可证

[MIT](LICENSE)

<div align="center">

Made with ❤️ using Tauri & React

</div>
