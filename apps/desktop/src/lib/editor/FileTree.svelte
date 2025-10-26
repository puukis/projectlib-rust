<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { TreeNode } from "./file-tree-service";

  export let rootPath: string = "";
  export let nodes: TreeNode[] = [];
  export let selectedPath: string | null = null;

  const dispatch = createEventDispatcher<{
    open: { path: string };
    refresh: void;
    reveal: { path: string };
    createFile: { directory: string };
    createFolder: { directory: string };
    rename: { path: string };
    delete: { path: string };
    terminal: { directory: string };
  }>();

const expanded = new Set<string>();

let contextNode: TreeNode | null = null;
let contextX = 0;
let contextY = 0;
let flattened: { node: TreeNode; level: number }[] = [];
let lastRootPath = rootPath;

  function isExpanded(path: string): boolean {
    return expanded.has(path);
  }

  function toggle(node: TreeNode) {
    if (node.type !== "directory") {
      return;
    }
    if (expanded.has(node.path)) {
      expanded.delete(node.path);
    } else {
      expanded.add(node.path);
    }
  }

  function open(node: TreeNode) {
    if (node.type === "file") {
      dispatch("open", { path: node.path });
    } else {
      toggle(node);
    }
  }

  function showContext(event: MouseEvent, node: TreeNode) {
    event.preventDefault();
    contextNode = node;
    contextX = event.clientX;
    contextY = event.clientY;
    window.addEventListener("click", closeContextMenu, { once: true });
  }

  function closeContextMenu() {
    contextNode = null;
  }

  function indent(level: number): string {
    return `calc(${level} * 1.25rem)`;
  }

function buildFlattened(nodes: TreeNode[], level = 0, acc: { node: TreeNode; level: number }[] = []) {
  for (const node of nodes) {
    acc.push({ node, level });
    if (node.type === "directory" && node.children && isExpanded(node.path)) {
      buildFlattened(node.children, level + 1, acc);
    }
  }
  return acc;
}

$: flattened = buildFlattened(nodes, 0, []);

$: if (rootPath !== lastRootPath) {
  expanded.clear();
  lastRootPath = rootPath;
}
</script>

<div class="tree">
  <header>
    <span>Files</span>
    <button type="button" on:click={() => dispatch("refresh")}>⟲</button>
  </header>
  <div class="nodes">
    {#each flattened as entry (entry.node.path)}
      <div
        class:directory={entry.node.type === "directory"}
        class:file={entry.node.type === "file"}
        class:expanded={entry.node.type === "directory" && isExpanded(entry.node.path)}
        class:selected={selectedPath === entry.node.path}
        style={`padding-left:${indent(entry.level)};`}
        on:click={() => open(entry.node)}
        on:contextmenu={(event) => showContext(event, entry.node)}
      >
        {#if entry.node.type === "directory"}
          <span class="chevron">{isExpanded(entry.node.path) ? "▼" : "▶"}</span>
        {:else}
          <span class="chevron">•</span>
        {/if}
        <span class="label">{entry.node.name}</span>
      </div>
    {/each}
  </div>
</div>

{#if contextNode}
  <div class="context-menu" style={`top:${contextY}px;left:${contextX}px`}>
    {#if contextNode.type === "file"}
      <button type="button" on:click={() => contextNode && dispatch("open", { path: contextNode.path })}>
        Open
      </button>
    {/if}
    <button
      type="button"
      on:click={() => {
        if (contextNode?.type === "directory") {
          dispatch("createFile", { directory: contextNode.path });
        }
        contextNode = null;
      }}
    >
      New File
    </button>
    <button
      type="button"
      on:click={() => {
        if (contextNode?.type === "directory") {
          dispatch("createFolder", { directory: contextNode.path });
        }
        contextNode = null;
      }}
    >
      New Folder
    </button>
    <button
      type="button"
      on:click={() => {
        if (contextNode) {
          dispatch("rename", { path: contextNode.path });
        }
        contextNode = null;
      }}
    >
      Rename
    </button>
    <button
      type="button"
      on:click={() => {
        if (contextNode) {
          dispatch("delete", { path: contextNode.path });
        }
        contextNode = null;
      }}
    >
      Delete
    </button>
    <button
      type="button"
      on:click={() => {
        if (contextNode) {
          dispatch("reveal", { path: contextNode.path });
        }
        contextNode = null;
      }}
    >
      Reveal in Finder
    </button>
    {#if contextNode?.type === "directory"}
      <button
        type="button"
        on:click={() => {
          if (contextNode) {
            dispatch("terminal", { directory: contextNode.path });
          }
          contextNode = null;
        }}
      >
        Open Terminal Here
      </button>
    {/if}
  </div>
{/if}

<style>
  .tree {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem 0.75rem;
    font-weight: 600;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  }

  header button {
    background: transparent;
    border: none;
    cursor: pointer;
    color: inherit;
  }

  .nodes {
    flex: 1;
    overflow: auto;
  }

  .nodes div {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.25rem 0.5rem;
    cursor: pointer;
  }

  .nodes div.selected {
    background: rgba(255, 255, 255, 0.1);
  }

  .chevron {
    width: 1.25rem;
    text-align: center;
  }

  .context-menu {
    position: fixed;
    background: #111827;
    color: white;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 0.5rem;
    padding: 0.25rem 0;
    min-width: 180px;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
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
