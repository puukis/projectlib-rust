<script lang="ts">
  import { createEventDispatcher, onDestroy } from "svelte";

  export type EditorTab = {
    path: string;
    name: string;
    language: string;
    dirty: boolean;
  };

  export let tabs: EditorTab[] = [];
  export let activePath: string | null = null;

  const dispatch = createEventDispatcher<{
    select: { path: string };
    close: { path: string };
    closeOthers: { path: string };
    reveal: { path: string };
  }>();

  let contextTab: EditorTab | null = null;
  let contextX = 0;
  let contextY = 0;

  function openContextMenu(event: MouseEvent, tab: EditorTab) {
    event.preventDefault();
    contextTab = tab;
    contextX = event.clientX;
    contextY = event.clientY;
    window.addEventListener("click", closeContextMenuOnce, { once: true });
  }

  function closeContextMenuOnce() {
    contextTab = null;
  }

  onDestroy(() => {
    window.removeEventListener("click", closeContextMenuOnce);
  });

  function handleAuxClick(event: MouseEvent, tab: EditorTab) {
    if (event.button === 1) {
      event.preventDefault();
      dispatch("close", { path: tab.path });
    }
  }
</script>

<nav class="tabs" role="tablist">
  {#each tabs as tab}
    <button
      class:active={tab.path === activePath}
      role="tab"
      aria-selected={tab.path === activePath}
      on:click={() => dispatch("select", { path: tab.path })}
      on:auxclick={(event) => handleAuxClick(event, tab)}
      on:contextmenu={(event) => openContextMenu(event, tab)}
    >
      <span class="name">{tab.name}</span>
      {#if tab.dirty}
        <span class="dirty" aria-label="unsaved changes">●</span>
      {/if}
      <span class="close" on:click|stopPropagation={() => dispatch("close", { path: tab.path })}
        >×</span
      >
    </button>
  {/each}
</nav>

{#if contextTab}
  <div class="context-menu" style={`top:${contextY}px;left:${contextX}px`}>
    <button
      type="button"
      on:click={() => {
        contextTab && dispatch("close", { path: contextTab.path });
        contextTab = null;
      }}
    >
      Close
    </button>
    <button
      type="button"
      on:click={() => {
        if (contextTab) {
          dispatch("closeOthers", { path: contextTab.path });
        }
        contextTab = null;
      }}
    >
      Close Others
    </button>
    <button
      type="button"
      on:click={() => {
        if (contextTab) {
          dispatch("reveal", { path: contextTab.path });
        }
        contextTab = null;
      }}
    >
      Reveal in Tree
    </button>
  </div>
{/if}

<style>
  .tabs {
    display: flex;
    gap: 0.25rem;
    padding: 0.25rem 0.5rem;
    background: var(--tab-bg, rgba(0, 0, 0, 0.2));
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    overflow-x: auto;
  }

  button[role="tab"] {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.35rem 0.75rem;
    background: transparent;
    border: none;
    color: inherit;
    border-radius: 0.5rem 0.5rem 0 0;
    cursor: pointer;
    position: relative;
  }

  button[role="tab"].active {
    background: rgba(255, 255, 255, 0.1);
    font-weight: 600;
  }

  .close {
    font-weight: bold;
    opacity: 0.6;
  }

  .close:hover {
    opacity: 1;
  }

  .dirty {
    color: #ff8a65;
  }

  .context-menu {
    position: fixed;
    background: #1f2937;
    color: white;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 0.5rem;
    display: flex;
    flex-direction: column;
    min-width: 160px;
    z-index: 1000;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.3);
  }

  .context-menu button {
    background: none;
    border: none;
    color: inherit;
    text-align: left;
    padding: 0.5rem 0.75rem;
    cursor: pointer;
  }

  .context-menu button:hover {
    background: rgba(255, 255, 255, 0.1);
  }
</style>
