use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use super::errors::AppError;
use super::ffmpeg::{ffmpeg_path, ms_to_timestamp, parse_progress_line, JobRegistry};
use super::{audio, overlays, timeline, VideoProject};

pub const PROGRESS_EVENT: &str = "export://progress";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportEvent {
    pub job_id: String,
    pub status: String, // running | complete | error | cancelled
    pub percent: f64,
    pub out_time_ms: u64,
    pub total_ms: u64,
    pub output_path: String,
    pub message: Option<String>,
}

struct BuiltExport {
    args: Vec<String>,
    total_ms: u64,
}

fn is_audio_only(format: &str) -> bool {
    matches!(format, "mp3" | "wav")
}

fn is_folder_output(format: &str) -> bool {
    matches!(format, "hls" | "dash")
}

fn quality_crf(quality: &str) -> &'static str {
    match quality {
        "small" => "28",
        "high" => "18",
        _ => "23", // balanced
    }
}

fn quality_audio_bitrate(quality: &str) -> &'static str {
    match quality {
        "small" => "128k",
        "high" => "192k",
        _ => "160k",
    }
}

fn preset_short_side(preset: &str) -> Option<u32> {
    match preset {
        "2160p" => Some(2160),
        "1440p" => Some(1440),
        "1080p" => Some(1080),
        "720p" => Some(720),
        "480p" => Some(480),
        "360p" => Some(360),
        _ => None,
    }
}

fn round_even(v: u32) -> u32 {
    if v.is_multiple_of(2) {
        v
    } else {
        v + 1
    }
}

#[derive(Debug)]
struct HlsVariant {
    name: String,
    width: u32,
    height: u32,
    bitrate_k: u32,
}

fn bitrate_for_side(side: u32) -> u32 {
    match side {
        s if s >= 2160 => 12000,
        s if s >= 1440 => 8000,
        s if s >= 1080 => 5000,
        s if s >= 720 => 2800,
        s if s >= 480 => 1400,
        _ => 800,
    }
}

/// Resolves the requested HLS renditions against the source resolution.
/// Never upscales; duplicate renditions collapse into one (e.g. a 480p
/// source requested at 1080p+720p+480p produces a single 480p variant).
fn resolve_hls_variants(project: &VideoProject) -> Result<Vec<HlsVariant>, AppError> {
    let settings = &project.export_settings;
    let mut requested: Vec<u32> = settings
        .hls_variants
        .as_ref()
        .map(|v| v.iter().filter_map(|s| preset_short_side(s)).collect())
        .filter(|v: &Vec<u32>| !v.is_empty())
        .unwrap_or_else(|| vec![1080, 720, 480, 360]);
    requested.sort_unstable_by(|a, b| b.cmp(a));
    requested.dedup();

    let clips = timeline::ordered_clips(project);
    let first = clips
        .first()
        .ok_or_else(|| AppError::InvalidInput("The timeline is empty.".to_string()))?;
    let src = timeline::source_of(project, &first.source_video_id)?;
    let (w, h) = (src.width.max(2), src.height.max(2));

    let quality_pct: u32 = match settings.quality.as_str() {
        "small" => 70,
        "high" => 140,
        _ => 100,
    };

    let mut variants: Vec<HlsVariant> = Vec::new();
    for side in requested {
        let (tw, th) = if w >= h {
            let th2 = side.min(h);
            (
                round_even(((w as u64 * th2 as u64) / h as u64) as u32),
                round_even(th2),
            )
        } else {
            let tw2 = side.min(w);
            (
                round_even(tw2),
                round_even(((h as u64 * tw2 as u64) / w as u64) as u32),
            )
        };
        let actual_side = tw.min(th);
        let name = format!("{actual_side}p");
        if variants.iter().any(|v| v.name == name) {
            continue;
        }
        variants.push(HlsVariant {
            name,
            width: tw,
            height: th,
            bitrate_k: bitrate_for_side(actual_side) * quality_pct / 100,
        });
    }

    if variants.is_empty() {
        return Err(AppError::InvalidInput(
            "Select at least one HLS resolution.".to_string(),
        ));
    }
    Ok(variants)
}

/// Original dimensions of the first clip's source, rounded to even values.
fn original_dims(project: &VideoProject) -> Result<(u32, u32), AppError> {
    let clips = timeline::ordered_clips(project);
    let first = clips
        .first()
        .ok_or_else(|| AppError::InvalidInput("The timeline is empty.".to_string()))?;
    let src = timeline::source_of(project, &first.source_video_id)?;
    Ok((round_even(src.width.max(2)), round_even(src.height.max(2))))
}

