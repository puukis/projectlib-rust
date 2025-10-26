<script lang="ts">
  import { onMount } from "svelte";
  import { createEventDispatcher } from "svelte";

  export let projectName: string;
  export let showStop = false;
  export let position: { x: number; y: number } = { x: 0, y: 0 };

  const dispatch = createEventDispatcher();

  function emit(action: string) {
    dispatch(action);
    dispatch("close");
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.stopPropagation();
      dispatch("close");
    }
  }

  onMount(() => {
    const listener = (event: MouseEvent) => {
      if (!(event.target instanceof Node)) {
        return;
      }
      if (!menuElement?.contains(event.target)) {
        dispatch("close");
      }
    };
    window.addEventListener("mousedown", listener);
    return () => {
      window.removeEventListener("mousedown", listener);
    };
  });

  let menuElement: HTMLUListElement | null = null;
</script>

<ul
  class="menu"
  bind:this={menuElement}
  style={`top:${position.y}px;left:${position.x}px;`}
  role="menu"
  on:keydown={handleKeydown}
>
  <li role="menuitem">
    <button type="button" on:click={() => emit("run")}>Run</button>
  </li>
  <li role="menuitem">
    <button type="button" on:click={() => emit("runWithArgs")}>Run with Args…</button>
  </li>
  <li role="menuitem">
    <button type="button" on:click={() => emit("editConfig")}>Edit Run Config…</button>
  </li>
  <li role="menuitem">
    <button type="button" on:click={() => emit("openTerminal")}>Open Terminal Here</button>
  </li>
  <li role="menuitem">
    <button type="button" on:click={() => emit("openFolder")}>Open Folder</button>
  </li>
  {#if showStop}
    <li role="menuitem">
      <button type="button" class="stop" on:click={() => emit("stop")}>Stop</button>
    </li>
  {/if}
</ul>

<style>
  .menu {
    position: fixed;
    z-index: 1000;
    min-width: 14rem;
    background: color-mix(in srgb, var(--terminal-bg) 90%, transparent);
    border-radius: 0.75rem;
    border: 1px solid color-mix(in srgb, var(--terminal-fg) 20%, transparent);
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.25);
    padding: 0.35rem;
    list-style: none;
    margin: 0;
  }

  li {
    margin: 0;
  }

  button {
    width: 100%;
    text-align: left;
    padding: 0.5rem 0.75rem;
    border: none;
    background: transparent;
    color: inherit;
    font: inherit;
    border-radius: 0.6rem;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  button:hover,
  button:focus-visible {
    background: color-mix(in srgb, var(--terminal-bg) 80%, transparent);
    outline: none;
  }

  button.stop {
    color: #ef4444;
  }
</style>
