<script lang="ts">
  import type { RunLifecycleStatus } from "../lib/run";

  export let projectName: string;
  export let status: RunLifecycleStatus = "idle";
  export let disabled = false;

  const playPath = "M6 4l20 12L6 28z";
  const stopPath = "M8 8h16v16H8z";

  $: isStarting = status === "starting";
  $: isRunning = status === "running";
  $: label = isRunning ? "Stop" : "Run";
  $: ariaLabel = `${label} ${projectName}`;
</script>

<button
  type="button"
  class="run-button"
  class:running={isRunning}
  class:starting={isStarting}
  disabled={disabled}
  aria-label={ariaLabel}
  title={`${label} (⏎)`}
>
  {#if isStarting}
    <span class="spinner" aria-hidden="true"></span>
    <span>Starting…</span>
  {:else}
    <svg viewBox="0 0 32 32" aria-hidden="true">
      <path d={isRunning ? stopPath : playPath}></path>
    </svg>
    <span>{label}</span>
  {/if}
</button>
