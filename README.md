# tt-audio-lab

桌面音频可视化组件（Windows），基于 **Tauri 2 + Vue 3 + TypeScript + Rust**。  
A desktop audio visualizer widget for Windows, built with **Tauri 2 + Vue 3 + TypeScript + Rust**.

---

## 中文说明

### 项目简介
`tt-audio-lab` 是一个面向桌面常驻场景的音频频谱组件，支持多种可视化风格、实时参数调节、窗口模式切换与托盘控制，目标是兼顾视觉效果与高刷新率流畅度。

### 当前主要功能
- 多风格频谱：`bars / wave / radial / mirror / spiral / matrix / particles / waterfall / radar`
- 柱状图渲染管线：`WebGL2 -> Canvas2D -> DOM` 自动降级
- 非柱状图渲染：统一 `Canvas2D` 组件路径（性能优化）
- 风格参数调节：速度 / 密度 / 发光 / 拖尾（按风格）
- 设置持久化：本地保存并在启动后恢复
- release 可切换频谱风格（设置弹窗内）
- 沉浸模式：支持“沉浸全屏 / 退出沉浸”
- 托盘控制：显示/隐藏、暂停/恢复、打开设置、退出
- 窗口模式：普通窗口 / 桌面组件 / 悬浮覆盖层

### 运行环境
- Node.js 20+
- Rust toolchain（`cargo` 可用）
- Windows Visual Studio C++ Build Tools（Tauri 桌面构建所需）

### 快速开始
1. 安装依赖
```powershell
npm install
```

2. 前端开发模式（仅 Vite）
```powershell
npm run dev
```

3. Tauri 桌面开发模式
```powershell
npm run tauri dev
```

4. 构建前端静态资源
```powershell
npm run build
```

### 常用脚本
- `npm run dev`：启动 Vite 开发服务
- `npm run tauri dev`：启动 Tauri 桌面开发
- `npm run build`：构建前端资源
- `npm run lint:comment`：中文注释规范检查

### 文档入口
- 产品文档：`docs/PRD.md`
- 架构文档：`docs/ARCH.md`
- 代码规范：`docs/CODING_STANDARD.md`
- 会话上下文：`docs/SESSION_SUMMARY_2026-02-15.md`

### 目录结构（前端核心）
```text
src/app/
  App.vue
  components/
    SettingsModal.vue
    SpectrumCanvas.vue
  visualization/
    frame-store.ts
    tuning.ts
    types.ts
  locales/
    zh-CN.ts
  stores/
    settings.ts
  types.ts
```

### Windows 常见问题
- `cargo metadata ... program not found`
  - 原因：Rust 未安装或 PATH 未生效
  - 处理：安装 Rustup 后重启终端

- `link.exe not found`
  - 原因：未在 VS Build Tools 环境中运行
  - 处理：使用 VS 2022 x64 开发者命令环境

---

## English

### Overview
`tt-audio-lab` is a desktop audio spectrum widget for always-on use cases.  
It focuses on visual quality, low-latency updates, and smooth behavior on high refresh-rate displays.

### Key Features
- Multiple spectrum styles: `bars / wave / radial / mirror / spiral / matrix / particles / waterfall / radar`
- Bars renderer pipeline with automatic fallback: `WebGL2 -> Canvas2D -> DOM`
- Non-bars styles rendered with a unified `Canvas2D` component path
- Per-style tuning controls: speed / density / glow / trail
- Persistent settings with restore-on-launch behavior
- Style switching available in both dev and release (inside settings modal)
- Immersive mode with explicit enter/exit controls
- Tray controls: show/hide, pause/resume, settings, quit
- Window modes: normal / desktop widget / overlay

### Requirements
- Node.js 20+
- Rust toolchain (`cargo` in PATH)
- Visual Studio C++ Build Tools on Windows

### Quick Start
1. Install dependencies
```powershell
npm install
```

2. Frontend dev server (Vite only)
```powershell
npm run dev
```

3. Desktop dev mode (Tauri)
```powershell
npm run tauri dev
```

4. Build frontend assets
```powershell
npm run build
```

### Useful Scripts
- `npm run dev`
- `npm run tauri dev`
- `npm run build`
- `npm run lint:comment`

### Docs
- PRD: `docs/PRD.md`
- Architecture: `docs/ARCH.md`
- Coding Standard: `docs/CODING_STANDARD.md`
- Session Context: `docs/SESSION_SUMMARY_2026-02-15.md`