/// Computes target output dimensions, preserving the aspect ratio and
/// orientation of the first clip's source video.
fn target_dims(project: &VideoProject) -> Result<(u32, u32), AppError> {
    let clips = timeline::ordered_clips(project);
    let first = clips
        .first()
        .ok_or_else(|| AppError::InvalidInput("The timeline is empty.".to_string()))?;
    let src = timeline::source_of(project, &first.source_video_id)?;
    let (w, h) = (src.width.max(2), src.height.max(2));
    let settings = &project.export_settings;

    let (tw, th) = match settings.resolution.as_str() {
        "original" => (w, h),
        "custom" => (
            settings.custom_width.unwrap_or(w),
            settings.custom_height.unwrap_or(h),
        ),
        preset => {
            let side = preset_short_side(preset).ok_or_else(|| {
                AppError::InvalidInput(format!("Unknown resolution preset: {preset}"))
            })?;
            if w >= h {
                let th = side.min(h);
                let tw = ((w as u64 * th as u64) / h as u64) as u32;
                (tw, th)
            } else {
                let tw = side.min(w);
                let th = ((h as u64 * tw as u64) / w as u64) as u32;
                (tw, th)
            }
        }
    };

    Ok((round_even(tw), round_even(th)))
}

fn validate_output(project: &VideoProject, output_path: &str) -> Result<(), AppError> {
    for src in &project.source_videos {
        if src.path == output_path {
            return Err(AppError::InvalidInput(
                "The output path would overwrite a source video.".to_string(),
            ));
        }
    }

    let format = project.export_settings.format.as_str();
    let path = std::path::Path::new(output_path);

    if is_folder_output(format) {
        std::fs::create_dir_all(path)
            .map_err(|_| AppError::OutputNotWritable(output_path.to_string()))?;
    } else if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.is_dir() {
            return Err(AppError::OutputNotWritable(output_path.to_string()));
        }
    }

    Ok(())
}

