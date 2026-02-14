# tt-audio-lab

Windows desktop audio visualization widget based on Tauri 2, Vue 3, and Rust.

## Current Stage
- M1 foundation scaffold
- Frontend shell with Simplified Chinese (`zh-CN`) UI copy
- Rust command bridge (`health_check`, `load_settings`, `save_settings`, `set_quality`)
- Audio analysis event stream (`audio:analysis_frame`): realtime capture first, mock fallback

## Quick Start
### Prerequisites
- Node.js 20+
- Rust toolchain (cargo in PATH)
- Visual Studio C++ Build Tools (for Windows Rust/Tauri build)

1. Install dependencies:
```powershell
npm install
```
2. Run desktop app in development mode:
```powershell
npm run tauri dev
```

If `npm run tauri dev` reports `program not found` for `cargo`, install Rust first:
```powershell
winget install Rustlang.Rustup
```
Then restart terminal (or re-login) to refresh PATH.

## Document Entry
- PRD: `docs/PRD.md`
- Architecture: `docs/ARCH.md`
- Coding standard: `docs/CODING_STANDARD.md`

## Code Rule
- Business code must include Chinese comments.
- Run check before commit:
```powershell
npm run lint:comment
```

## Troubleshooting (Windows)
- `cargo metadata ... program not found`
- Cause: Rust toolchain is not in current terminal PATH.
- Fix (cmd):
```bat
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
where cargo
cargo -V
```
- `link.exe not found`
- Cause: Visual C++ Build Tools environment not loaded.
- Fix: run inside VS 2022 x64 developer environment:
```bat
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat" -arch=x64 -host_arch=x64
where link
```
- Why `localhost:1420` appears in development
- This is expected in Tauri dev mode. Vite dev server runs at `http://localhost:1420`, and Tauri desktop window loads that URL.
