use std::process::Command;

use serde::{Deserialize, Serialize};

use super::errors::AppError;
use super::ffmpeg::ffprobe_path;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoMetadata {
    pub duration_ms: u64,
    pub width: u32,
    pub height: u32,
    pub fps: Option<f64>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub bitrate: Option<u64>,
    pub file_size_bytes: Option<u64>,
    pub has_audio: bool,
}

#[derive(Debug, Deserialize)]
struct FfprobeOutput {
    #[serde(default)]
    streams: Vec<FfprobeStream>,
    format: Option<FfprobeFormat>,
}

#[derive(Debug, Deserialize)]
struct FfprobeStream {
    codec_type: Option<String>,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    avg_frame_rate: Option<String>,
    r_frame_rate: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
    bit_rate: Option<String>,
    size: Option<String>,
}

pub fn read_metadata(path: &str) -> Result<VideoMetadata, AppError> {
    if !std::path::Path::new(path).is_file() {
        return Err(AppError::InvalidInput(format!("File not found: {path}")));
    }

    let output = Command::new(ffprobe_path()?)
        .args([
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(AppError::Metadata(if stderr.is_empty() {
            "FFprobe could not analyze this file.".to_string()
        } else {
            stderr
        }));
    }

    let parsed: FfprobeOutput = serde_json::from_slice(&output.stdout)
        .map_err(|e| AppError::Metadata(format!("Could not parse FFprobe output: {e}")))?;

    let video = parsed
        .streams
        .iter()
        .find(|s| s.codec_type.as_deref() == Some("video"))
        .ok_or_else(|| AppError::Metadata("No video stream found in this file.".to_string()))?;
    let audio = parsed
        .streams
        .iter()
        .find(|s| s.codec_type.as_deref() == Some("audio"));

    let format = parsed.format.as_ref();
    let duration_ms = format
        .and_then(|f| f.duration.as_deref())
        .and_then(|d| d.parse::<f64>().ok())
        .map(|secs| (secs * 1000.0).round() as u64)
        .ok_or_else(|| AppError::Metadata("Could not determine video duration.".to_string()))?;

    let fps = video
        .avg_frame_rate
        .as_deref()
        .and_then(parse_frame_rate)
        .or_else(|| video.r_frame_rate.as_deref().and_then(parse_frame_rate));

    Ok(VideoMetadata {
        duration_ms,
        width: video.width.unwrap_or(0),
        height: video.height.unwrap_or(0),
        fps,
        video_codec: video.codec_name.clone(),
        audio_codec: audio.and_then(|a| a.codec_name.clone()),
        bitrate: format
            .and_then(|f| f.bit_rate.as_deref())
            .and_then(|b| b.parse().ok()),
        file_size_bytes: format
            .and_then(|f| f.size.as_deref())
            .and_then(|s| s.parse().ok()),
        has_audio: audio.is_some(),
    })
}

fn parse_frame_rate(rate: &str) -> Option<f64> {
    let (num, den) = rate.split_once('/')?;
    let num: f64 = num.parse().ok()?;
    let den: f64 = den.parse().ok()?;
    if den == 0.0 || num == 0.0 {
        None
    } else {
        Some(num / den)
    }
}
