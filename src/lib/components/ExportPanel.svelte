<script lang="ts">
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { project, updateExportSettings } from "$lib/stores/project";
  import { cancelExport, exportState, resetExportState, startExport } from "$lib/stores/export";
  import type { ExportFormat, QualityPreset, ResolutionPreset } from "$lib/types/project";

  const FORMATS: { value: ExportFormat; label: string }[] = [
    { value: "mp4", label: "MP4 video" },
    { value: "mp3", label: "MP3 audio" },
    { value: "wav", label: "WAV audio" },
    { value: "hls", label: "HLS stream (folder)" },
    { value: "dash", label: "DASH stream (folder)" }
  ];

  const RESOLUTIONS: ResolutionPreset[] = [
    "original",
    "2160p",
    "1440p",
    "1080p",
    "720p",
    "480p",
    "360p"
  ];

  const QUALITIES: { value: QualityPreset; label: string }[] = [
    { value: "small", label: "Small file" },
    { value: "balanced", label: "Balanced" },
    { value: "high", label: "High quality" }
  ];

  const HLS_VARIANTS = ["1080p", "720p", "480p", "360p"];

  const settings = $derived($project.exportSettings);
  const isVideoFormat = $derived(["mp4", "dash"].includes(settings.format));
  const running = $derived($exportState.status === "running");
  let errorMessage = $state("");

  function toggleVariant(variant: string, checked: boolean): void {
    const current = settings.hlsVariants ?? [...HLS_VARIANTS];
    const next = checked
      ? [...new Set([...current, variant])]
      : current.filter((v) => v !== variant);
    updateExportSettings({ hlsVariants: next });
  }

  async function pickOutputAndExport(): Promise<void> {
    errorMessage = "";
    if ($project.clips.length === 0) {
      errorMessage = "Add at least one clip to the timeline first.";
      return;
    }
    if (settings.format === "hls" && (settings.hlsVariants ?? []).length === 0) {
      errorMessage = "Select at least one HLS resolution.";
      return;
    }

    let outputPath: string | null = null;
    const format = settings.format;

    if (format === "hls" || format === "dash") {
      outputPath = await open({ directory: true, title: "Output Folder" });
    } else {
      outputPath = await save({
        title: "Export",
        defaultPath: `${$project.name}.${format}`,
        filters: [{ name: format.toUpperCase(), extensions: [format] }]
      });
    }

    if (!outputPath) return;
    await startExport($project, outputPath);
  }

  async function onCancel(): Promise<void> {
    if ($exportState.jobId) await cancelExport($exportState.jobId);
  }
</script>

<div class="panel">
  <h3>Export</h3>

  <label>
    Format
    <select
      value={settings.format}
      onchange={(e) => updateExportSettings({ format: e.currentTarget.value as ExportFormat })}
    >
      {#each FORMATS as f (f.value)}
        <option value={f.value}>{f.label}</option>
      {/each}
    </select>
  </label>

  {#if isVideoFormat}
    <label>
      Resolution
      <select
        value={settings.resolution}
        onchange={(e) =>
          updateExportSettings({ resolution: e.currentTarget.value as ResolutionPreset })}
      >
        {#each RESOLUTIONS as r (r)}
          <option value={r}>{r === "original" ? "Original" : r}</option>
        {/each}
      </select>
    </label>
  {/if}

  {#if settings.format === "hls"}
    <fieldset class="variants">
      <legend>Resolutions in master playlist</legend>
      {#each HLS_VARIANTS as variant (variant)}
        <label class="check">
          <input
            type="checkbox"
            checked={(settings.hlsVariants ?? HLS_VARIANTS).includes(variant)}
            onchange={(e) => toggleVariant(variant, e.currentTarget.checked)}
          />
          {variant}
        </label>
      {/each}
      <p class="note">
        Each resolution becomes its own folder; <code>master.m3u8</code> links them all for
        adaptive streaming.
      </p>
    </fieldset>
  {/if}

  <label>
    Quality
    <select
      value={settings.quality}
      onchange={(e) => updateExportSettings({ quality: e.currentTarget.value as QualityPreset })}
    >
      {#each QUALITIES as q (q.value)}
        <option value={q.value}>{q.label}</option>
      {/each}
    </select>
  </label>

  {#if settings.format === "mp4"}
    <label class="check">
      <input
        type="checkbox"
        checked={settings.optimizeForWeb}
        onchange={(e) => updateExportSettings({ optimizeForWeb: e.currentTarget.checked })}
      />
      Optimize for Web
    </label>
  {/if}

  <label>
    Master Volume: {Math.round(settings.masterVolume * 100)}%
    <input
      type="range"
      min="0"
      max="2"
      step="0.05"
      value={settings.masterVolume}
      oninput={(e) => updateExportSettings({ masterVolume: Number(e.currentTarget.value) })}
    />
  </label>

  {#if running}
    <div class="progress">
      <div class="bar">
        <div class="fill" style="width: {$exportState.percent}%"></div>
      </div>
      <span>{$exportState.percent.toFixed(1)}%</span>
    </div>
    <button class="danger" onclick={onCancel}>Cancel Export</button>
  {:else}
    <button class="primary" onclick={pickOutputAndExport}>Export</button>
  {/if}

  {#if errorMessage}
    <p class="error">{errorMessage}</p>
  {/if}

  {#if $exportState.status === "complete"}
    <p class="success">
      Export finished:<br />
      <span class="path">{$exportState.outputPath}</span>
    </p>
    <button class="small" onclick={resetExportState}>Dismiss</button>
  {:else if $exportState.status === "error"}
    <p class="error">{$exportState.message ?? "FFmpeg export failed."}</p>
    <button class="small" onclick={resetExportState}>Dismiss</button>
  {:else if $exportState.status === "cancelled"}
    <p class="muted">Export cancelled.</p>
    <button class="small" onclick={resetExportState}>Dismiss</button>
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
  select {
    background: #2a2a2a;
    border: 1px solid #3a3a3a;
    border-radius: 4px;
    color: #eee;
    padding: 6px 8px;
    font-size: 13px;
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
  .primary:hover {
    background: #3d7ce4;
  }
  .danger {
    background: #8a2f2f;
    color: white;
  }
  .small {
    background: #2e2e2e;
    color: #ccc;
    padding: 4px 10px;
    font-size: 12px;
    align-self: flex-start;
  }
  .progress {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: #ccc;
  }
  .bar {
    flex: 1;
    height: 8px;
    background: #2a2a2a;
    border-radius: 4px;
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: #2f6fdb;
    transition: width 0.2s;
  }
  .error {
    color: #e07a7a;
    font-size: 12px;
    white-space: pre-wrap;
    margin: 0;
  }
  .success {
    color: #7ac88f;
    font-size: 12px;
    margin: 0;
    word-break: break-all;
  }
  .path {
    color: #aaa;
  }
  .muted {
    color: #888;
    font-size: 12px;
    margin: 0;
  }
  .variants {
    border: 1px solid #2c2c2c;
    border-radius: 6px;
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 0;
  }
  .variants legend {
    font-size: 11px;
    color: #888;
    padding: 0 4px;
  }
  .note {
    color: #666;
    font-size: 11px;
    margin: 2px 0 0;
  }
  code {
    background: #222;
    padding: 0 4px;
    border-radius: 3px;
  }
</style>
