<script lang="ts">
  import {
    activeSourceId,
    addClipFromSource,
    project,
    removeClip,
    reorderClip,
    splitClipAtPlayhead,
    timelineSegments,
    totalDurationMs,
    updateClipTimes
  } from "$lib/stores/project";
  import { isPlaying, playheadMs, selectedClipId } from "$lib/stores/playback";
  import { formatDuration } from "$lib/utils/time";
  import TimeInput from "$lib/components/TimeInput.svelte";

  let scrollEl = $state<HTMLDivElement | null>(null);
  let containerWidth = $state(800);
  let zoom = $state(1); // 1 = fit to width

  type Drag =
    | { type: "trim-l" | "trim-r"; clipId: string; startX: number; origStart: number; origEnd: number; sourceDurationMs: number }
    | { type: "move"; clipId: string }
    | { type: "scrub" };
  let drag = $state<Drag | null>(null);

  const MIN_CLIP_MS = 100;
  const totalMs = $derived(Math.max($totalDurationMs, 1));
  const fitPxPerMs = $derived(Math.max(containerWidth - 20, 100) / totalMs);
  const pxPerMs = $derived(fitPxPerMs * zoom);
  const contentWidth = $derived(Math.max(totalMs * pxPerMs, containerWidth - 20));

  const tickStepSec = $derived.by(() => {
    const steps = [0.1, 0.25, 0.5, 1, 2, 5, 10, 15, 30, 60, 120, 300, 600, 1800];
    const pxPerSec = pxPerMs * 1000;
    return steps.find((s) => s * pxPerSec >= 70) ?? 3600;
  });

  const ticks = $derived.by(() => {
    const result: { ms: number; label: string }[] = [];
    const stepMs = tickStepSec * 1000;
    for (let ms = 0; ms <= totalMs; ms += stepMs) {
      result.push({ ms, label: formatDuration(ms) });
    }
    return result;
  });

  function pointerToMs(e: MouseEvent): number {
    if (!scrollEl) return 0;
    const rect = scrollEl.getBoundingClientRect();
    const x = e.clientX - rect.left + scrollEl.scrollLeft;
    return Math.max(0, Math.min(totalMs, x / pxPerMs));
  }

  function startScrub(e: MouseEvent): void {
    isPlaying.set(false);
    playheadMs.set(pointerToMs(e));
    drag = { type: "scrub" };
  }

  function startTrim(e: MouseEvent, clipId: string, edge: "trim-l" | "trim-r"): void {
    e.stopPropagation();
    const seg = $timelineSegments.find((s) => s.clip.id === clipId);
    if (!seg) return;
    selectedClipId.set(clipId);
    drag = {
      type: edge,
      clipId,
      startX: e.clientX,
      origStart: seg.clip.startMs,
      origEnd: seg.clip.endMs,
      sourceDurationMs: seg.source.durationMs
    };
  }

  function startMove(e: MouseEvent, clipId: string): void {
    e.stopPropagation();
    selectedClipId.set(clipId);
    drag = { type: "move", clipId };
  }

  function onMouseMove(e: MouseEvent): void {
    if (!drag) return;
    if (drag.type === "scrub") {
      playheadMs.set(pointerToMs(e));
      return;
    }
    if (drag.type === "move") {
      const ms = pointerToMs(e);
      const segs = $timelineSegments;
      let target = segs.length - 1;
      for (let i = 0; i < segs.length; i++) {
        if (ms < segs[i].offsetMs + segs[i].durationMs / 2) {
          target = i;
          break;
        }
      }
      reorderClip(drag.clipId, target);
      return;
    }
    const deltaMs = (e.clientX - drag.startX) / pxPerMs;
    if (drag.type === "trim-l") {
      const startMs = Math.round(
        Math.max(0, Math.min(drag.origStart + deltaMs, drag.origEnd - MIN_CLIP_MS))
      );
      updateClipTimes(drag.clipId, startMs, drag.origEnd);
    } else {
      const endMs = Math.round(
        Math.min(drag.sourceDurationMs, Math.max(drag.origEnd + deltaMs, drag.origStart + MIN_CLIP_MS))
      );
      updateClipTimes(drag.clipId, drag.origStart, endMs);
    }
  }

  function onMouseUp(): void {
    drag = null;
  }

  function onSplit(): void {
    splitClipAtPlayhead($playheadMs);
  }

  function onDeleteSelected(): void {
    const id = $selectedClipId;
    if (id) {
      removeClip(id);
      selectedClipId.set(null);
    }
  }

  function onKeyDown(e: KeyboardEvent): void {
    const target = e.target as HTMLElement | null;
    if (target && ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName)) return;
    if (e.key === " ") {
      e.preventDefault();
      isPlaying.update((v) => !v);
    } else if (e.key === "s" || e.key === "S") {
      onSplit();
    } else if (e.key === "Delete" || e.key === "Backspace") {
      onDeleteSelected();
    }
  }

  const selectedSegment = $derived(
    $timelineSegments.find((s) => s.clip.id === $selectedClipId) ?? null
  );

  function onSelectedStartChange(ms: number | null): void {
    if (!selectedSegment || ms === null) return;
    const startMs = Math.min(ms, selectedSegment.clip.endMs - MIN_CLIP_MS);
    updateClipTimes(selectedSegment.clip.id, Math.max(0, startMs), selectedSegment.clip.endMs);
  }

  function onSelectedEndChange(ms: number | null): void {
    if (!selectedSegment || ms === null) return;
    const endMs = Math.max(
      Math.min(ms, selectedSegment.source.durationMs),
      selectedSegment.clip.startMs + MIN_CLIP_MS
    );
    updateClipTimes(selectedSegment.clip.id, selectedSegment.clip.startMs, endMs);
  }

  const CLIP_COLORS = ["#3a6ea5", "#5f9e6e", "#9e6e5f", "#7a5f9e", "#9e8f5f", "#5f8f9e"];
  function clipColor(index: number): string {
    return CLIP_COLORS[index % CLIP_COLORS.length];
  }
