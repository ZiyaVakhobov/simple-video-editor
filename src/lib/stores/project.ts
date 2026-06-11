import { derived, get, writable } from "svelte/store";
import type {
  BackgroundAudio,
  ExportSettings,
  SourceVideo,
  TextOverlay,
  TimelineClip,
  VideoProject
} from "$lib/types/project";

function createId(): string {
  return crypto.randomUUID();
}

function defaultExportSettings(): ExportSettings {
  return {
    format: "mp4",
    resolution: "original",
    optimizeForWeb: true,
    quality: "balanced",
    originalVolume: 1,
    masterVolume: 1,
    hlsVariants: ["1080p", "720p", "480p", "360p"]
  };
}

function emptyProject(): VideoProject {
  return {
    id: createId(),
    name: "Untitled Project",
    sourceVideos: [],
    clips: [],
    textOverlays: [],
    exportSettings: defaultExportSettings()
  };
}

export const project = writable<VideoProject>(emptyProject());

/** Source video currently shown in the preview player. */
export const activeSourceId = writable<string | null>(null);

// ---------------------------------------------------------------------------
// History (undo / redo)
// ---------------------------------------------------------------------------

export interface HistoryEntry {
  label: string;
  state: VideoProject;
  at: number;
}

const MAX_HISTORY = 100;
/** Snapshots taken AFTER each action; index 0 is the initial project. */
export const historyEntries = writable<HistoryEntry[]>([
  { label: "Project created", state: structuredClone(get(project)), at: Date.now() }
]);
/** Index of the entry matching the current project state. */
export const historyCursor = writable(0);

export const canUndo = derived(historyCursor, (c) => c > 0);
export const canRedo = derived(
  [historyEntries, historyCursor],
  ([entries, cursor]) => cursor < entries.length - 1
);

let restoring = false;
let lastLabel = "";
let lastAt = 0;
const COALESCE_MS = 800;

/** Records the current project state as a history entry. Rapid repeats of
 *  the same action (slider drags, trim drags) collapse into one entry. */
function record(label: string): void {
  if (restoring) return;
  const now = Date.now();
  const cursor = get(historyCursor);
  let entries = get(historyEntries).slice(0, cursor + 1);
  const snapshot = structuredClone(get(project));

  if (label === lastLabel && now - lastAt < COALESCE_MS && entries.length > 1) {
    entries[entries.length - 1] = { label, state: snapshot, at: now };
  } else {
    entries.push({ label, state: snapshot, at: now });
    if (entries.length > MAX_HISTORY) entries = entries.slice(entries.length - MAX_HISTORY);
  }

  lastLabel = label;
  lastAt = now;
  historyEntries.set(entries);
  historyCursor.set(entries.length - 1);
}

/** Restores the project to a specific history entry (panel click). */
export function jumpToHistory(index: number): void {
  const entries = get(historyEntries);
  if (index < 0 || index >= entries.length || index === get(historyCursor)) return;
  restoring = true;
  project.set(structuredClone(entries[index].state));
  restoring = false;
  historyCursor.set(index);
  lastLabel = "";
}

export function undo(): void {
  jumpToHistory(get(historyCursor) - 1);
}

export function redo(): void {
  jumpToHistory(get(historyCursor) + 1);
}

export const totalDurationMs = derived(project, (p) =>
  p.clips.reduce((sum, c) => sum + Math.max(0, c.endMs - c.startMs), 0)
);

export interface TimelineSegment {
  clip: TimelineClip;
  source: SourceVideo;
  /** Where this clip starts in timeline time. */
  offsetMs: number;
  durationMs: number;
}

/** Ordered clips resolved to sources with cumulative timeline offsets. */
export const timelineSegments = derived(project, (p) => {
  const ordered = [...p.clips].sort((a, b) => a.order - b.order);
  const segments: TimelineSegment[] = [];
  let offset = 0;
  for (const clip of ordered) {
    const source = p.sourceVideos.find((s) => s.id === clip.sourceVideoId);
    if (!source) continue;
    const durationMs = Math.max(0, clip.endMs - clip.startMs);
    segments.push({ clip, source, offsetMs: offset, durationMs });
    offset += durationMs;
  }
  return segments;
});

export function addSourceVideo(source: Omit<SourceVideo, "id">): SourceVideo {
  const withId: SourceVideo = { ...source, id: createId() };
  project.update((p) => ({ ...p, sourceVideos: [...p.sourceVideos, withId] }));
  activeSourceId.set(withId.id);
  record(`Import “${withId.fileName}”`);
  return withId;
}

