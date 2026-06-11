# Build Instructions

## Linux

Build Linux locally:

```bash
npm install
npm run fetch-ffmpeg
npm run latest:linux
```

The latest Linux package is copied to:

```text
latest_builds/linux/
```

For Ubuntu/Debian, install the `.deb` file:

```bash
sudo apt install ./latest_builds/linux/video-editor_0.1.0_amd64.deb
```

## Windows

Build Windows on a Windows PC.

Install first:

- Node.js 22 or newer
- Rust from `https://rustup.rs`
- Visual Studio Build Tools with **Desktop development with C++**
- WebView2 Runtime, if it is not already installed

Then run in PowerShell from the project folder:

```powershell
npm install
npm run fetch-ffmpeg
npm run tauri -- build
node scripts/stage-builds.mjs windows
```

The latest Windows installers are copied to:

```text
latest_builds/windows/
```

Original Tauri output is also under:

```text
src-tauri/target/release/bundle/
```

Look for `.exe` or `.msi` files.

## macOS Apple Silicon

Build macOS for M-chip Macs on an Apple Silicon Mac.

Install first:

- Node.js 22 or newer
- Rust from `https://rustup.rs`
- Xcode Command Line Tools:

```bash
xcode-select --install
```

Then run from the project folder:

```bash
npm install
npm run fetch-ffmpeg
npm run tauri -- build --target aarch64-apple-darwin
node scripts/stage-builds.mjs mac
```

The latest macOS Apple Silicon build is copied to:

```text
latest_builds/mac/
```

Original Tauri output is also under:

```text
src-tauri/target/release/bundle/
```

Look for `.dmg` or `.app` files.