fn build_export_args(project: &VideoProject, output_path: &str) -> Result<BuiltExport, AppError> {
    timeline::validate(project)?;
    validate_output(project, output_path)?;

    let clips = timeline::ordered_clips(project);
    let total_ms = timeline::total_duration_ms(project);
    let settings = &project.export_settings;
    let audio_only = is_audio_only(&settings.format);
    let clip_count = clips.len();

    let mut args: Vec<String> = vec![
        "-y".into(),
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-nostats".into(),
        "-progress".into(),
        "pipe:1".into(),
    ];

    // One input per clip, trimmed with input seeking.
    for clip in &clips {
        let src = timeline::source_of(project, &clip.source_video_id)?;
        args.push("-ss".into());
        args.push(ms_to_timestamp(clip.start_ms));
        args.push("-t".into());
        args.push(ms_to_timestamp(clip.end_ms - clip.start_ms));
        args.push("-i".into());
        args.push(src.path.clone());
    }

    let music_input_idx = clip_count;
    if let Some(bg) = &project.background_audio {
        if !std::path::Path::new(&bg.path).is_file() {
            return Err(AppError::InvalidInput(format!(
                "Background music file is missing: {}",
                bg.path
            )));
        }
        if bg.looped {
            args.push("-stream_loop".into());
            args.push("-1".into());
        }
        args.push("-i".into());
        args.push(bg.path.clone());
    }

    let is_hls = settings.format == "hls";

    // Filter graph
    let mut filters: Vec<String> = Vec::new();

    if !audio_only {
        // HLS scales per rendition later; concat at original resolution.
        let (tw, th) = if is_hls {
            original_dims(project)?
        } else {
            target_dims(project)?
        };
        for i in 0..clip_count {
            filters.push(format!(
                "[{i}:v]scale={tw}:{th}:force_original_aspect_ratio=decrease,\
                 pad={tw}:{th}:(ow-iw)/2:(oh-ih)/2,setsar=1[v{i}]"
            ));
        }
    }

    for (i, clip) in clips.iter().enumerate() {
        let src = timeline::source_of(project, &clip.source_video_id)?;
        if src.audio_codec.is_some() {
            filters.push(format!("[{i}:a]anull[a{i}]"));
        } else {
            // Silent track for sources without audio so concat stays aligned.
            let dur = (clip.end_ms - clip.start_ms) as f64 / 1000.0;
            filters.push(format!(
                "anullsrc=channel_layout=stereo:sample_rate=44100,atrim=duration={dur:.3}[a{i}]"
            ));
        }
    }

    let concat_inputs: String = (0..clip_count)
        .map(|i| {
            if audio_only {
                format!("[a{i}]")
            } else {
                format!("[v{i}][a{i}]")
            }
        })
        .collect();

    let mut video_label = String::new();
    if audio_only {
        filters.push(format!("{concat_inputs}concat=n={clip_count}:v=0:a=1[acat]"));
    } else {
        filters.push(format!(
            "{concat_inputs}concat=n={clip_count}:v=1:a=1[vcat][acat]"
        ));
        video_label = "vcat".to_string();

        if !project.text_overlays.is_empty() {
            let chain: Vec<String> = project
                .text_overlays
                .iter()
                .map(overlays::drawtext_filter)
                .collect();
            filters.push(format!("[vcat]{}[vtxt]", chain.join(",")));
            video_label = "vtxt".to_string();
        }
    }

    filters.push(format!(
        "[acat]volume={:.3}[aorig]",
        settings.original_volume.clamp(0.0, 4.0)
    ));
    let mut audio_label = "aorig".to_string();

    if let Some(bg) = &project.background_audio {
        filters.push(audio::background_music_chain(bg, music_input_idx, total_ms));
        filters.push("[aorig][bgm]amix=inputs=2:duration=first:normalize=0[amixed]".to_string());
        audio_label = "amixed".to_string();
    }

    filters.push(format!(
        "[{audio_label}]volume={:.3}[aout]",
        settings.master_volume.clamp(0.0, 4.0)
    ));

    // HLS rendition ladder: split the edited video/audio per variant.
    let hls_ladder: Option<Vec<HlsVariant>> = if is_hls {
        let ladder = resolve_hls_variants(project)?;
        let n = ladder.len();
        let split_outs: String = (0..n).map(|i| format!("[vs{i}]")).collect();
        filters.push(format!("[{video_label}]split={n}{split_outs}"));
        for (i, v) in ladder.iter().enumerate() {
            filters.push(format!(
                "[vs{i}]scale={w}:{h}:force_original_aspect_ratio=decrease,\
                 pad={w}:{h}:(ow-iw)/2:(oh-ih)/2,setsar=1[vv{i}]",
                w = v.width,
                h = v.height
            ));
        }
        let asplit_outs: String = (0..n).map(|i| format!("[aa{i}]")).collect();
        filters.push(format!("[aout]asplit={n}{asplit_outs}"));
        Some(ladder)
    } else {
        None
    };

    args.push("-filter_complex".into());
    args.push(filters.join(";"));

    if let Some(ladder) = &hls_ladder {
        for i in 0..ladder.len() {
            args.push("-map".into());
            args.push(format!("[vv{i}]"));
            args.push("-map".into());
            args.push(format!("[aa{i}]"));
        }
    } else {
        if !audio_only {
            args.push("-map".into());
            args.push(format!("[{video_label}]"));
        }
        args.push("-map".into());
        args.push("[aout]".into());
    }

    let crf = quality_crf(&settings.quality);
    let audio_bitrate = quality_audio_bitrate(&settings.quality);

    match settings.format.as_str() {
        "mp4" => {
            args.extend([
                "-c:v".into(),
                "libx264".into(),
                "-preset".into(),
                "veryfast".into(),
                "-crf".into(),
                crf.into(),
                "-pix_fmt".into(),
                "yuv420p".into(),
                "-c:a".into(),
                "aac".into(),
                "-b:a".into(),
                audio_bitrate.into(),
            ]);
            if settings.optimize_for_web {
                args.push("-movflags".into());
                args.push("+faststart".into());
            }
            args.push(output_path.into());
        }
        "hls" => {
            let ladder = hls_ladder
                .as_ref()
                .expect("HLS ladder is built for hls format");
            let dir = std::path::Path::new(output_path);
            for v in ladder {
                std::fs::create_dir_all(dir.join(&v.name))
                    .map_err(|_| AppError::OutputNotWritable(output_path.to_string()))?;
            }

            args.extend([
                "-c:v".into(),
                "libx264".into(),
                "-preset".into(),
                "veryfast".into(),
                "-pix_fmt".into(),
                "yuv420p".into(),
                "-c:a".into(),
                "aac".into(),
                "-b:a".into(),
                audio_bitrate.into(),
            ]);
            for (i, v) in ladder.iter().enumerate() {
                args.push(format!("-b:v:{i}"));
                args.push(format!("{}k", v.bitrate_k));
                args.push(format!("-maxrate:v:{i}"));
                args.push(format!("{}k", v.bitrate_k * 12 / 10));
                args.push(format!("-bufsize:v:{i}"));
                args.push(format!("{}k", v.bitrate_k * 2));
            }

            let var_stream_map = ladder
                .iter()
                .enumerate()
                .map(|(i, v)| format!("v:{i},a:{i},name:{}", v.name))
                .collect::<Vec<_>>()
                .join(" ");

            args.extend([
                "-f".into(),
                "hls".into(),
                "-hls_time".into(),
                "6".into(),
                "-hls_list_size".into(),
                "0".into(),
                "-hls_playlist_type".into(),
                "vod".into(),
                "-master_pl_name".into(),
                "master.m3u8".into(),
                "-var_stream_map".into(),
                var_stream_map,
                "-hls_segment_filename".into(),
                dir.join("%v").join("segment_%03d.ts").to_string_lossy().into_owned(),
                dir.join("%v").join("playlist.m3u8").to_string_lossy().into_owned(),
            ]);
        }
        "dash" => {
            let dir = std::path::Path::new(output_path);
            args.extend([
                "-c:v".into(),
                "libx264".into(),
                "-preset".into(),
                "veryfast".into(),
                "-crf".into(),
                crf.into(),
                "-pix_fmt".into(),
                "yuv420p".into(),
                "-c:a".into(),
                "aac".into(),
                "-b:a".into(),
                audio_bitrate.into(),
                "-f".into(),
                "dash".into(),
                "-seg_duration".into(),
                "6".into(),
                dir.join("manifest.mpd").to_string_lossy().into_owned(),
            ]);
        }
        "mp3" => {
            args.extend([
                "-vn".into(),
                "-c:a".into(),
                "libmp3lame".into(),
                "-b:a".into(),
                match settings.quality.as_str() {
                    "small" => "128k".into(),
                    "high" => "256k".into(),
                    _ => "192k".to_string(),
                },
            ]);
            args.push(output_path.into());
        }
        "wav" => {
            args.extend(["-vn".into(), "-c:a".into(), "pcm_s16le".into()]);
            args.push(output_path.into());
        }
        other => {
            return Err(AppError::InvalidInput(format!(
                "Unsupported export format: {other}"
            )));
        }
    }

    Ok(BuiltExport { args, total_ms })
}

