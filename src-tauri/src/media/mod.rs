pub mod audio;
pub mod errors;
pub mod export;
pub mod ffmpeg;
pub mod metadata;
pub mod overlays;
pub mod server;
pub mod timeline;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceVideo {
    pub id: String,
    pub path: String,
    pub file_name: String,
    pub duration_ms: u64,
    pub width: u32,
    pub height: u32,
    pub fps: Option<f64>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub file_size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineClip {
    pub id: String,
    pub source_video_id: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextOverlay {
    pub id: String,
    pub text: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub position: String,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub font_size: u32,
    pub color: String,
    pub background_color: Option<String>,
    pub opacity: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundAudio {
    pub path: String,
    pub start_ms: u64,
    pub end_ms: Option<u64>,
    pub volume: f64,
    #[serde(rename = "loop")]
    pub looped: bool,
    pub fade_in_ms: Option<u64>,
    pub fade_out_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSettings {
    pub format: String,
    pub resolution: String,
    pub custom_width: Option<u32>,
    pub custom_height: Option<u32>,
    pub optimize_for_web: bool,
    pub quality: String,
    pub original_volume: f64,
    pub master_volume: f64,
    /// Renditions for HLS export, e.g. ["1080p", "720p"]. Defaults to all.
    pub hls_variants: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoProject {
    pub id: String,
    pub name: String,
    pub source_videos: Vec<SourceVideo>,
    pub clips: Vec<TimelineClip>,
    pub text_overlays: Vec<TextOverlay>,
    pub background_audio: Option<BackgroundAudio>,
    pub export_settings: ExportSettings,
}
