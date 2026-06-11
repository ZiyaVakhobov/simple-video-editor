<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import {
    project,
    setBackgroundAudio,
    totalDurationMs,
    updateBackgroundAudio,
    updateExportSettings
  } from "$lib/stores/project";
  import TimeInput from "$lib/components/TimeInput.svelte";
  import { formatDuration } from "$lib/utils/time";

  const settings = $derived($project.exportSettings);
  const music = $derived($project.backgroundAudio);

  function onStartChange(ms: number | null): void {
    if (!music || ms === null) return;
    const startMs =
      music.endMs !== undefined ? Math.min(ms, Math.max(0, music.endMs - 100)) : ms;
    updateBackgroundAudio({ startMs });
  }

  function onEndChange(ms: number | null): void {
    if (!music) return;
    if (ms === null || ms === 0) {
      updateBackgroundAudio({ endMs: undefined });
      return;
    }
    updateBackgroundAudio({ endMs: Math.max(ms, music.startMs + 100) });
  }

  function fileName(path: string): string {
    return path.split(/[\\/]/).pop() ?? path;
  }

  async function addMusic(): Promise<void> {
    const path = await open({
      title: "Add Music",
      filters: [{ name: "Audio", extensions: ["mp3", "wav", "m4a", "aac", "ogg", "flac"] }]
    });
    if (typeof path === "string") {
      let mediaUrl: string | undefined;
      try {
        mediaUrl = await invoke<string>("register_media", { path });
      } catch {
        mediaUrl = undefined;
      }
      setBackgroundAudio({ path, mediaUrl, startMs: 0, volume: 0.5, loop: false });
    }
  }
</script>

<div class="panel">
  <h3>Add Music</h3>

  <label>
    Original Volume: {Math.round(settings.originalVolume * 100)}%
    <input
      type="range"
      min="0"
      max="2"
      step="0.05"
      value={settings.originalVolume}
      oninput={(e) => updateExportSettings({ originalVolume: Number(e.currentTarget.value) })}
    />
  </label>

  {#if music}
    <div class="music">
      <span class="file" title={music.path}>{fileName(music.path)}</span>
      <button class="small danger" onclick={() => setBackgroundAudio(undefined)}>Remove</button>
    </div>

    <div class="row">
      <label>
        Start Time
        <TimeInput valueMs={music.startMs} onValueChange={onStartChange} />
      </label>
      <label>
        End Time
        <TimeInput
          valueMs={music.endMs ?? null}
          allowEmpty
          placeholder="video end"
          onValueChange={onEndChange}
        />
      </label>
    </div>
    <p class="hint">
      Timeline position where music starts and stops (m:ss.s). Leave End Time empty to play
      until the video ends. Timeline length: {formatDuration($totalDurationMs)}.
    </p>

    <label>
      Music Volume: {Math.round(music.volume * 100)}%
      <input
        type="range"
        min="0"
        max="2"
        step="0.05"
        value={music.volume}
        oninput={(e) => updateBackgroundAudio({ volume: Number(e.currentTarget.value) })}
      />
    </label>

    <label class="check">
      <input
        type="checkbox"
        checked={music.loop}
        onchange={(e) => updateBackgroundAudio({ loop: e.currentTarget.checked })}
      />
      Loop music
    </label>
  {:else}
    <button class="primary" onclick={addMusic}>Add Music</button>
    <p class="hint">Background music is mixed with the original audio on export.</p>
  {/if}
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
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    color: #999;
  }
  label.check {
    flex-direction: row;
    align-items: center;
    gap: 8px;
    color: #ccc;
  }
  .music {
    display: flex;
    align-items: center;
    gap: 8px;
    background: #222;
    border-radius: 4px;
    padding: 6px 10px;
    font-size: 12px;
  }
  .file {
    flex: 1;
    color: #ddd;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
    padding: 4px 10px;
    font-size: 12px;
  }
  .small.danger:hover {
    background: #5a2727;
  }
  .hint {
    color: #666;
    font-size: 11px;
    margin: 0;
  }
  .row {
    display: flex;
    gap: 10px;
  }
  .row label {
    flex: 1;
  }
</style>
