<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { Project } from "@projectlib/db";
  import type { RunState } from "../lib/run";
  import RunButton from "./RunButton.svelte";
  import RunContextMenu from "./RunContextMenu.svelte";

  export let project: Project;
  export let selected = false;
  export let state: RunState;

  const dispatch = createEventDispatcher();

  let menuPosition = { x: 0, y: 0 };
  let showMenu = false;

  function openMenu(event: MouseEvent) {
    event.preventDefault();
    menuPosition = { x: event.clientX, y: event.clientY };
    showMenu = true;
  }

  function closeMenu() {
    showMenu = false;
  }

  function handleRun(event?: MouseEvent) {
    event?.stopPropagation();
    dispatch("run");
  }

  function handleStop() {
    dispatch("stop");
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" && event.shiftKey) {
      event.preventDefault();
      dispatch("runWithArgs");
    } else if (event.key === "Enter") {
      event.preventDefault();
      dispatch("run");
    } else if (event.key === "Escape") {
      event.preventDefault();
      dispatch("stop");
    }
  }

  $: statusLabel = (() => {
    switch (state.status) {
      case "starting":
        return "Starting";
      case "running":
        return "Running";
      case "succeeded":
        return state.lastExitCode === 0 ? "Last run succeeded" : "Run finished";
      case "failed":
        return `Failed (exit ${state.lastExitCode ?? "?"})`;
      case "stopped":
        return "Stopped";
      default:
        return state.lastExitCode === null
          ? "Not run yet"
          : `Last exit ${state.lastExitCode}`;
    }
  })();
</script>

<article
  class:selected
  role="button"
  aria-pressed={selected}
  tabindex="0"
  on:click={() => dispatch("select")}
  on:keydown={handleKeydown}
  on:contextmenu={openMenu}
>
  <header>
    <div>
      <h3>{project.name}</h3>
      <p class="path" title={project.path}>{project.path}</p>
    </div>
    <div class="run-control">
      <RunButton
        projectName={project.name}
        status={state.status}
        on:click={handleRun}
      />
    </div>
  </header>
  <footer>
    <span>{statusLabel}</span>
    {#if state.status === "running"}
      <button type="button" class="inline-stop" on:click|stopPropagation={handleStop}>Stop</button>
    {/if}
  </footer>

  {#if showMenu}
    <RunContextMenu
      projectName={project.name}
      position={menuPosition}
      showStop={state.status === "running"}
      on:run={() => {
        handleRun();
        closeMenu();
      }}
      on:runWithArgs={() => {
        dispatch("runWithArgs");
        closeMenu();
      }}
      on:editConfig={() => {
        dispatch("editConfig");
        closeMenu();
      }}
      on:openTerminal={() => {
        dispatch("openTerminal");
        closeMenu();
      }}
      on:openFolder={() => {
        dispatch("openFolder");
        closeMenu();
      }}
      on:stop={() => {
        handleStop();
        closeMenu();
      }}
      on:close={closeMenu}
    />
  {/if}
</article>

<style>
  article {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    padding: 1rem;
    border-radius: 0.9rem;
    border: 1px solid color-mix(in srgb, currentColor 18%, transparent);
    background: color-mix(in srgb, var(--terminal-bg) 70%, transparent);
    cursor: pointer;
    transition: border-color 0.2s ease, transform 0.2s ease, background 0.2s ease;
    position: relative;
  }

  article:hover {
    border-color: color-mix(in srgb, currentColor 35%, transparent);
    background: color-mix(in srgb, var(--terminal-bg) 85%, transparent);
    transform: translateY(-2px);
  }

  article.selected {
    border-color: #22c55e;
    box-shadow: 0 12px 32px rgba(34, 197, 94, 0.2);
  }

  header {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: center;
  }

  .run-control {
    display: inline-flex;
  }

  h3 {
    margin: 0;
    font-size: 1.1rem;
  }

  .path {
    margin: 0.25rem 0 0;
    font-size: 0.85rem;
    opacity: 0.7;
    word-break: break-all;
  }

  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 0.9rem;
    color: color-mix(in srgb, currentColor 70%, transparent);
  }

  .inline-stop {
    border: none;
    background: transparent;
    color: #ef4444;
    font: inherit;
    cursor: pointer;
    text-decoration: underline;
  }

  .inline-stop:focus-visible {
    outline: none;
    text-decoration: underline;
  }
</style>
