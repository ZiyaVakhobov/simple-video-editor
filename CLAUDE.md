# CLAUDE.md

## Project Overview

We are building a lightweight desktop video editor and converter.

The app should let users:

1. Load video files.
2. Convert videos to MP4, HLS, DASH.
3. Extract audio as MP3 or WAV.
4. Cut videos by start/end time.
5. Join multiple clips on a simple timeline.
6. Add text overlays.
7. Add background music.
8. Adjust original audio and background music volume.
9. Export using selected resolution, defaulting to the source resolution.
10. Optimize exported videos for web playback.

## Tech Stack

Use this stack unless explicitly instructed otherwise:

```text
Tauri v2
Rust backend
Svelte or React frontend
TypeScript frontend code
FFmpeg sidecar binary
FFprobe for metadata
SQLite only if project persistence is needed
```

Do not build a custom video decoding/rendering engine. Use FFmpeg for media processing.

## High-Level Architecture

The frontend handles:

```text
UI
Video preview
Timeline editing
User input
Export settings
Progress display
```

The Rust backend handles:

```text
File access
FFmpeg execution
FFprobe metadata reading
Project validation
Export job management
Progress parsing
Path safety
Error handling
```

FFmpeg should be wrapped behind a clean Rust service layer. Do not call FFmpeg directly from random parts of the codebase.

Preferred backend modules:

```text
src-tauri/src/media/
  metadata.rs
  ffmpeg.rs
  export.rs
  timeline.rs
  audio.rs
  overlays.rs
  errors.rs
```

Preferred frontend modules:

```text
src/lib/
  components/
    VideoPreview.svelte
    Timeline.svelte
    ExportPanel.svelte
    TextOverlayPanel.svelte
    AudioPanel.svelte
  stores/
    project.ts
    export.ts
  types/
    project.ts
    media.ts
```

Adjust paths to match the actual frontend framework.

## Core Rule

Always keep the app simple first.

Do not add advanced features before the MVP is stable.

MVP priority order:

```text
1. Import video
2. Read metadata with ffprobe
3. Preview video
4. Trim video by start/end time
5. Export MP4
6. Choose export resolution
7. Optimize MP4 for web
8. Extract MP3/WAV
9. Join clips
10. Add text overlay
11. Add background music
12. Adjust audio volume
13. Export HLS
14. Export DASH
```

## Media Time Representation

Internally store time values in milliseconds as integers.

Use:

```ts
type Milliseconds = number;
```

Avoid storing important edit timings as floating-point seconds in project state.

Only convert to FFmpeg time format when generating commands.

Example:

```json
{
  "startMs": 5000,
  "endMs": 20800
}
```

## Project Data Model

Use a project model similar to this:

```ts
export type ExportFormat = "mp4" | "hls" | "dash" | "mp3" | "wav";

export type ResolutionPreset =
  | "original"
  | "2160p"
  | "1440p"
  | "1080p"
  | "720p"
  | "480p"
  | "360p"
  | "custom";

export interface SourceVideo {
  id: string;
  path: string;
  fileName: string;
  durationMs: number;
  width: number;
  height: number;
  fps?: number;
  videoCodec?: string;
  audioCodec?: string;
  fileSizeBytes?: number;
}

export interface TimelineClip {
  id: string;
  sourceVideoId: string;
  startMs: number;
  endMs: number;
  order: number;
}

export interface TextOverlay {
  id: string;
  text: string;
  startMs: number;
  endMs: number;
  position:
    | "top-left"
    | "top-center"
    | "top-right"
    | "center"
    | "bottom-left"
    | "bottom-center"
    | "bottom-right"
    | "custom";
  x?: number;
  y?: number;
  fontSize: number;
  color: string;
  backgroundColor?: string;
  opacity?: number;
}

export interface BackgroundAudio {
  path: string;
  startMs: number;
  endMs?: number;
  volume: number;
  loop: boolean;
  fadeInMs?: number;
  fadeOutMs?: number;
}

export interface ExportSettings {
  format: ExportFormat;
  resolution: ResolutionPreset;
  customWidth?: number;
  customHeight?: number;
  optimizeForWeb: boolean;
  quality: "small" | "balanced" | "high";
  originalVolume: number;
  masterVolume: number;
}

export interface VideoProject {
  id: string;
  name: string;
  sourceVideos: SourceVideo[];
  clips: TimelineClip[];
  textOverlays: TextOverlay[];
  backgroundAudio?: BackgroundAudio;
  exportSettings: ExportSettings;
}
```

