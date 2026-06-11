import type { Milliseconds } from "$lib/types/project";

export function msToSeconds(ms: Milliseconds): number {
  return ms / 1000;
}

export function secondsToMs(seconds: number): Milliseconds {
  return Math.max(0, Math.round(seconds * 1000));
}

/** Formats milliseconds as m:ss or h:mm:ss for display. */
export function formatDuration(ms: Milliseconds): string {
  const totalSeconds = Math.floor(ms / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  if (hours > 0) {
    return `${hours}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
  }
  return `${minutes}:${String(seconds).padStart(2, "0")}`;
}

export function formatFileSize(bytes?: number): string {
  if (!bytes) return "";
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  return `${value.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
}
