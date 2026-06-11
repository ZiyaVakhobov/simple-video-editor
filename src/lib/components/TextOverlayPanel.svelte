<script lang="ts">
  import { addOverlay, project, removeOverlay, totalDurationMs } from "$lib/stores/project";
  import { formatDuration } from "$lib/utils/time";
  import TimeInput from "$lib/components/TimeInput.svelte";
  import type { OverlayPosition } from "$lib/types/project";

  const POSITIONS: { value: OverlayPosition; label: string }[] = [
    { value: "top-left", label: "Top left" },
    { value: "top-center", label: "Top center" },
    { value: "top-right", label: "Top right" },
    { value: "center", label: "Center" },
    { value: "bottom-left", label: "Bottom left" },
    { value: "bottom-center", label: "Bottom center" },
    { value: "bottom-right", label: "Bottom right" }
  ];

  let text = $state("");
  let startMs = $state(0);
  let endMs = $state(5000);
  let position = $state<OverlayPosition>("bottom-center");
  let fontSize = $state(36);
  let color = $state("#ffffff");
  let useBackground = $state(false);
  let backgroundColor = $state("#000000");
  let errorMessage = $state("");

  function onAdd(): void {
    errorMessage = "";
    if (!text.trim()) {
      errorMessage = "Enter the overlay text.";
      return;
    }
    if (endMs <= startMs) {
      errorMessage = "End Time must be after Start Time.";
      return;
    }
    addOverlay({
      text: text.trim(),
      startMs,
      endMs,
      position,
      fontSize,
      color,
      backgroundColor: useBackground ? backgroundColor : undefined,
      opacity: 1
    });
    text = "";
  }
</script>

<div class="panel">
  <h3>Add Text</h3>

  {#if $project.textOverlays.length > 0}
    <div class="list">
      {#each $project.textOverlays as overlay (overlay.id)}
        <div class="item">
          <span class="item-text" title={overlay.text}>{overlay.text}</span>
          <span class="item-time">
            {formatDuration(overlay.startMs)}–{formatDuration(overlay.endMs)}
          </span>
          <button class="small danger" onclick={() => removeOverlay(overlay.id)}>✕</button>
        </div>
      {/each}
    </div>
  {/if}

  <label>
    Text
    <input type="text" bind:value={text} placeholder="Your text…" />
  </label>

  <div class="row">
    <label>
      Start Time
      <TimeInput valueMs={startMs} onValueChange={(ms) => (startMs = ms ?? 0)} />
    </label>
    <label>
      End Time
      <TimeInput valueMs={endMs} onValueChange={(ms) => (endMs = ms ?? 0)} />
    </label>
  </div>

  <label>
    Position
    <select bind:value={position}>
      {#each POSITIONS as p (p.value)}
        <option value={p.value}>{p.label}</option>
      {/each}
    </select>
  </label>

  <div class="row">
    <label>
      Font Size
      <input type="number" min="8" max="200" bind:value={fontSize} />
    </label>
    <label>
      Color
      <input type="color" bind:value={color} />
    </label>
  </div>

  <label class="check">
    <input type="checkbox" bind:checked={useBackground} />
    Background
    {#if useBackground}
      <input type="color" bind:value={backgroundColor} />
    {/if}
  </label>

  <button class="primary" onclick={onAdd}>Add Text</button>

  {#if errorMessage}
    <p class="error">{errorMessage}</p>
  {/if}

  <p class="hint">
    Timeline length: {formatDuration($totalDurationMs)}. Text is rendered by FFmpeg on export.
  </p>
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  h3 {
    margin: 0;
    font-size: 14px;
    color: #ddd;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 8px;
    background: #222;
    border-radius: 4px;
    padding: 4px 8px;
    font-size: 12px;
  }
  .item-text {
    flex: 1;
    color: #ddd;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .item-time {
    color: #888;
    white-space: nowrap;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    color: #999;
    flex: 1;
  }
  label.check {
    flex-direction: row;
    align-items: center;
    gap: 8px;
    color: #ccc;
  }
  .row {
    display: flex;
    gap: 10px;
  }
  input[type="text"],
  input[type="number"],
  select {
    background: #2a2a2a;
    border: 1px solid #3a3a3a;
    border-radius: 4px;
    color: #eee;
    padding: 6px 8px;
    font-size: 13px;
  }
  input[type="color"] {
    width: 40px;
    height: 28px;
    padding: 0;
    border: 1px solid #3a3a3a;
    border-radius: 4px;
    background: #2a2a2a;
  }
  button {
    border: none;
    border-radius: 6px;
    padding: 8px 12px;
    font-size: 13px;
    cursor: pointer;
  }
  .primary {
    background: #2f6fdb;
    color: white;
    font-weight: 600;
  }
  .small {
    background: #2e2e2e;
    color: #ccc;
    padding: 2px 8px;
    font-size: 11px;
  }
  .small.danger:hover {
    background: #5a2727;
  }
  .error {
    color: #e07a7a;
    font-size: 12px;
    margin: 0;
  }
  .hint {
    color: #666;
    font-size: 11px;
    margin: 0;
  }
</style>
