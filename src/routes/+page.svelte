<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";

  import VideoPreview from "$lib/components/VideoPreview.svelte";
  import Timeline from "$lib/components/Timeline.svelte";
  import ExportPanel from "$lib/components/ExportPanel.svelte";
  import TextOverlayPanel from "$lib/components/TextOverlayPanel.svelte";
  import AudioPanel from "$lib/components/AudioPanel.svelte";
  import HistoryPanel from "$lib/components/HistoryPanel.svelte";

  import {
    addClipFromSource,
    addSourceVideo,
    canRedo,
    canUndo,
    project,
    redo,
    setProjectName,
    undo
  } from "$lib/stores/project";
  import { initExportListener } from "$lib/stores/export";
  import type { VideoMetadata } from "$lib/types/media";

  type Tab = "export" | "text" | "music" | "history";
  let activeTab = $state<Tab>("export");

  function onGlobalKeyDown(e: KeyboardEvent): void {
    const target = e.target as HTMLElement | null;
    if (target && ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName)) return;
    const mod = e.ctrlKey || e.metaKey;
    if (!mod) return;
    const key = e.key.toLowerCase();
    if (key === "z" && !e.shiftKey) {
      e.preventDefault();
      undo();
    } else if (key === "y" || (key === "z" && e.shiftKey)) {
      e.preventDefault();
      redo();
    }
  }
  let importError = $state("");
  let ffmpegMissing = $state(false);

  onMount(async () => {
    await initExportListener();
    try {
      ffmpegMissing = !(await invoke<boolean>("check_ffmpeg"));
    } catch {
      ffmpegMissing = true;
    }
  });

  async function importVideo(): Promise<void> {
    importError = "";
    const path = await open({
      title: "Import Video",
      filters: [
        {
          name: "Video",
          extensions: ["mp4", "mov", "mkv", "webm", "avi", "m4v", "ts", "mts", "wmv", "flv"]
        }
      ]
    });
    if (typeof path !== "string") return;

    try {
      const meta = await invoke<VideoMetadata>("get_video_metadata", { path });
      const mediaUrl = await invoke<string>("register_media", { path });
      const source = addSourceVideo({
        path,
        mediaUrl,
        fileName: path.split(/[\\/]/).pop() ?? path,
        durationMs: meta.durationMs,
        width: meta.width,
        height: meta.height,
        fps: meta.fps,
        videoCodec: meta.videoCodec,
        audioCodec: meta.hasAudio ? (meta.audioCodec ?? "unknown") : undefined,
        fileSizeBytes: meta.fileSizeBytes
      });
      addClipFromSource(source.id);
    } catch (error) {
      importError = String(error);
    }
  }
</script>

<svelte:window onkeydown={onGlobalKeyDown} />

<main>
  <header>
    <input
      class="project-name"
      value={$project.name}
      onchange={(e) => setProjectName(e.currentTarget.value)}
    />
    <div class="header-actions">
      <button class="ghost" disabled={!$canUndo} onclick={undo} title="Undo (Ctrl+Z)">↶</button>
      <button class="ghost" disabled={!$canRedo} onclick={redo} title="Redo (Ctrl+Shift+Z)">↷</button>
      <button class="primary" onclick={importVideo}>Import Video</button>
    </div>
  </header>

  {#if ffmpegMissing}
    <div class="banner">
      FFmpeg was not found. Install FFmpeg (e.g. <code>sudo apt install ffmpeg</code>) and restart
      the app.
    </div>
  {/if}
  {#if importError}
    <div class="banner error">{importError}</div>
  {/if}

  <div class="content">
    <section class="preview-area">
      <VideoPreview />
    </section>

    <aside class="side">
      <nav class="tabs">
        <button class:active={activeTab === "export"} onclick={() => (activeTab = "export")}>
          Export
        </button>
        <button class:active={activeTab === "text"} onclick={() => (activeTab = "text")}>
          Add Text
        </button>
        <button class:active={activeTab === "music"} onclick={() => (activeTab = "music")}>
          Add Music
        </button>
        <button class:active={activeTab === "history"} onclick={() => (activeTab = "history")}>
          History
        </button>
      </nav>
      <div class="tab-body">
        {#if activeTab === "export"}
          <ExportPanel />
        {:else if activeTab === "text"}
          <TextOverlayPanel />
        {:else if activeTab === "music"}
          <AudioPanel />
        {:else}
          <HistoryPanel />
        {/if}
      </div>
    </aside>
  </div>

  <section class="timeline-area">
    <Timeline />
  </section>
</main>

<style>
  :global(html) {
    color-scheme: dark;
  }
  :global(body) {
    margin: 0;
    background: #101010;
    color: #ddd;
    font-family:
      system-ui,
      -apple-system,
      "Segoe UI",
      Roboto,
      sans-serif;
  }
  :global(select),
  :global(select option) {
    background-color: #2a2a2a;
    color: #eee;
  }
  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
    padding: 10px;
    box-sizing: border-box;
    gap: 10px;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .project-name {
    background: transparent;
    border: 1px solid transparent;
    border-radius: 6px;
    color: #eee;
    font-size: 16px;
    font-weight: 600;
    padding: 6px 8px;
    min-width: 220px;
  }
  .project-name:hover,
  .project-name:focus {
    border-color: #333;
    background: #1a1a1a;
    outline: none;
  }
  .header-actions {
    display: flex;
    gap: 8px;
  }
  button.primary {
    background: #2f6fdb;
    color: white;
    border: none;
    border-radius: 6px;
    padding: 8px 14px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }
  button.primary:hover {
    background: #3d7ce4;
  }
  button.ghost {
    background: #2a2a2a;
    border: 1px solid #3a3a3a;
    border-radius: 6px;
    color: #ccc;
    padding: 8px 12px;
    font-size: 13px;
    cursor: pointer;
  }
  button.ghost:hover:not(:disabled) {
    background: #333;
  }
  button.ghost:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .banner {
    background: #3a3214;
    color: #e8d48b;
    border-radius: 6px;
    padding: 8px 12px;
    font-size: 13px;
  }
  .banner.error {
    background: #3a1717;
    color: #e09c9c;
  }
  .content {
    display: flex;
    gap: 10px;
    flex: 1;
    min-height: 0;
  }
  .preview-area {
    flex: 1;
    min-width: 0;
  }
  .side {
    width: 300px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    background: #181818;
    border-radius: 8px;
    overflow: hidden;
  }
  .tabs {
    display: flex;
    border-bottom: 1px solid #262626;
  }
  .tabs button {
    flex: 1;
    background: none;
    border: none;
    color: #888;
    padding: 10px 0;
    font-size: 12px;
    cursor: pointer;
  }
  .tabs button.active {
    color: #fff;
    border-bottom: 2px solid #2f6fdb;
  }
  .tab-body {
    padding: 14px;
    overflow-y: auto;
  }
  .timeline-area {
    height: 250px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
  }
  .timeline-area :global(.timeline) {
    flex: 1;
  }
  code {
    background: #222;
    padding: 1px 5px;
    border-radius: 4px;
  }
</style>
