<script lang="ts">
  import {
    activeSourceId,
    project,
    timelineSegments,
    totalDurationMs,
    type TimelineSegment
  } from "$lib/stores/project";
  import { isPlaying, playheadMs } from "$lib/stores/playback";
  import { formatDuration } from "$lib/utils/time";

  let videoEl = $state<HTMLVideoElement | null>(null);
  let audioEl = $state<HTMLAudioElement | null>(null);
  let wrapHeight = $state(0);
  let loadedSrc = "";
  let currentSegIndex = -1;
  let mediaError = $state("");

  const backgroundMusic = $derived($project.backgroundAudio ?? null);

  /** Overlays visible at the current playhead (approximate preview;
   *  FFmpeg renders the final text on export). */
  const activeOverlays = $derived(
    $project.textOverlays.filter((o) => o.startMs <= $playheadMs && $playheadMs < o.endMs)
  );

  function overlayStyle(position: string): string {
    switch (position) {
      case "top-left":
        return "top: 4%; left: 3%;";
      case "top-center":
        return "top: 4%; left: 50%; transform: translateX(-50%);";
      case "top-right":
        return "top: 4%; right: 3%;";
      case "bottom-left":
        return "bottom: 6%; left: 3%;";
      case "bottom-center":
        return "bottom: 6%; left: 50%; transform: translateX(-50%);";
      case "bottom-right":
        return "bottom: 6%; right: 3%;";
      default:
        return "top: 50%; left: 50%; transform: translate(-50%, -50%);";
    }
  }

  /** Scales overlay font from source-video pixels to preview pixels. */
  const fontScale = $derived.by(() => {
    const seg = segments[Math.max(0, segmentIndexAt($playheadMs))];
    if (!seg || wrapHeight === 0) return 1;
    return wrapHeight / Math.max(seg.source.height, 1);
  });

  // When the timeline is empty, preview the active source as one big segment.
  const segments = $derived.by<TimelineSegment[]>(() => {
    if ($timelineSegments.length > 0) return $timelineSegments;
    const source = $project.sourceVideos.find((s) => s.id === $activeSourceId);
    if (!source) return [];
    return [
      {
        clip: {
          id: "__preview__",
          sourceVideoId: source.id,
          startMs: 0,
          endMs: source.durationMs,
          order: 0
        },
        source,
        offsetMs: 0,
        durationMs: source.durationMs
      }
    ];
  });

  const totalMs = $derived(
    $timelineSegments.length > 0
      ? $totalDurationMs
      : (segments[0]?.durationMs ?? 0)
  );

  function segmentIndexAt(ms: number): number {
    for (let i = 0; i < segments.length; i++) {
      const seg = segments[i];
      if (ms < seg.offsetMs + seg.durationMs) return i;
    }
    return segments.length - 1;
  }

  // Sync the <video> element to the playhead (scrubs, clip jumps, segment edits).
  $effect(() => {
    const ms = $playheadMs;
    if (!videoEl || segments.length === 0) return;
    const idx = segmentIndexAt(ms);
    if (idx < 0) return;
    const seg = segments[idx];
    const sourceSec = (seg.clip.startMs + Math.max(0, ms - seg.offsetMs)) / 1000;
    const src = seg.source.mediaUrl ?? "";
    if (!src) return;

    if (src !== loadedSrc) {
      loadedSrc = src;
      currentSegIndex = idx;
      videoEl.src = src;
      videoEl.currentTime = sourceSec;
      if ($isPlaying) void videoEl.play().catch(() => isPlaying.set(false));
      return;
    }
    if (idx !== currentSegIndex) {
      currentSegIndex = idx;
      videoEl.currentTime = sourceSec;
      if ($isPlaying) void videoEl.play().catch(() => isPlaying.set(false));
      return;
    }
    // Scrub guard: only hard-seek when the element drifted from the playhead.
    if (Math.abs(videoEl.currentTime - sourceSec) > 0.35) {
      videoEl.currentTime = sourceSec;
    }
  });

  $effect(() => {
    if (!videoEl) return;
    if ($isPlaying) {
      void videoEl.play().catch(() => isPlaying.set(false));
    } else {
      videoEl.pause();
    }
  });

  // Preview volumes (export applies the exact FFmpeg gains; the <video>/<audio>
  // elements only support 0..1, so >100% is clamped in preview).
  $effect(() => {
    if (videoEl) {
      videoEl.volume = Math.min(1, Math.max(0, $project.exportSettings.originalVolume));
    }
  });
  $effect(() => {
    if (audioEl && backgroundMusic) {
      audioEl.volume = Math.min(1, Math.max(0, backgroundMusic.volume));
    }
  });

  // Keep background music in sync with timeline playback.
  $effect(() => {
    const bg = backgroundMusic;
    if (!audioEl || !bg?.mediaUrl) return;
    const offsetSec = ($playheadMs - bg.startMs) / 1000;
    const inRange =
      $playheadMs >= bg.startMs && (bg.endMs === undefined || $playheadMs < bg.endMs);

    if ($isPlaying && inRange && segments.length > 0) {
      if (!bg.loop && Math.abs(audioEl.currentTime - offsetSec) > 0.4) {
        audioEl.currentTime = Math.max(0, offsetSec);
      }
      if (audioEl.paused) void audioEl.play().catch(() => {});
    } else {
      audioEl.pause();
      if (!bg.loop && offsetSec >= 0) {
        audioEl.currentTime = Math.max(0, offsetSec);
      }
    }
  });

  function onMediaError(): void {
    const err = videoEl?.error;
    const codes: Record<number, string> = {
      1: "playback aborted",
      2: "network error while streaming",
      3: "decoding failed (codec issue)",
      4: "format not supported"
    };
    mediaError = err
      ? `Cannot play this video: ${codes[err.code] ?? "unknown error"} (code ${err.code})`
      : "";
    isPlaying.set(false);
  }

  function onTimeUpdate(): void {
    mediaError = "";
    if (!videoEl || currentSegIndex < 0 || currentSegIndex >= segments.length) return;
    const seg = segments[currentSegIndex];
    const tMs = videoEl.currentTime * 1000;

    if (tMs >= seg.clip.endMs - 45) {
      const next = segments[currentSegIndex + 1];
      if (next) {
        playheadMs.set(next.offsetMs + 1);
      } else {
        isPlaying.set(false);
        playheadMs.set(totalMs);
      }
      return;
    }
    if (tMs >= seg.clip.startMs) {
      playheadMs.set(seg.offsetMs + (tMs - seg.clip.startMs));
    }
  }

  function togglePlay(): void {
    if (segments.length === 0) return;
    if (!$isPlaying && $playheadMs >= totalMs - 50) playheadMs.set(0);
    isPlaying.set(!$isPlaying);
  }

  function stepFrame(direction: -1 | 1): void {
    isPlaying.set(false);
    const seg = segments[Math.max(0, segmentIndexAt($playheadMs))];
    const fps = seg?.source.fps ?? 30;
    const frameMs = 1000 / fps;
    playheadMs.set(Math.max(0, Math.min(totalMs, $playheadMs + direction * frameMs)));
  }

  const activeSource = $derived(segments[segmentIndexAt($playheadMs)]?.source ?? null);
