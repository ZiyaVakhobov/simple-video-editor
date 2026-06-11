import { writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ExportProgressEvent } from "$lib/types/media";
import type { VideoProject } from "$lib/types/project";

export interface ExportState {
  status: "idle" | "running" | "complete" | "error" | "cancelled";
  percent: number;
  jobId?: string;
  outputPath?: string;
  message?: string;
}

export const exportState = writable<ExportState>({ status: "idle", percent: 0 });

let unlisten: UnlistenFn | null = null;

export async function initExportListener(): Promise<void> {
  if (unlisten) return;
  unlisten = await listen<ExportProgressEvent>("export://progress", (event) => {
    const p = event.payload;
    exportState.set({
      status: p.status,
      percent: p.percent,
      jobId: p.jobId,
      outputPath: p.outputPath,
      message: p.message
    });
  });
}

export async function startExport(project: VideoProject, outputPath: string): Promise<void> {
  await initExportListener();
  exportState.set({ status: "running", percent: 0, outputPath });
  try {
    const jobId = await invoke<string>("start_export", { project, outputPath });
    exportState.update((s) => ({ ...s, jobId }));
  } catch (error) {
    exportState.set({ status: "error", percent: 0, message: String(error) });
  }
}

export async function cancelExport(jobId: string): Promise<void> {
  try {
    await invoke("cancel_export", { jobId });
  } catch {
    // Job already finished; the completion event wins.
  }
}

export function resetExportState(): void {
  exportState.set({ status: "idle", percent: 0 });
}