export function addClipFromSource(sourceVideoId: string): void {
  project.update((p) => {
    const source = p.sourceVideos.find((s) => s.id === sourceVideoId);
    if (!source) return p;
    const clip: TimelineClip = {
      id: createId(),
      sourceVideoId,
      startMs: 0,
      endMs: source.durationMs,
      order: p.clips.length
    };
    return { ...p, clips: [...p.clips, clip] };
  });
  record("Add clip to timeline");
}

export function removeClip(clipId: string): void {
  project.update((p) => {
    const clips = p.clips
      .filter((c) => c.id !== clipId)
      .sort((a, b) => a.order - b.order)
      .map((c, i) => ({ ...c, order: i }));
    return { ...p, clips };
  });
  record("Delete clip");
}

export function moveClip(clipId: string, direction: -1 | 1): void {
  project.update((p) => {
    const clips = [...p.clips].sort((a, b) => a.order - b.order);
    const index = clips.findIndex((c) => c.id === clipId);
    const target = index + direction;
    if (index < 0 || target < 0 || target >= clips.length) return p;
    [clips[index], clips[target]] = [clips[target], clips[index]];
    return { ...p, clips: clips.map((c, i) => ({ ...c, order: i })) };
  });
  record("Reorder clip");
}

/** Moves a clip to a specific position in the timeline order. */
export function reorderClip(clipId: string, newIndex: number): void {
  project.update((p) => {
    const clips = [...p.clips].sort((a, b) => a.order - b.order);
    const from = clips.findIndex((c) => c.id === clipId);
    if (from < 0) return p;
    const target = Math.max(0, Math.min(newIndex, clips.length - 1));
    if (target === from) return p;
    const [moved] = clips.splice(from, 1);
    clips.splice(target, 0, moved);
    return { ...p, clips: clips.map((c, i) => ({ ...c, order: i })) };
  });
  record("Reorder clip");
}

const MIN_CLIP_MS = 100;

/** Splits the clip under the playhead into two clips (Premiere-style razor). */
export function splitClipAtPlayhead(playheadMs: number): boolean {
  let didSplit = false;
  project.update((p) => {
    const ordered = [...p.clips].sort((a, b) => a.order - b.order);
    let offset = 0;
    for (const clip of ordered) {
      const dur = clip.endMs - clip.startMs;
      const local = playheadMs - offset;
      if (local > MIN_CLIP_MS && local < dur - MIN_CLIP_MS) {
        const splitSourceMs = Math.round(clip.startMs + local);
        const first = { ...clip, endMs: splitSourceMs };
        const second = { ...clip, id: createId(), startMs: splitSourceMs };
        const clips = ordered
          .flatMap((c) => (c.id === clip.id ? [first, second] : [c]))
          .map((c, i) => ({ ...c, order: i }));
        didSplit = true;
        return { ...p, clips };
      }
      offset += dur;
    }
    return p;
  });
  if (didSplit) record("Split clip");
  return didSplit;
}

export function updateClipTimes(clipId: string, startMs: number, endMs: number): void {
  project.update((p) => ({
    ...p,
    clips: p.clips.map((c) => (c.id === clipId ? { ...c, startMs, endMs } : c))
  }));
  record("Trim clip");
}

export function addOverlay(overlay: Omit<TextOverlay, "id">): void {
  project.update((p) => ({
    ...p,
    textOverlays: [...p.textOverlays, { ...overlay, id: createId() }]
  }));
  record(`Add text “${overlay.text.slice(0, 24)}”`);
}

export function removeOverlay(overlayId: string): void {
  project.update((p) => ({
    ...p,
    textOverlays: p.textOverlays.filter((o) => o.id !== overlayId)
  }));
  record("Remove text");
}

export function setBackgroundAudio(audio: BackgroundAudio | undefined): void {
  project.update((p) => ({ ...p, backgroundAudio: audio }));
  record(audio ? "Add music" : "Remove music");
}

export function updateBackgroundAudio(patch: Partial<BackgroundAudio>): void {
  project.update((p) =>
    p.backgroundAudio ? { ...p, backgroundAudio: { ...p.backgroundAudio, ...patch } } : p
  );
  record("Adjust music");
}

export function updateExportSettings(patch: Partial<ExportSettings>): void {
  project.update((p) => ({ ...p, exportSettings: { ...p.exportSettings, ...patch } }));
  record("Change export settings");
}

export function setProjectName(name: string): void {
  project.update((p) => ({ ...p, name }));
  record("Rename project");
}
