import { writable } from "svelte/store";
import type { Milliseconds } from "$lib/types/project";

/** Current playhead position in timeline time (ms). */
export const playheadMs = writable<Milliseconds>(0);

/** Whether timeline playback is running. */
export const isPlaying = writable(false);

/** Clip currently selected in the timeline (for split/delete/trim details). */
export const selectedClipId = writable<string | null>(null);
