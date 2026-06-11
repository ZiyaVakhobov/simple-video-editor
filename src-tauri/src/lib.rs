mod media;

use std::sync::Arc;

use media::errors::AppError;
use media::ffmpeg::JobRegistry;
use media::metadata::VideoMetadata;
use media::VideoProject;
use tauri::State;

#[tauri::command]
async fn get_video_metadata(path: String) -> Result<VideoMetadata, AppError> {
    tauri::async_runtime::spawn_blocking(move || media::metadata::read_metadata(&path))
        .await
        .map_err(|e| AppError::Metadata(e.to_string()))?
}

#[tauri::command]
async fn start_export(
    app: tauri::AppHandle,
    registry: State<'_, Arc<JobRegistry>>,
    project: VideoProject,
    output_path: String,
) -> Result<String, AppError> {
    media::export::start_export(app, registry.inner().clone(), project, output_path)
}

#[tauri::command]
fn cancel_export(registry: State<'_, Arc<JobRegistry>>, job_id: String) -> Result<(), AppError> {
    registry.cancel(&job_id)
}

#[tauri::command]
fn check_ffmpeg() -> Result<bool, AppError> {
    Ok(media::ffmpeg::ffmpeg_path().is_ok() && media::ffmpeg::ffprobe_path().is_ok())
}

/// Registers a local media file with the preview HTTP server and returns
/// the URL the <video> element can stream from (WebKitGTK's GStreamer
/// backend cannot read Tauri's asset:// protocol).
#[tauri::command]
fn register_media(
    server: State<'_, media::server::MediaServer>,
    path: String,
) -> Result<String, AppError> {
    server.register(&path)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let media_server =
        media::server::MediaServer::start().expect("failed to start media preview server");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Arc::new(JobRegistry::default()))
        .manage(media_server)
        .invoke_handler(tauri::generate_handler![
            get_video_metadata,
            start_export,
            cancel_export,
            check_ffmpeg,
            register_media
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
