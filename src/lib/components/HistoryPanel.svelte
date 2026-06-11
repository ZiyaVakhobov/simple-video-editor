<script lang="ts">
  import {
    canRedo,
    canUndo,
    historyCursor,
    historyEntries,
    jumpToHistory,
    redo,
    undo
  } from "$lib/stores/project";

  function timeLabel(at: number): string {
    const d = new Date(at);
    return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}:${String(d.getSeconds()).padStart(2, "0")}`;
  }
</script>

<div class="panel">
  <div class="head">
    <h3>History</h3>
    <div class="btns">
      <button disabled={!$canUndo} onclick={undo} title="Undo (Ctrl+Z)">↶ Undo</button>
      <button disabled={!$canRedo} onclick={redo} title="Redo (Ctrl+Shift+Z)">↷ Redo</button>
    </div>
  </div>

  <div class="list">
    {#each [...$historyEntries].reverse() as entry, revIndex ($historyEntries.length - 1 - revIndex)}
      {@const index = $historyEntries.length - 1 - revIndex}
      <button
        class="entry"
        class:current={index === $historyCursor}
        class:future={index > $historyCursor}
        onclick={() => jumpToHistory(index)}
      >
        <span class="label">{entry.label}</span>
        <span class="time">{timeLabel(entry.at)}</span>
      </button>
    {/each}
  </div>

  <p class="hint">
    Click any step to go back (or forward) to that state. Ctrl+Z undoes, Ctrl+Shift+Z or Ctrl+Y
    redoes. Steps above the highlighted one are redo states.
  </p>
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  h3 {
    margin: 0;
    font-size: 14px;
    color: #ddd;
  }
  .btns {
    display: flex;
    gap: 6px;
  }
  .btns button {
    background: #2a2a2a;
    border: 1px solid #3a3a3a;
    border-radius: 5px;
    color: #ccc;
    padding: 4px 10px;
    font-size: 12px;
    cursor: pointer;
  }
  .btns button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: 340px;
    overflow-y: auto;
  }
  .entry {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    background: #202020;
    border: none;
    border-left: 3px solid transparent;
    border-radius: 4px;
    color: #bbb;
    padding: 6px 10px;
    font-size: 12px;
    cursor: pointer;
    text-align: left;
  }
  .entry:hover {
    background: #262626;
  }
  .entry.current {
    border-left-color: #2f6fdb;
    background: #24304a;
    color: #fff;
  }
  .entry.future {
    opacity: 0.45;
  }
  .label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .time {
    color: #777;
    font-variant-numeric: tabular-nums;
  }
  .hint {
    color: #666;
    font-size: 11px;
    margin: 0;
  }
</style>
