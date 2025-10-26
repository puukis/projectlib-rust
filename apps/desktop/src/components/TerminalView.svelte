<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { FitAddon } from "@xterm/addon-fit";
  import type { TerminalTab } from "../lib/terminal";
  import { terminalService } from "../lib/terminal";

  export let tab: TerminalTab;

  let container: HTMLDivElement;
  let fitAddon: FitAddon | null = null;
  let resizeObserver: ResizeObserver | null = null;

  function syncSize() {
    if (!fitAddon) {
      return;
    }

    fitAddon.fit();
    terminalService.resize(tab.id, tab.terminal.cols, tab.terminal.rows);
  }

  onMount(() => {
    fitAddon = new FitAddon();
    tab.terminal.loadAddon(fitAddon);
    tab.terminal.open(container);
    syncSize();

    resizeObserver = new ResizeObserver(() => {
      syncSize();
    });
    resizeObserver.observe(container);
  });

  onDestroy(() => {
    resizeObserver?.disconnect();
    fitAddon = null;
  });
</script>

<div class="terminal" bind:this={container} aria-label={`${tab.title} terminal`}></div>

<style>
  .terminal {
    width: 100%;
    height: 320px;
    border-radius: 0.75rem;
    overflow: hidden;
    background-color: var(--terminal-bg);
    border: 1px solid color-mix(in srgb, var(--terminal-fg) 20%, transparent);
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.04);
  }
</style>