## FFmpeg Strategy

Use FFmpeg as a sidecar binary.

Do not assume FFmpeg is globally installed.

Do not execute FFmpeg through raw shell strings.

Correct approach:

```rust
Command::new(ffmpeg_path)
    .arg("-i")
    .arg(input_path)
    .arg(...)
```

Avoid:

```rust
Command::new("sh")
    .arg("-c")
    .arg(format!("ffmpeg -i {}", user_path))
```

Never concatenate user-controlled paths into shell commands.

## FFprobe Metadata

When a video is imported, run ffprobe to get:

```text
Duration
Width
Height
Frame rate
Video codec
Audio codec
Bitrate
File size
```

Return this metadata to the frontend as typed JSON.

If metadata extraction fails, show a useful error message.

## Export Rules

### MP4 Export

Default MP4 export should use:

```text
H.264 video
AAC audio
MP4 container
Original resolution by default
Web optimization enabled by default
```

For web optimization, use fast-start MP4 output.

FFmpeg concept:

```text
-movflags +faststart
```

### Resolution

Default resolution is the original source resolution.

When the user selects a resolution preset, preserve aspect ratio.

Do not stretch the video.

If the source is vertical, preserve vertical orientation.

Examples:

```text
1920x1080 -> 1280x720
1080x1920 -> 720x1280
3840x2160 -> 1920x1080
```

### Quality Presets

Use simple quality presets.

```text
Small:
  Lower bitrate
  Smaller file size
  Good for chat/email

Balanced:
  Good default
  Web-friendly quality and size

High:
  Better quality
  Larger file size
```

Do not expose too many advanced codec settings in the first version.

## HLS Export

HLS export should create a folder, not a single file.

Expected output:

```text
output-folder/
  playlist.m3u8
  segment_000.ts
  segment_001.ts
  segment_002.ts
```

HLS should be added after MP4 export is stable.

## DASH Export

DASH export should create a folder, not a single file.

Expected output:

```text
output-folder/
  manifest.mpd
  init-stream0.m4s
  chunk-stream0-00001.m4s
  chunk-stream1-00001.m4s
```

DASH should be added after HLS export is stable.

## Audio Extraction

Support:

```text
MP3
WAV
```

Use MP3 for compressed audio.

Use WAV for high-quality uncompressed audio.

Correct spelling is WAV, not WAW.

## Cutting and Trimming

Support these operations:

```text
Trim from start
Trim from end
Extract selected range
Split at playhead
Remove selected range
```

For MVP, implement:

```text
Start time
End time
Export selected range
```

Then add timeline split/delete later.

## Joining Clips

Use a simple single-track timeline first.

Support:

```text
Add clip
Remove clip
Reorder clip
Set clip start/end
Export joined clips
```

Do not build multi-track editing in the MVP.

## Text Overlays

Support basic text overlay features:

```text
Text content
Start time
End time
Position
Font size
Text color
Optional background color
Optional opacity
```

Start with fixed positions:

```text
Top left
Top center
Top right
Center
Bottom left
Bottom center
Bottom right
```

Add custom X/Y later.

Text rendering should be done through FFmpeg filters during export.

The frontend preview can show an approximate overlay, but FFmpeg output is the source of truth.

## Background Music

Support:

```text
Add audio file
Set background music volume
Set original video volume
Mute original video
Loop background music
Fade in
Fade out
```

For MVP, implement:

```text
Add music
Set music volume
Set original video volume
Export mixed audio
```

Add looping and fades later.

## Export Progress

FFmpeg progress should be shown in the UI.

Parse FFmpeg output to estimate progress using:

```text
Current processed time
Total duration
Percentage
```

The frontend should show:

```text
Progress bar
Current status
Cancel export button
Output path after completion
```

Cancel export should terminate the running FFmpeg process safely.

## Error Handling

All backend commands must return structured errors.

Good errors:

```text
Could not read video metadata.
FFmpeg export failed.
Output folder is not writable.
Unsupported input format.
The selected clip has invalid start/end times.
```

Bad errors:

```text
Failed.
Unknown error.
Command failed.
```

