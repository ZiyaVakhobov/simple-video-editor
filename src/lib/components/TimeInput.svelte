<script lang="ts">
  interface Props {
    /** Time in milliseconds; null = empty (when allowEmpty). */
    valueMs: number | null;
    allowEmpty?: boolean;
    placeholder?: string;
    onValueChange: (ms: number | null) => void;
  }

  let { valueMs, allowEmpty = false, placeholder = "0:00.0", onValueChange }: Props = $props();

  let focused = $state(false);
  let text = $state("");

  function formatMs(ms: number): string {
    const totalSec = ms / 1000;
    const hours = Math.floor(totalSec / 3600);
    const minutes = Math.floor((totalSec % 3600) / 60);
    const seconds = totalSec % 60;
    const secStr = seconds.toFixed(1).padStart(4, "0");
    return hours > 0
      ? `${hours}:${String(minutes).padStart(2, "0")}:${secStr}`
      : `${minutes}:${secStr}`;
  }

  // Keep the field synced with the value unless the user is typing.
  $effect(() => {
    if (!focused) text = valueMs === null ? "" : formatMs(valueMs);
  });

  /** Accepts "12", "12.5", "1:23", "1:23.4", "1:02:03.5". undefined = invalid. */
  function parseToMs(input: string): number | null | undefined {
    const t = input.trim().replace(",", ".");
    if (t === "") return allowEmpty ? null : undefined;
    if (!/^[\d:.]+$/.test(t)) return undefined;
    const parts = t.split(":");
    if (parts.length > 3) return undefined;
    let seconds = 0;
    for (const part of parts) {
      if (part === "" || Number.isNaN(Number(part))) return undefined;
      seconds = seconds * 60 + Number(part);
    }
    return Math.max(0, Math.round(seconds * 1000));
  }

  function commit(): void {
    const parsed = parseToMs(text);
    if (parsed === undefined) {
      text = valueMs === null ? "" : formatMs(valueMs);
      return;
    }
    onValueChange(parsed);
    if (parsed !== null) text = formatMs(parsed);
  }

  function onKeyDown(e: KeyboardEvent): void {
    if (e.key === "Enter") {
      (e.currentTarget as HTMLInputElement).blur();
    } else if (e.key === "ArrowUp" || e.key === "ArrowDown") {
      e.preventDefault();
      const base = valueMs ?? 0;
      const step = e.shiftKey ? 1000 : 100;
      const next = Math.max(0, base + (e.key === "ArrowUp" ? step : -step));
      onValueChange(next);
      text = formatMs(next);
    }
  }
</script>

<input
  class="time-input"
  type="text"
  inputmode="decimal"
  bind:value={text}
  {placeholder}
  title="Format m:ss.s — arrow keys ±0.1s, Shift+arrows ±1s"
  onfocus={() => (focused = true)}
  onblur={() => {
    focused = false;
    commit();
  }}
  onkeydown={onKeyDown}
/>

<style>
  .time-input {
    background: #2a2a2a;
    border: 1px solid #3a3a3a;
    border-radius: 4px;
    color: #eee;
    padding: 5px 8px;
    font-size: 12px;
    width: 84px;
    font-variant-numeric: tabular-nums;
    box-sizing: border-box;
  }
  .time-input:focus {
    border-color: #2f6fdb;
    outline: none;
  }
</style>