fn new_job_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("job-{nanos}")
}

fn emit_event(app: &AppHandle, event: &ExportEvent) {
    let _ = app.emit(PROGRESS_EVENT, event.clone());
}

/// Starts an export job. Returns the job id immediately; progress and
/// completion are reported through the `export://progress` event.
pub fn start_export(
    app: AppHandle,
    registry: Arc<JobRegistry>,
    project: VideoProject,
    output_path: String,
) -> Result<String, AppError> {
    let built = build_export_args(&project, &output_path)?;
    let ffmpeg = ffmpeg_path()?;
    let job_id = new_job_id();

    let mut child = Command::new(ffmpeg)
        .args(&built.args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::ExportFailed("Could not capture FFmpeg output.".to_string()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| AppError::ExportFailed("Could not capture FFmpeg errors.".to_string()))?;

    registry.clear_cancelled(&job_id);
    registry.insert(&job_id, child);

    let total_ms = built.total_ms.max(1);
    let worker_job_id = job_id.clone();

    std::thread::spawn(move || {
        // Collect stderr on a separate thread so the pipe never blocks.
        let stderr_handle = std::thread::spawn(move || {
            let mut buf = String::new();
            let mut reader = BufReader::new(stderr);
            let _ = reader.read_to_string(&mut buf);
            buf
        });

        let mut last_percent = -1.0_f64;
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Some(out_ms) = parse_progress_line(&line) {
                let percent = ((out_ms as f64 / total_ms as f64) * 100.0).min(99.9);
                if percent - last_percent >= 0.5 {
                    last_percent = percent;
                    emit_event(
                        &app,
                        &ExportEvent {
                            job_id: worker_job_id.clone(),
                            status: "running".into(),
                            percent,
                            out_time_ms: out_ms,
                            total_ms,
                            output_path: output_path.clone(),
                            message: None,
                        },
                    );
                }
            }
        }

        let stderr_text = stderr_handle.join().unwrap_or_default();
        let status = registry.take(&worker_job_id).map(|mut child| child.wait());
        let cancelled = registry.was_cancelled(&worker_job_id);
        registry.clear_cancelled(&worker_job_id);

        let event = if cancelled {
            ExportEvent {
                job_id: worker_job_id.clone(),
                status: "cancelled".into(),
                percent: last_percent.max(0.0),
                out_time_ms: 0,
                total_ms,
                output_path: output_path.clone(),
                message: Some("Export cancelled.".to_string()),
            }
        } else {
            match status {
                Some(Ok(exit)) if exit.success() => ExportEvent {
                    job_id: worker_job_id.clone(),
                    status: "complete".into(),
                    percent: 100.0,
                    out_time_ms: total_ms,
                    total_ms,
                    output_path: output_path.clone(),
                    message: None,
                },
                _ => {
                    let detail = stderr_text.trim();
                    let message = if detail.is_empty() {
                        "FFmpeg export failed.".to_string()
                    } else {
                        // Last lines carry the actual error.
                        let tail: Vec<&str> = detail.lines().rev().take(4).collect();
                        tail.into_iter().rev().collect::<Vec<_>>().join("\n")
                    };
                    eprintln!("[export {worker_job_id}] ffmpeg stderr:\n{detail}");
                    ExportEvent {
                        job_id: worker_job_id.clone(),
                        status: "error".into(),
                        percent: last_percent.max(0.0),
                        out_time_ms: 0,
                        total_ms,
                        output_path: output_path.clone(),
                        message: Some(message),
                    }
                }
            }
        };

        emit_event(&app, &event);
    });

    Ok(job_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::{ExportSettings, SourceVideo, TimelineClip};

    fn project_with_clip() -> VideoProject {
        VideoProject {
            id: "p1".into(),
            name: "Test".into(),
            source_videos: vec![SourceVideo {
                id: "s1".into(),
                path: "/nonexistent/video.mp4".into(),
                file_name: "video.mp4".into(),
                duration_ms: 60_000,
                width: 1920,
                height: 1080,
                fps: Some(30.0),
                video_codec: Some("h264".into()),
                audio_codec: Some("aac".into()),
                file_size_bytes: None,
            }],
            clips: vec![TimelineClip {
                id: "c1".into(),
                source_video_id: "s1".into(),
                start_ms: 5_000,
                end_ms: 20_800,
                order: 0,
            }],
            text_overlays: vec![],
            background_audio: None,
            export_settings: ExportSettings {
                format: "mp4".into(),
                resolution: "720p".into(),
                custom_width: None,
                custom_height: None,
                optimize_for_web: true,
                quality: "balanced".into(),
                original_volume: 1.0,
                master_volume: 1.0,
                hls_variants: None,
            },
        }
    }

    #[test]
    fn hls_variants_never_upscale_and_dedupe() {
        let mut p = project_with_clip();
        p.source_videos[0].width = 854;
        p.source_videos[0].height = 480;
        p.export_settings.format = "hls".into();
        p.export_settings.hls_variants = Some(vec![
            "1080p".into(),
            "720p".into(),
            "480p".into(),
            "360p".into(),
        ]);
        let variants = resolve_hls_variants(&p).unwrap();
        assert_eq!(variants.len(), 2); // 480p (clamped 1080/720/480) + 360p
        assert_eq!(variants[0].name, "480p");
        assert_eq!(variants[1].name, "360p");
    }

    #[test]
    fn hls_full_ladder_for_1080p_source() {
        let mut p = project_with_clip();
        p.export_settings.format = "hls".into();
        p.export_settings.hls_variants = None;
        let variants = resolve_hls_variants(&p).unwrap();
        let names: Vec<&str> = variants.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, vec!["1080p", "720p", "480p", "360p"]);
        assert_eq!(variants[0].width, 1920);
        assert_eq!(variants[0].height, 1080);
    }

    #[test]
    fn dims_preserve_landscape_aspect() {
        let p = project_with_clip();
        assert_eq!(target_dims(&p).unwrap(), (1280, 720));
    }

    #[test]
    fn dims_preserve_vertical_aspect() {
        let mut p = project_with_clip();
        p.source_videos[0].width = 1080;
        p.source_videos[0].height = 1920;
        assert_eq!(target_dims(&p).unwrap(), (720, 1280));
    }

    #[test]
    fn dims_4k_to_1080p() {
        let mut p = project_with_clip();
        p.source_videos[0].width = 3840;
        p.source_videos[0].height = 2160;
        p.export_settings.resolution = "1080p".into();
        assert_eq!(target_dims(&p).unwrap(), (1920, 1080));
    }

    #[test]
    fn missing_source_file_rejected() {
        let p = project_with_clip();
        assert!(build_export_args(&p, "/tmp/out.mp4").is_err());
    }
}