</script>

<svelte:window onmousemove={onMouseMove} onmouseup={onMouseUp} onkeydown={onKeyDown} />

<div class="timeline" bind:clientWidth={containerWidth}>
  <div class="toolbar">
    <div class="tools">
      <button class="tool" onclick={onSplit} title="Split at playhead (S)">✂ Split</button>
      <button
        class="tool danger"
        disabled={!$selectedClipId}
        onclick={onDeleteSelected}
        title="Delete selected clip (Del)"
      >
        ✕ Delete
      </button>
      <span class="sep"></span>
      <button class="tool" onclick={() => (zoom = Math.max(1, zoom / 1.5))} title="Zoom out">−</button>
      <button class="tool" onclick={() => (zoom = Math.min(40, zoom * 1.5))} title="Zoom in">+</button>
      <button class="tool" onclick={() => (zoom = 1)} title="Fit timeline">Fit</button>
    </div>
    <div class="info">
      <span class="time">{formatDuration($playheadMs)}</span>
      <span class="total">/ {formatDuration($totalDurationMs)}</span>
    </div>
  </div>

  {#if $project.sourceVideos.length > 0}
    <div class="sources">
      {#each $project.sourceVideos as source (source.id)}
        <div class="source" class:active={source.id === $activeSourceId}>
          <button class="link" onclick={() => activeSourceId.set(source.id)}>
            {source.fileName}
          </button>
          <button class="small" onclick={() => addClipFromSource(source.id)}>
            + Add to Timeline
          </button>
        </div>
      {/each}
    </div>
  {/if}

  {#if $timelineSegments.length === 0}
    <p class="empty">No clips yet. Import a video, then add it to the timeline.</p>
  {:else}
    <div class="scroll" bind:this={scrollEl}>
      <div class="content" style="width: {contentWidth}px">
        <!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
        <div class="ruler" onmousedown={startScrub}>
          {#each ticks as tick (tick.ms)}
            <div class="tick" style="left: {tick.ms * pxPerMs}px">
              <span>{tick.label}</span>
            </div>
          {/each}
        </div>

        <!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
        <div class="track" onmousedown={startScrub}>
          {#each $timelineSegments as seg, i (seg.clip.id)}
            <!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
            <div
              class="clip"
              class:selected={seg.clip.id === $selectedClipId}
              style="left: {seg.offsetMs * pxPerMs}px; width: {Math.max(
                seg.durationMs * pxPerMs,
                14
              )}px; background: {clipColor(i)}"
              onmousedown={(e) => startMove(e, seg.clip.id)}
            >
              <div class="handle left" onmousedown={(e) => startTrim(e, seg.clip.id, "trim-l")}></div>
              <span class="clip-label">{seg.source.fileName}</span>
              <span class="clip-dur">{formatDuration(seg.durationMs)}</span>
              <div class="handle right" onmousedown={(e) => startTrim(e, seg.clip.id, "trim-r")}></div>
            </div>
          {/each}
        </div>

        <div class="playhead" style="left: {$playheadMs * pxPerMs}px">
          <div class="playhead-cap"></div>
        </div>
      </div>
    </div>
  {/if}

  {#if selectedSegment}
    <div class="detail">
      <span class="detail-name">{selectedSegment.source.fileName}</span>
      <label>
        Start Time
        <TimeInput valueMs={selectedSegment.clip.startMs} onValueChange={onSelectedStartChange} />
      </label>
      <label>
        End Time
        <TimeInput valueMs={selectedSegment.clip.endMs} onValueChange={onSelectedEndChange} />
      </label>
      <span class="detail-dur">
        Duration: {formatDuration(selectedSegment.clip.endMs - selectedSegment.clip.startMs)}
      </span>
      <span class="hint">Drag clip edges to trim · drag clip to reorder · S splits · Space plays</span>
    </div>
  {/if}
</div>

<style>
  .timeline {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px;
    background: #181818;
    border-radius: 8px;
    user-select: none;
  }
  .toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .tools {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .tool {
    background: #2a2a2a;
    border: 1px solid #3a3a3a;
    border-radius: 5px;
    color: #ccc;
    padding: 4px 10px;
    font-size: 12px;
    cursor: pointer;
  }
  .tool:hover:not(:disabled) {
    background: #333;
  }
  .tool:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .tool.danger:hover:not(:disabled) {
    background: #5a2727;
  }
  .sep {
    width: 1px;
    height: 18px;
    background: #333;
    margin: 0 4px;
  }
  .info {
    font-size: 13px;
    font-variant-numeric: tabular-nums;
  }
  .time {
    color: #4a8af4;
    font-weight: 600;
  }
  .total {
    color: #777;
  }
  .sources {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .source {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 3px 8px;
    background: #222;
    border-radius: 6px;
    font-size: 12px;
  }
  .source.active {
    outline: 1px solid #4a8af4;
  }
  .link {
    background: none;
    border: none;
    color: #ccc;
    cursor: pointer;
    padding: 0;
    font-size: 12px;
    max-width: 180px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .small {
    background: #2e2e2e;
    border: 1px solid #3d3d3d;
    border-radius: 4px;
    color: #ccc;
    padding: 2px 8px;
    font-size: 11px;
    cursor: pointer;
  }
  .scroll {
    overflow-x: auto;
    overflow-y: hidden;
  }
  .content {
    position: relative;
    min-height: 96px;
  }
  .ruler {
    position: relative;
    height: 22px;
    border-bottom: 1px solid #2c2c2c;
    cursor: col-resize;
  }
  .tick {
    position: absolute;
    top: 0;
    height: 100%;
    border-left: 1px solid #3a3a3a;
    padding-left: 4px;
    font-size: 10px;
    color: #777;
    white-space: nowrap;
  }
  .track {
    position: relative;
    height: 64px;
    margin-top: 8px;
    background: repeating-linear-gradient(
      to right,
      #1d1d1d,
      #1d1d1d 8px,
      #1f1f1f 8px,
      #1f1f1f 16px
    );
    border-radius: 4px;
    cursor: col-resize;
  }
  .clip {
    position: absolute;
    top: 4px;
    bottom: 4px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 10px;
    overflow: hidden;
    cursor: grab;
    border: 1px solid rgba(255, 255, 255, 0.15);
    box-sizing: border-box;
  }
  .clip:active {
    cursor: grabbing;
  }
  .clip.selected {
    outline: 2px solid #fff;
    outline-offset: -1px;
  }
  .clip-label {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.92);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    pointer-events: none;
  }
  .clip-dur {
    font-size: 10px;
    color: rgba(255, 255, 255, 0.7);
    white-space: nowrap;
    pointer-events: none;
  }
  .handle {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 9px;
    cursor: ew-resize;
    background: rgba(255, 255, 255, 0.18);
  }
  .handle:hover {
    background: rgba(255, 255, 255, 0.4);
  }
  .handle.left {
    left: 0;
    border-radius: 4px 0 0 4px;
  }
  .handle.right {
    right: 0;
    border-radius: 0 4px 4px 0;
  }
  .playhead {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 1px;
    background: #f44;
    pointer-events: none;
    z-index: 5;
  }
  .playhead-cap {
    position: absolute;
    top: 0;
    left: -5px;
    width: 0;
    height: 0;
    border-left: 5px solid transparent;
    border-right: 5px solid transparent;
    border-top: 7px solid #f44;
  }
  .detail {
    display: flex;
    align-items: center;
    gap: 14px;
    font-size: 12px;
    color: #999;
    flex-wrap: wrap;
  }
  .detail-name {
    color: #ddd;
    font-weight: 600;
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .detail label {
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .detail-dur {
    color: #6a9;
  }
  .hint {
    color: #555;
    font-size: 11px;
    margin-left: auto;
  }
  .empty {
    color: #666;
    font-size: 13px;
    margin: 4px 0;
  }
</style>
