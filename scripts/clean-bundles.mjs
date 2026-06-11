#!/usr/bin/env node
import { rmSync } from "node:fs";
import { join } from "node:path";

rmSync(join(process.cwd(), "src-tauri", "target", "release", "bundle"), {
  recursive: true,
  force: true
});
