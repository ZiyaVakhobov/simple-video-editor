export type Milliseconds = number;

export type ExportFormat = "mp4" | "hls" | "dash" | "mp3" | "wav";

export type ResolutionPreset =
  | "original"
  | "2160p"
  | "1440p"
  | "1080p"
  | "720p"
  | "480p"
  | "360p"
  | "custom";

export type OverlayPosition =
  | "top-left"
  | "top-center"
  | "top-right"
  | "center"
  | "bottom-left"
  | "bottom-center"
  | "bottom-right"
  | "custom";

export type QualityPreset = "small" | "balanced" | "high";

export interface SourceVideo {
  id: string;
  path: string;
  fileName: string;
  durationMs: Milliseconds;
  width: number;
  height: number;
  fps?: number;
  videoCodec?: string;
  audioCodec?: string;
  fileSizeBytes?: number;
  /** Local streaming URL for the preview player (frontend only). */
  mediaUrl?: string;
}

export interface TimelineClip {
  id: string;
  sourceVideoId: string;
  startMs: Milliseconds;
  endMs: Milliseconds;
  order: number;
}

export interface TextOverlay {
  id: string;
  text: string;
  startMs: Milliseconds;
  endMs: Milliseconds;
  position: OverlayPosition;
  x?: number;
  y?: number;
  fontSize: number;
  color: string;
  backgroundColor?: string;
  opacity?: number;
}

export interface BackgroundAudio {
  path: string;
  startMs: Milliseconds;
  endMs?: Milliseconds;
  volume: number;
  loop: boolean;
  fadeInMs?: Milliseconds;
  fadeOutMs?: Milliseconds;
  /** Local streaming URL for the preview player (frontend only). */
  mediaUrl?: string;
}

export interface ExportSettings {
  format: ExportFormat;
  resolution: ResolutionPreset;
  customWidth?: number;
  customHeight?: number;
  optimizeForWeb: boolean;
  quality: QualityPreset;
  originalVolume: number;
  masterVolume: number;
  /** Renditions included in HLS export (master.m3u8 references all). */
  hlsVariants?: string[];
}

export interface VideoProject {
  id: string;
  name: string;
  sourceVideos: SourceVideo[];
  clips: TimelineClip[];
  textOverlays: TextOverlay[];
  backgroundAudio?: BackgroundAudio;
  exportSettings: ExportSettings;
}