Include technical details in logs, but show user-friendly messages in the UI.

## Security Rules

This is a desktop app that handles local files. Be careful.

Follow these rules:

```text
Never run raw shell strings.
Never concatenate user file paths into shell commands.
Always pass paths as command arguments.
Validate input and output paths.
Do not overwrite files without confirmation.
Do not delete source files.
Do not upload files anywhere.
Do not add network features unless explicitly requested.
```

## Frontend UI Principles

The app should feel simple and practical.

Preferred layout:

```text
Left/top: video preview
Bottom: simple timeline
Right: settings panel
Top bar: import, save, export
```

Main screens:

```text
Home
Editor
Converter
Export Progress
Settings
```

Avoid complex professional editor UI in the first version.

This is not Adobe Premiere.

This is a simple video editor and converter.

## UI Copy

Use clear labels:

```text
Import Video
Start Time
End Time
Cut
Join Clips
Add Text
Add Music
Original Volume
Music Volume
Export
Optimize for Web
Resolution
Quality
Output Folder
```

Avoid technical labels unless necessary.

## Testing Strategy

When changing Rust backend code, run:

```bash
cargo check
cargo clippy
cargo test
```

When changing frontend code, run the project's available commands, usually:

```bash
npm run check
npm run lint
npm run build
```

If a command does not exist, inspect `package.json` and use the correct available command.

Do not claim tests passed unless they were actually run.

## Development Workflow

Before making changes:

```text
1. Inspect the project structure.
2. Identify the framework and package manager.
3. Read package.json, Cargo.toml, and existing source files.
4. Make the smallest useful change.
5. Run relevant checks.
6. Fix errors.
7. Summarize what changed.
```

Do not rewrite the whole app unless explicitly requested.

Do not introduce large dependencies without explaining why.

Do not commit changes unless explicitly asked.

## Coding Standards

Rust:

```text
Use Result types properly.
Use thiserror or anyhow where appropriate.
Keep command handlers thin.
Put business logic in services/modules.
Avoid unwrap() in production code.
Avoid panic! for recoverable errors.
Use typed structs for command inputs/outputs.
```

TypeScript:

```text
Use explicit types for project data.
Avoid any unless necessary.
Keep stores simple.
Keep UI components focused.
Validate user input before calling backend commands.
```

## Tauri Command Style

Tauri commands should be thin wrappers.

Good:

```rust
#[tauri::command]
async fn get_video_metadata(path: String) -> Result<VideoMetadata, AppError> {
    media::metadata::read_metadata(path).await
}
```

Bad:

```rust
#[tauri::command]
async fn get_video_metadata(path: String) -> Result<String, String> {
    // 200 lines of ffprobe parsing here
}
```

## Definition of Done

A feature is done only when:

```text
The UI works.
The backend command works.
Errors are handled.
The feature works with real local files.
The code is typed.
Relevant checks pass.
The implementation does not use raw shell command strings.
```

## Current Product Direction

Build the app in this order:

### Phase 1 — Basic Converter

```text
Import video
Read metadata
Preview video
Export MP4
Extract MP3
Extract WAV
Choose output folder
Show progress
```

### Phase 2 — Simple Editing

```text
Trim by start/end time
Join clips
Simple timeline
Export edited MP4
```

### Phase 3 — Overlays and Audio

```text
Add text overlay
Add background music
Adjust original volume
Adjust music volume
```

### Phase 4 — Web Optimization

```text
Web MP4 preset
Fast start MP4
Resolution presets
Quality presets
Bitrate presets
Remove metadata option
```

### Phase 5 — Streaming Formats

```text
HLS export
DASH export
Output folder validation
Manifest/playlist generation
```

## Important Constraints

Do not overcomplicate the MVP.

Do not build:

```text
Multi-track professional timeline
Keyframe animation
Color grading
Effects marketplace
Cloud rendering
Collaboration
Plugin system
AI video generation
```

unless explicitly requested later.

The first goal is a reliable desktop app for:

```text
Convert
Cut
Join
Text
Music
Export
Optimize for web
```

## Final Reminder

When implementing features, prefer boring, reliable code.

Use FFmpeg for media processing.

Use Rust for safe backend orchestration.

Use the frontend only for UI, preview, and user interaction.

Keep the product simple, fast, and useful.
