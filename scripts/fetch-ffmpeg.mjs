#!/usr/bin/env node
// Downloads static FFmpeg + FFprobe binaries for the current platform into
// src-tauri/binaries/, named with the Rust target triple as required by
// Tauri's bundle.externalBin. Sources:
//   Linux/Windows: BtbN static GPL builds
//   macOS (arm64 + x86_64): martin-riedl.de static builds
import { execSync } from "node:child_process";
import {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  rmSync,
  writeFileSync
} from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const binDir = join(root, "src-tauri", "binaries");
const tmp = join(binDir, "_tmp");

const triple =
  process.env.TAURI_TARGET_TRIPLE ||
  execSync("rustc -Vv").toString().match(/host: (\S+)/)[1];

const isWindows = triple.includes("windows");
const exe = isWindows ? ".exe" : "";
const ffmpegOut = join(binDir, `ffmpeg-${triple}${exe}`);
const ffprobeOut = join(binDir, `ffprobe-${triple}${exe}`);

if (existsSync(ffmpegOut) && existsSync(ffprobeOut)) {
  console.log(`Sidecar FFmpeg already present for ${triple}, skipping download.`);
  process.exit(0);
}

mkdirSync(tmp, { recursive: true });

async function download(url, dest) {
  console.log(`Downloading ${url}`);
  const res = await fetch(url, { redirect: "follow" });
  if (!res.ok) throw new Error(`Download failed (${res.status}): ${url}`);
  writeFileSync(dest, Buffer.from(await res.arrayBuffer()));
}

function extract(archive) {
  execSync(`tar -xf "${archive}" -C "${tmp}"`, { stdio: "inherit" });
}

function install(src, dest) {
  copyFileSync(src, dest);
  if (!isWindows) chmodSync(dest, 0o755);
  console.log(`Installed ${dest}`);
}

try {
  if (triple.includes("linux")) {
    const name = "ffmpeg-master-latest-linux64-gpl";
    const archive = join(tmp, "ffmpeg.tar.xz");
    await download(
      `https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/${name}.tar.xz`,
      archive
    );
    extract(archive);
    install(join(tmp, name, "bin", "ffmpeg"), ffmpegOut);
    install(join(tmp, name, "bin", "ffprobe"), ffprobeOut);
  } else if (isWindows) {
    const name = "ffmpeg-master-latest-win64-gpl";
    const archive = join(tmp, "ffmpeg.zip");
    await download(
      `https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/${name}.zip`,
      archive
    );
    extract(archive);
    install(join(tmp, name, "bin", "ffmpeg.exe"), ffmpegOut);
    install(join(tmp, name, "bin", "ffprobe.exe"), ffprobeOut);
  } else if (triple.includes("apple")) {
    const arch = triple.startsWith("aarch64") ? "arm64" : "x86_64";
    for (const tool of ["ffmpeg", "ffprobe"]) {
      const archive = join(tmp, `${tool}.zip`);
      await download(
        `https://ffmpeg.martin-riedl.de/redirect/latest/macos/${arch}/release/${tool}.zip`,
        archive
      );
      extract(archive);
      install(join(tmp, tool), tool === "ffmpeg" ? ffmpegOut : ffprobeOut);
    }
  } else {
    throw new Error(`Unsupported target triple: ${triple}`);
  }
} finally {
  rmSync(tmp, { recursive: true, force: true });
}

console.log("FFmpeg sidecar binaries ready.");
