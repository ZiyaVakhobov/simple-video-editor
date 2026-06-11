use std::collections::HashSet;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use super::errors::AppError;

/// Minimal localhost HTTP server with Range support.
///
/// WebKitGTK's media backend (GStreamer) fetches URLs itself and cannot use
/// Tauri's custom asset:// protocol, so <video> playback needs a real HTTP
/// origin. Only explicitly registered files are served; everything else is 403.
pub struct MediaServer {
    port: u16,
    allowed: Arc<Mutex<HashSet<PathBuf>>>,
}

impl MediaServer {
    pub fn start() -> std::io::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        let allowed: Arc<Mutex<HashSet<PathBuf>>> = Arc::new(Mutex::new(HashSet::new()));

        let allowed_for_loop = allowed.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let allowed = allowed_for_loop.clone();
                std::thread::spawn(move || {
                    let _ = handle_connection(stream, allowed);
                });
            }
        });

        Ok(Self { port, allowed })
    }

    /// Registers a file for serving and returns its playback URL.
    pub fn register(&self, path: &str) -> Result<String, AppError> {
        let canonical = std::fs::canonicalize(path)
            .map_err(|_| AppError::InvalidInput(format!("File not found: {path}")))?;
        if !canonical.is_file() {
            return Err(AppError::InvalidInput(format!("Not a file: {path}")));
        }
        self.allowed.lock().unwrap().insert(canonical.clone());
        Ok(format!(
            "http://127.0.0.1:{}/media?path={}",
            self.port,
            percent_encode(&canonical.to_string_lossy())
        ))
    }
}

fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 3);
    for byte in input.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(*byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() + 1 && i + 2 <= bytes.len() - 1 + 1 {
            if let (Some(h), Some(l)) = (
                bytes.get(i + 1).and_then(|b| (*b as char).to_digit(16)),
                bytes.get(i + 2).and_then(|b| (*b as char).to_digit(16)),
            ) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn content_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "mp4" | "m4v" => "video/mp4",
        "mov" => "video/quicktime",
        "mkv" => "video/x-matroska",
        "webm" => "video/webm",
        "avi" => "video/x-msvideo",
        "ts" | "mts" => "video/mp2t",
        "wmv" => "video/x-ms-wmv",
        "flv" => "video/x-flv",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "m4a" | "aac" => "audio/mp4",
        "ogg" => "audio/ogg",
        "flac" => "audio/flac",
        _ => "application/octet-stream",
    }
}

fn handle_connection(
    mut stream: TcpStream,
    allowed: Arc<Mutex<HashSet<PathBuf>>>,
) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    let mut range_header: Option<String> = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Range:").or_else(|| trimmed.strip_prefix("range:")) {
            range_header = Some(value.trim().to_string());
        }
    }

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("");

    if method != "GET" && method != "HEAD" {
        return respond_status(&mut stream, 405, "Method Not Allowed");
    }

    let path_param = target
        .split_once("path=")
        .map(|(_, v)| v.split('&').next().unwrap_or(v))
        .map(percent_decode);

    let Some(file_path) = path_param else {
        return respond_status(&mut stream, 400, "Bad Request");
    };

    let canonical = match std::fs::canonicalize(&file_path) {
        Ok(p) => p,
        Err(_) => return respond_status(&mut stream, 404, "Not Found"),
    };

    if !allowed.lock().unwrap().contains(&canonical) {
        return respond_status(&mut stream, 403, "Forbidden");
    }

    let mut file = match std::fs::File::open(&canonical) {
        Ok(f) => f,
        Err(_) => return respond_status(&mut stream, 404, "Not Found"),
    };
    let file_len = file.metadata()?.len();
    let ctype = content_type(&canonical);

    let range = range_header.as_deref().and_then(|r| parse_range(r, file_len));

    match range {
        Some((start, end)) => {
            let len = end - start + 1;
            let header = format!(
                "HTTP/1.1 206 Partial Content\r\nContent-Type: {ctype}\r\nAccept-Ranges: bytes\r\nContent-Range: bytes {start}-{end}/{file_len}\r\nContent-Length: {len}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n"
            );
            stream.write_all(header.as_bytes())?;
            if method == "GET" {
                file.seek(SeekFrom::Start(start))?;
                copy_limited(&mut file, &mut stream, len)?;
            }
        }
        None => {
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {ctype}\r\nAccept-Ranges: bytes\r\nContent-Length: {file_len}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n"
            );
            stream.write_all(header.as_bytes())?;
            if method == "GET" {
                copy_limited(&mut file, &mut stream, file_len)?;
            }
        }
    }
    Ok(())
}

fn parse_range(value: &str, file_len: u64) -> Option<(u64, u64)> {
    if file_len == 0 {
        return None;
    }
    let spec = value.strip_prefix("bytes=")?.split(',').next()?.trim();
    let (start_s, end_s) = spec.split_once('-')?;
    if start_s.is_empty() {
        // suffix range: last N bytes
        let suffix: u64 = end_s.parse().ok()?;
        let start = file_len.saturating_sub(suffix);
        return Some((start, file_len - 1));
    }
    let start: u64 = start_s.parse().ok()?;
    if start >= file_len {
        return None;
    }
    let end = if end_s.is_empty() {
        file_len - 1
    } else {
        end_s.parse::<u64>().ok()?.min(file_len - 1)
    };
    if end < start {
        return None;
    }
    Some((start, end))
}

fn copy_limited<R: Read, W: Write>(reader: &mut R, writer: &mut W, mut remaining: u64) -> std::io::Result<()> {
    let mut buf = [0u8; 64 * 1024];
    while remaining > 0 {
        let to_read = buf.len().min(remaining as usize);
        let n = reader.read(&mut buf[..to_read])?;
        if n == 0 {
            break;
        }
        writer.write_all(&buf[..n])?;
        remaining -= n as u64;
    }
    Ok(())
}

fn respond_status(stream: &mut TcpStream, code: u16, text: &str) -> std::io::Result<()> {
    let body = text.as_bytes();
    let header = format!(
        "HTTP/1.1 {code} {text}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_parsing() {
        assert_eq!(parse_range("bytes=0-499", 1000), Some((0, 499)));
        assert_eq!(parse_range("bytes=500-", 1000), Some((500, 999)));
        assert_eq!(parse_range("bytes=-200", 1000), Some((800, 999)));
        assert_eq!(parse_range("bytes=0-9999", 1000), Some((0, 999)));
        assert_eq!(parse_range("bytes=1000-", 1000), None);
    }

    #[test]
    fn percent_roundtrip() {
        let path = "/home/user/My Videos/clip (1).mp4";
        assert_eq!(percent_decode(&percent_encode(path)), path);
    }
}