</script>

<div class="preview">
  {#if segments.length > 0}
    <div class="video-wrap" bind:clientHeight={wrapHeight}>
      <!-- svelte-ignore a11y_media_has_caption -->
      <video
        bind:this={videoEl}
        ontimeupdate={onTimeUpdate}
        onended={() => isPlaying.set(false)}
        onerror={onMediaError}
        onclick={togglePlay}
      ></video>
      {#each activeOverlays as overlay (overlay.id)}
        <div
          class="text-overlay"
          style="{overlayStyle(overlay.position)} font-size: {Math.max(
            10,
            overlay.fontSize * fontScale
          )}px; color: {overlay.color}; opacity: {overlay.opacity ?? 1}; {overlay.backgroundColor
            ? `background: ${overlay.backgroundColor}; padding: 0.15em 0.4em; border-radius: 0.2em;`
            : ''}"
        >
          {overlay.text}
        </div>
      {/each}
    </div>
    {#if backgroundMusic?.mediaUrl}
      <audio
        bind:this={audioEl}
        src={backgroundMusic.mediaUrl}
        loop={backgroundMusic.loop}
        preload="auto"
      ></audio>
    {/if}
    {#if mediaError}
      <div class="media-error">{mediaError}</div>
    {/if}
    <div class="transport">
      <button class="ctrl" onclick={() => stepFrame(-1)} title="Previous frame">⏮</button>
      <button class="ctrl play" onclick={togglePlay} title="Play / Pause">
        {$isPlaying ? "⏸" : "▶"}
      </button>
      <button class="ctrl" onclick={() => stepFrame(1)} title="Next frame">⏭</button>
      <span class="time">
        {formatDuration($playheadMs)} / {formatDuration(totalMs)}
      </span>
      {#if activeSource}
        <span class="meta-inline">
          {activeSource.fileName} · {activeSource.width}×{activeSource.height}
          {#if activeSource.fps}· {activeSource.fps.toFixed(2)} fps{/if}
        </span>
      {/if}
    </div>
  {:else}
    <div class="empty">
      <p>No video loaded</p>
      <p class="hint">Use “Import Video” to get started.</p>
    </div>
  {/if}
</div>

<style>
  .preview {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #111;
    border-radius: 8px;
    overflow: hidden;
  }
  .video-wrap {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
  }
  video {
    flex: 1;
    width: 100%;
    min-height: 0;
    background: #000;
    object-fit: contain;
    cursor: pointer;
  }
  .text-overlay {
    position: absolute;
    pointer-events: none;
    white-space: pre-wrap;
    text-align: center;
    max-width: 90%;
    text-shadow: 0 1px 3px rgba(0, 0, 0, 0.6);
    line-height: 1.25;
    z-index: 3;
  }
  .media-error {
    background: #3a1717;
    color: #e09c9c;
    font-size: 12px;
    padding: 6px 12px;
  }
  .transport {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: #1a1a1a;
  }
  .ctrl {
    background: #2a2a2a;
    border: 1px solid #3a3a3a;
    border-radius: 6px;
    color: #ddd;
    width: 34px;
    height: 28px;
    font-size: 13px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .ctrl:hover {
    background: #333;
  }
  .ctrl.play {
    width: 44px;
    background: #2f6fdb;
    border-color: #2f6fdb;
    color: white;
  }
  .time {
    font-size: 12px;
    color: #ccc;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .meta-inline {
    margin-left: auto;
    font-size: 11px;
    color: #777;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: #666;
  }
  .empty p {
    margin: 4px 0;
  }
  .hint {
    font-size: 13px;
  }
</style>
