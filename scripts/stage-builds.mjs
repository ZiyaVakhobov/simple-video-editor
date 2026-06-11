#!/usr/bin/env node
import { copyFileSync, cpSync, existsSync, mkdirSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { basename, extname, join } from "node:path";

const platform = process.argv[2];
const allowed = new Set(["windows", "mac", "linux"]);

if (!allowed.has(platform)) {
  console.error("Usage: node scripts/stage-builds.mjs <windows|mac|linux>");
  process.exit(1);
}

const root = process.cwd();
const bundleDir = join(root, "src-tauri", "target");
const latestRoot = join(root, "latest_builds");
const outputDir = join(latestRoot, platform);

const extensionsByPlatform = {
  windows: new Set([".exe", ".msi"]),
  mac: new Set([".dmg"]),
  linux: new Set([".deb", ".rpm", ".AppImage"])
};

function stageFile(source) {
  copyFileSync(source, join(outputDir, basename(source)));
}

function stageDir(source) {
  cpSync(source, join(outputDir, basename(source)), { recursive: true });
}

function walk(dir) {
  if (!existsSync(dir)) return;

  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);

    if (entry.isDirectory()) {
      if (platform === "mac" && entry.name.endsWith(".app")) {
        stageDir(path);
        continue;
      }
      walk(path);
      continue;
    }

    if (!entry.isFile()) continue;
    if (extensionsByPlatform[platform].has(extname(entry.name))) {
      stageFile(path);
    }
  }
}

rmSync(outputDir, { recursive: true, force: true });
mkdirSync(outputDir, { recursive: true });
walk(bundleDir);
writeFileSync(join(outputDir, ".gitkeep"), "");

const staged = readdirSync(outputDir).filter((name) => name !== ".gitkeep");
if (staged.length === 0) {
  console.error(`No ${platform} build artifacts found under ${bundleDir}`);
  process.exit(1);
}

console.log(`Staged ${platform} build artifacts:`);
for (const name of staged) {
  console.log(`- latest_builds/${platform}/${name}`);
}
