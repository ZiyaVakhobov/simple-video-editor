use super::TextOverlay;

/// Escapes text for use inside a single-quoted drawtext value.
/// Backslashes are doubled and single quotes use the close-escape-reopen
/// pattern required by FFmpeg filter graph quoting.
pub fn escape_drawtext(text: &str) -> String {
    text.replace('\\', "\\\\").replace('\'', "'\\''")
}

/// Sanitizes a color value to FFmpeg-safe characters (named colors,
/// #RRGGBB, or 0xRRGGBB). Falls back to white when empty.
fn sanitize_color(color: &str) -> String {
    let cleaned: String = color
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '#')
        .collect();
    if cleaned.is_empty() {
        "white".to_string()
    } else {
        cleaned
    }
}

fn position_expr(overlay: &TextOverlay) -> (String, String) {
    const MARGIN: &str = "20";
    match overlay.position.as_str() {
        "top-left" => (MARGIN.into(), MARGIN.into()),
        "top-center" => ("(w-text_w)/2".into(), MARGIN.into()),
        "top-right" => (format!("w-text_w-{MARGIN}"), MARGIN.into()),
        "bottom-left" => (MARGIN.into(), format!("h-text_h-{MARGIN}")),
        "bottom-center" => ("(w-text_w)/2".into(), format!("h-text_h-{MARGIN}")),
        "bottom-right" => (format!("w-text_w-{MARGIN}"), format!("h-text_h-{MARGIN}")),
        "custom" => (
            format!("{}", overlay.x.unwrap_or(0.0)),
            format!("{}", overlay.y.unwrap_or(0.0)),
        ),
        // "center" and anything unknown
        _ => ("(w-text_w)/2".into(), "(h-text_h)/2".into()),
    }
}

/// Builds one drawtext filter for a text overlay, time-gated with enable=between().
pub fn drawtext_filter(overlay: &TextOverlay) -> String {
    let (x, y) = position_expr(overlay);
    let start = overlay.start_ms as f64 / 1000.0;
    let end = overlay.end_ms as f64 / 1000.0;
    let opacity = overlay.opacity.unwrap_or(1.0).clamp(0.0, 1.0);

    let mut filter = format!(
        "drawtext=text='{}':fontsize={}:fontcolor={}@{:.2}:x={}:y={}:enable='between(t,{:.3},{:.3})'",
        escape_drawtext(&overlay.text),
        overlay.font_size,
        sanitize_color(&overlay.color),
        opacity,
        x,
        y,
        start,
        end
    );

    if let Some(bg) = &overlay.background_color {
        filter.push_str(&format!(
            ":box=1:boxcolor={}@{:.2}:boxborderw=8",
            sanitize_color(bg),
            opacity
        ));
    }

    filter
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overlay(text: &str, position: &str) -> TextOverlay {
        TextOverlay {
            id: "t1".into(),
            text: text.into(),
            start_ms: 1_000,
            end_ms: 3_500,
            position: position.into(),
            x: None,
            y: None,
            font_size: 32,
            color: "#ffffff".into(),
            background_color: None,
            opacity: None,
        }
    }

    #[test]
    fn escapes_quotes() {
        assert_eq!(escape_drawtext("it's"), "it'\\''s");
        assert_eq!(escape_drawtext("a\\b"), "a\\\\b");
    }

    #[test]
    fn builds_gated_filter() {
        let f = drawtext_filter(&overlay("Hello", "bottom-center"));
        assert!(f.contains("text='Hello'"));
        assert!(f.contains("between(t,1.000,3.500)"));
        assert!(f.contains("x=(w-text_w)/2"));
    }
}
