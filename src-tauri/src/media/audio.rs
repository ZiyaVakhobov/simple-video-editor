use super::BackgroundAudio;

/// Builds the background music filter chain ending in the [bgm] label.
/// `input_idx` is the FFmpeg input index of the music file and `total_ms`
/// the total output duration, used to bound looped/long music tracks.
pub fn background_music_chain(bg: &BackgroundAudio, input_idx: usize, total_ms: u64) -> String {
    let mut parts: Vec<String> = Vec::new();

    let available_ms = total_ms.saturating_sub(bg.start_ms);
    let duration_ms = match bg.end_ms {
        Some(end) if end > bg.start_ms => (end - bg.start_ms).min(available_ms),
        _ => available_ms,
    };
    parts.push(format!("atrim=duration={:.3}", duration_ms as f64 / 1000.0));

    if let Some(fade_in) = bg.fade_in_ms {
        if fade_in > 0 {
            parts.push(format!("afade=t=in:st=0:d={:.3}", fade_in as f64 / 1000.0));
        }
    }
    if let Some(fade_out) = bg.fade_out_ms {
        if fade_out > 0 {
            let start = duration_ms.saturating_sub(fade_out) as f64 / 1000.0;
            parts.push(format!(
                "afade=t=out:st={:.3}:d={:.3}",
                start,
                fade_out as f64 / 1000.0
            ));
        }
    }

    parts.push(format!("volume={:.3}", bg.volume.clamp(0.0, 4.0)));

    if bg.start_ms > 0 {
        parts.push(format!("adelay={0}|{0}", bg.start_ms));
    }

    format!("[{input_idx}:a]{}[bgm]", parts.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_chain_with_delay_and_volume() {
        let bg = BackgroundAudio {
            path: "/tmp/music.mp3".into(),
            start_ms: 2_000,
            end_ms: None,
            volume: 0.5,
            looped: false,
            fade_in_ms: None,
            fade_out_ms: None,
        };
        let chain = background_music_chain(&bg, 3, 10_000);
        assert!(chain.starts_with("[3:a]"));
        assert!(chain.contains("atrim=duration=8.000"));
        assert!(chain.contains("volume=0.500"));
        assert!(chain.contains("adelay=2000|2000"));
        assert!(chain.ends_with("[bgm]"));
    }
}
