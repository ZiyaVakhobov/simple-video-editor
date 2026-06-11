use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("FFmpeg binary was not found. Please install FFmpeg or reinstall the application.")]
    FfmpegNotFound,

    #[error("FFprobe binary was not found. Please install FFmpeg or reinstall the application.")]
    FfprobeNotFound,

    #[error("Could not read video metadata: {0}")]
    Metadata(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("FFmpeg export failed: {0}")]
    ExportFailed(String),

    #[error("Export job not found: {0}")]
    JobNotFound(String),

    #[error("Output folder is not writable: {0}")]
    OutputNotWritable(String),

    #[error("File error: {0}")]
    Io(#[from] std::io::Error),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
