<script lang="ts">
  import { derived } from "svelte/store";
  import type { Project } from "@projectlib/db";
  import { terminalService, type TerminalTab } from "../lib/terminal";
  import TerminalView from "./TerminalView.svelte";

  export let projects: Project[] = [];
  export let selectedProjectId: string | null = null;
  export let creatingError: string | null = null;
  export let onCreateTab: (projectId: string) => Promise<void>;
  export let focusTabId: string | null = null;

  const tabsStore = terminalService.tabs;
  const tabsByProject = derived(tabsStore, ($tabsStore) => {
    const grouped = new Map<string, TerminalTab[]>();
    for (const tab of $tabsStore) {
      const list = grouped.get(tab.projectId) ?? [];
      list.push(tab);
      grouped.set(tab.projectId, list);
    }
    return grouped;
  });

  let activeTabId: string | null = null;

  $: currentProjectTabs = selectedProjectId
    ? tabsByProjectMap?.get(selectedProjectId) ?? []
    : [];
  $: if (currentProjectTabs.length > 0) {
    const hasActive = currentProjectTabs.some((tab) => tab.id === activeTabId);
    if (!hasActive) {
      activeTabId = currentProjectTabs[0].id;
    }
  } else {
    activeTabId = null;
  }

  let tabsByProjectMap: Map<string, TerminalTab[]> | undefined;
  $: tabsByProjectMap = $tabsByProject;

  $: if (focusTabId && currentProjectTabs.some((tab) => tab.id === focusTabId)) {
    activeTabId = focusTabId;
  }

  function handleSelectTab(tabId: string) {
    activeTabId = tabId;
  }

  function handleCloseTab(tabId: string) {
    terminalService.dispose(tabId);
  }

  function projectLabel(project: Project): string {
    return `${project.name}`;
  }
</script>

<section class="terminal-panel">
  <header>
    <div class="project-selector">
      <label>
        Project
        <select bind:value={selectedProjectId}>
          <option value="" disabled selected={!selectedProjectId}>Choose a project</option>
          {#each projects as project}
            <option value={project.id}>{projectLabel(project)}</option>
          {/each}
        </select>
      </label>
      <button
        type="button"
        on:click={() => selectedProjectId && onCreateTab(selectedProjectId)}
        disabled={!selectedProjectId}
      >
        New terminal
      </button>
    </div>
    {#if creatingError}
      <p class="error">{creatingError}</p>
    {/if}
  </header>

  {#if selectedProjectId}
    {#if currentProjectTabs.length === 0}
      <p class="empty">No terminals yet for this project.</p>
    {:else}
      <div class="tablist" role="tablist">
        {#each currentProjectTabs as tab}
          <button
            type="button"
            class:active={tab.id === activeTabId}
            role="tab"
            aria-selected={tab.id === activeTabId}
            on:click={() => handleSelectTab(tab.id)}
          >
            <span>{tab.title}</span>
            <span class="spacer"></span>
            <span class="close" role="presentation" on:click|stopPropagation={() => handleCloseTab(tab.id)}
              >&times;</span
            >
          </button>
        {/each}
      </div>
      {#if activeTabId}
        {#each currentProjectTabs as tab (tab.id)}
          {#if tab.id === activeTabId}
            <TerminalView {tab} />
          {/if}
        {/each}
      {/if}
    {/if}
  {:else}
    <p class="empty">Select a project to start a terminal session.</p>
  {/if}
</section>

<style>
  .terminal-panel {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  header {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .project-selector {
    display: flex;
    gap: 0.75rem;
    align-items: flex-end;
  }

  label {
    font-weight: 600;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  select,
  button {
    font: inherit;
    padding: 0.45rem 0.75rem;
    border-radius: 0.5rem;
    border: 1px solid color-mix(in srgb, currentColor 35%, transparent);
    background: color-mix(in srgb, var(--terminal-bg) 75%, transparent);
    color: inherit;
  }

  button {
    cursor: pointer;
    transition: background 0.2s ease, color 0.2s ease;
  }

  button:disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }

  button:not(:disabled):hover {
    background: color-mix(in srgb, var(--terminal-bg) 90%, transparent);
  }

  .tablist {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }

  .tablist button {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: transparent;
    border-radius: 0.6rem;
    padding: 0.35rem 0.75rem;
    border: 1px solid color-mix(in srgb, currentColor 25%, transparent);
  }

  .tablist button.active {
    background: color-mix(in srgb, var(--terminal-bg) 85%, transparent);
    border-color: color-mix(in srgb, currentColor 45%, transparent);
  }

  .tablist .spacer {
    flex: 1 1 auto;
  }

  .tablist .close {
    font-size: 1.1rem;
    line-height: 1;
    cursor: pointer;
  }

  .error {
    color: #ef4444;
    margin: 0;
    font-size: 0.9rem;
  }

  .empty {
    margin: 0;
    font-style: italic;
  }
</style>
