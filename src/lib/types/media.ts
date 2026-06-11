import type { Milliseconds } from "./project";

export interface VideoMetadata {
  durationMs: Milliseconds;
  width: number;
  height: number;
  fps?: number;
  videoCodec?: string;
  audioCodec?: string;
  bitrate?: number;
  fileSizeBytes?: number;
  hasAudio: boolean;
}

export interface ExportProgressEvent {
  jobId: string;
  status: "running" | "complete" | "error" | "cancelled";
  percent: number;
  outTimeMs: Milliseconds;
  totalMs: Milliseconds;
  outputPath: string;
  message?: string;
}
