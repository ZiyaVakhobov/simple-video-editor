use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Child;
use std::sync::Mutex;

use super::errors::AppError;

/// Tracks running FFmpeg export jobs so they can be cancelled safely.
#[derive(Default)]
pub struct JobRegistry {
    jobs: Mutex<HashMap<String, Child>>,
    cancelled: Mutex<HashSet<String>>,
}

impl JobRegistry {
    pub fn insert(&self, job_id: &str, child: Child) {
        self.jobs.lock().unwrap().insert(job_id.to_string(), child);
    }

    /// Removes the job and returns the child process so the worker can wait on it.
    pub fn take(&self, job_id: &str) -> Option<Child> {
        self.jobs.lock().unwrap().remove(job_id)
    }

    pub fn cancel(&self, job_id: &str) -> Result<(), AppError> {
        self.cancelled.lock().unwrap().insert(job_id.to_string());
        let mut jobs = self.jobs.lock().unwrap();
        match jobs.get_mut(job_id) {
            Some(child) => {
                let _ = child.kill();
                Ok(())
            }
            None => Err(AppError::JobNotFound(job_id.to_string())),
        }
    }

    pub fn was_cancelled(&self, job_id: &str) -> bool {
        self.cancelled.lock().unwrap().contains(job_id)
    }

    pub fn clear_cancelled(&self, job_id: &str) {
        self.cancelled.lock().unwrap().remove(job_id);
    }
}

/// Locates a media binary. Prefers a sidecar binary shipped next to the app
/// executable (plain name in bundles, triple-suffixed in dev builds), then
/// falls back to the system PATH.
fn find_binary(name: &str) -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for base in [dir.to_path_buf(), dir.join("binaries")] {
                for candidate in [base.join(name), base.join(format!("{name}.exe"))] {
                    if candidate.is_file() {
                        return Some(candidate);
                    }
                }
                // Dev builds keep the target triple: ffmpeg-x86_64-unknown-linux-gnu
                if let Ok(entries) = std::fs::read_dir(&base) {
                    let prefix = format!("{name}-");
                    for entry in entries.flatten() {
                        let file_name = entry.file_name();
                        let file_name = file_name.to_string_lossy();
                        if file_name.starts_with(&prefix) && entry.path().is_file() {
                            return Some(entry.path());
                        }
                    }
                }
            }
        }
    }
    which(name)
}

fn which(name: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths)
        .map(|dir| dir.join(name))
        .find(|p| p.is_file())
}

pub fn ffmpeg_path() -> Result<PathBuf, AppError> {
    find_binary("ffmpeg").ok_or(AppError::FfmpegNotFound)
}

pub fn ffprobe_path() -> Result<PathBuf, AppError> {
    find_binary("ffprobe").ok_or(AppError::FfprobeNotFound)
}

/// Formats milliseconds as an FFmpeg timestamp: H:MM:SS.mmm
pub fn ms_to_timestamp(ms: u64) -> String {
    let hours = ms / 3_600_000;
    let minutes = (ms % 3_600_000) / 60_000;
    let seconds = (ms % 60_000) / 1_000;
    let millis = ms % 1_000;
    format!("{hours}:{minutes:02}:{seconds:02}.{millis:03}")
}

/// Parses one line of `-progress pipe:1` output. Returns processed output
/// time in milliseconds when the line carries a time value.
pub fn parse_progress_line(line: &str) -> Option<u64> {
    let (key, value) = line.split_once('=')?;
    match key.trim() {
        // Despite the name, out_time_ms is in microseconds (FFmpeg quirk).
        "out_time_ms" | "out_time_us" => {
            let micros: i64 = value.trim().parse().ok()?;
            Some((micros.max(0) as u64) / 1_000)
        }
        "out_time" => parse_timestamp_ms(value.trim()),
        _ => None,
    }
}

fn parse_timestamp_ms(ts: &str) -> Option<u64> {
    // Format: HH:MM:SS.micros
    let mut parts = ts.split(':');
    let hours: u64 = parts.next()?.parse().ok()?;
    let minutes: u64 = parts.next()?.parse().ok()?;
    let seconds: f64 = parts.next()?.parse().ok()?;
    Some(hours * 3_600_000 + minutes * 60_000 + (seconds * 1000.0) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_formatting() {
        assert_eq!(ms_to_timestamp(0), "0:00:00.000");
        assert_eq!(ms_to_timestamp(5_000), "0:00:05.000");
        assert_eq!(ms_to_timestamp(20_800), "0:00:20.800");
        assert_eq!(ms_to_timestamp(3_661_500), "1:01:01.500");
    }

    #[test]
    fn progress_parsing() {
        assert_eq!(parse_progress_line("out_time_ms=5000000"), Some(5_000));
        assert_eq!(parse_progress_line("out_time_us=1500000"), Some(1_500));
        assert_eq!(parse_progress_line("out_time=00:00:05.000000"), Some(5_000));
        assert_eq!(parse_progress_line("frame=120"), None);
        assert_eq!(parse_progress_line("progress=end"), None);
    }
}
