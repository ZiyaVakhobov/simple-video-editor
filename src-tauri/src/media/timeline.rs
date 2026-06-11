use super::errors::AppError;
use super::{SourceVideo, TimelineClip, VideoProject};

pub fn ordered_clips(project: &VideoProject) -> Vec<TimelineClip> {
    let mut clips = project.clips.clone();
    clips.sort_by_key(|c| c.order);
    clips
}

pub fn source_of<'a>(
    project: &'a VideoProject,
    source_video_id: &str,
) -> Result<&'a SourceVideo, AppError> {
    project
        .source_videos
        .iter()
        .find(|s| s.id == source_video_id)
        .ok_or_else(|| {
            AppError::InvalidInput(format!("Clip references unknown source video: {source_video_id}"))
        })
}

pub fn total_duration_ms(project: &VideoProject) -> u64 {
    project
        .clips
        .iter()
        .map(|c| c.end_ms.saturating_sub(c.start_ms))
        .sum()
}

pub fn validate(project: &VideoProject) -> Result<(), AppError> {
    if project.clips.is_empty() {
        return Err(AppError::InvalidInput(
            "The timeline is empty. Add at least one clip before exporting.".to_string(),
        ));
    }

    for clip in &project.clips {
        let source = source_of(project, &clip.source_video_id)?;
        if clip.start_ms >= clip.end_ms {
            return Err(AppError::InvalidInput(format!(
                "The clip from \"{}\" has invalid start/end times.",
                source.file_name
            )));
        }
        if clip.end_ms > source.duration_ms {
            return Err(AppError::InvalidInput(format!(
                "The clip from \"{}\" ends after the video does.",
                source.file_name
            )));
        }
        if !std::path::Path::new(&source.path).is_file() {
            return Err(AppError::InvalidInput(format!(
                "Source file is missing: {}",
                source.path
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::ExportSettings;

    fn test_project() -> VideoProject {
        VideoProject {
            id: "p1".into(),
            name: "Test".into(),
            source_videos: vec![],
            clips: vec![],
            text_overlays: vec![],
            background_audio: None,
            export_settings: ExportSettings {
                format: "mp4".into(),
                resolution: "original".into(),
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
    fn empty_timeline_rejected() {
        assert!(validate(&test_project()).is_err());
    }

    #[test]
    fn duration_sums_clips() {
        let mut p = test_project();
        p.clips.push(TimelineClip {
            id: "c1".into(),
            source_video_id: "s1".into(),
            start_ms: 5_000,
            end_ms: 20_800,
            order: 0,
        });
        p.clips.push(TimelineClip {
            id: "c2".into(),
            source_video_id: "s1".into(),
            start_ms: 0,
            end_ms: 4_200,
            order: 1,
        });
        assert_eq!(total_duration_ms(&p), 20_000);
    }
}
