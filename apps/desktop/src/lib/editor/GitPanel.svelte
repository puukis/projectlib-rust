<script lang="ts">
  import { createEventDispatcher, onDestroy, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import {
    GitStatusResponseSchema,
    type GitStatusResponse,
    GitBranchesResponseSchema,
    type GitBranchesResponse,
    GitGraphResponseSchema,
    type GitGraphResponse,
    GitCommitDetailsSchema,
    type GitCommitDetails,
    GitStreamEventSchema,
    type GitStreamEvent,
    GitCommandHandleSchema,
  } from "@projectlib/shared";
  import { createGitgraph, templateExtend, TemplateName, type Branch } from "@gitgraph/js";

  export let repositoryPath: string | null = null;
  export let projectName: string | null = null;

  const dispatch = createEventDispatcher<{ error: { message: string } }>();

  let status: GitStatusResponse | null = null;
  let branches: GitBranchesResponse | null = null;
  let graph: GitGraphResponse | null = null;
  let commitMessage = "";
  let selectedBranch: string | null = null;
  let logEntries: {
    commandId: string;
    command: string;
    lines: string[];
    exitCode: number | null;
    success: boolean | null;
  }[] = [];
  let selectedCommit: GitCommitDetails | null = null;
  let graphContainer: HTMLDivElement | null = null;
  let streamUnlisten: Promise<UnlistenFn> | null = null;
  let loadingBranches = false;
  let performingCommit = false;
  let creatingBranch = false;
  let deletingBranch = false;
  let remoteBusy: Record<string, boolean> = {};
  let localBranches: string[] = [];
  let remoteBranches: string[] = [];

  const STREAM_EVENT = "git://stream";
  const GRAPH_TEMPLATE = templateExtend(TemplateName.Metro, {
    commit: {
      message: {
        displayAuthor: false,
        displayHash: false,
        displayBranch: false,
      },
    },
  });

  function appendLogEntry(entry: {
    commandId: string;
    command: string;
    lines: string[];
    exitCode: number | null;
    success: boolean | null;
  }) {
    const next = [...logEntries, entry];
    logEntries = next.slice(-20);
  }

  function ensureListener() {
    if (!streamUnlisten) {
      streamUnlisten = listen<GitStreamEvent>(STREAM_EVENT, (event) => {
        const parsed = GitStreamEventSchema.safeParse(event.payload);
        if (!parsed.success) {
          return;
        }
        const payload = parsed.data;
        const entry = logEntries.find((item) => item.commandId === payload.commandId);
        if (!entry) {
          return;
        }
        if (payload.kind === "stdout" || payload.kind === "stderr") {
          if (payload.data) {
            entry.lines = [...entry.lines, payload.data.trim()];
          }
        } else if (payload.kind === "completed") {
          entry.exitCode = payload.exitCode ?? null;
          entry.success = payload.success ?? null;
          if (payload.success) {
            void loadStatus();
            void loadBranches();
            void loadGraph();
          }
        } else if (payload.kind === "error") {
          entry.lines = [...entry.lines, payload.data ?? "Unknown error"];
          entry.success = false;
        }
        logEntries = [...logEntries];
      });
    }
  }

  function dispatchError(error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    dispatch("error", { message });
  }

  async function loadStatus() {
    if (!repositoryPath) {
      status = null;
      return;
    }
    try {
      const result = await invoke("git_status", { repositoryPath });
      status = GitStatusResponseSchema.parse(result);
    } catch (error) {
      dispatchError(error);
    }
  }

  async function loadBranches() {
    if (!repositoryPath) {
      branches = null;
      return;
    }
    loadingBranches = true;
    try {
      const result = await invoke("git_branches", { repositoryPath });
      branches = GitBranchesResponseSchema.parse(result);
      selectedBranch = branches.current ?? selectedBranch ?? null;
    } catch (error) {
      dispatchError(error);
    } finally {
      loadingBranches = false;
    }
  }

  async function loadGraph() {
    if (!repositoryPath) {
      graph = null;
      return;
    }
    try {
      const result = await invoke("git_graph", { repositoryPath });
      graph = GitGraphResponseSchema.parse(result);
      if (selectedCommit && !graph.entries.some((entry) => entry.commit === selectedCommit?.commit)) {
        selectedCommit = null;
      }
    } catch (error) {
      dispatchError(error);
    }
  }

  async function loadCommitDetails(commit: string) {
    if (!repositoryPath) {
      return;
    }
    try {
      const result = await invoke("git_commit_details", { request: { repositoryPath, commit } });
      selectedCommit = GitCommitDetailsSchema.parse(result);
    } catch (error) {
      dispatchError(error);
    }
  }

  async function stage(paths: string[]) {
    if (!repositoryPath || paths.length === 0) {
      return;
    }
    try {
      await invoke("git_stage", { request: { repositoryPath, paths } });
      await loadStatus();
    } catch (error) {
      dispatchError(error);
    }
  }

  async function unstage(paths: string[]) {
    if (!repositoryPath || paths.length === 0) {
      return;
    }
    try {
      await invoke("git_unstage", { request: { repositoryPath, paths } });
      await loadStatus();
    } catch (error) {
      dispatchError(error);
    }
  }

  async function commitChanges() {
    if (!repositoryPath || !commitMessage.trim()) {
      return;
    }
    performingCommit = true;
    try {
      await invoke("git_commit", {
        request: { repositoryPath, message: commitMessage.trim() },
      });
      commitMessage = "";
      await loadStatus();
      await loadGraph();
    } catch (error) {
      dispatchError(error);
    } finally {
      performingCommit = false;
    }
  }

  function stageAll() {
    const paths = [
      ...(status?.unstaged ?? []).map((file) => file.path),
      ...(status?.conflicts ?? []).map((file) => file.path),
      ...(status?.untracked ?? []),
    ];
    stage(paths);
  }

  function unstageAll() {
    const paths = (status?.staged ?? []).map((file) => file.path);
    unstage(paths);
  }

  async function switchBranch(branch: string) {
    if (!repositoryPath || !branch) {
      return;
    }
    try {
      await invoke("git_switch_branch", {
        request: { repositoryPath, branch },
      });
      selectedBranch = branch;
      await Promise.all([loadStatus(), loadBranches(), loadGraph()]);
    } catch (error) {
      dispatchError(error);
    }
  }

  async function createBranch() {
    if (!repositoryPath) {
      return;
    }
    const name = prompt("New branch name");
    if (!name) {
      return;
    }
    const trimmed = name.trim();
    if (!trimmed) {
      return;
    }
    creatingBranch = true;
    try {
      await invoke("git_switch_branch", {
        request: { repositoryPath, branch: trimmed, create: true },
      });
      selectedBranch = trimmed;
      await Promise.all([loadStatus(), loadBranches(), loadGraph()]);
    } catch (error) {
      dispatchError(error);
    } finally {
      creatingBranch = false;
    }
  }

  async function deleteBranch(branch: string) {
    if (!repositoryPath || !branch) {
      return;
    }
    if (branches?.current === branch) {
      dispatchError("Cannot delete the currently checked out branch.");
      return;
    }
    const confirmed = confirm(`Delete branch ${branch}?`);
    if (!confirmed) {
      return;
    }
    deletingBranch = true;
    try {
      await invoke("git_delete_branch", {
        request: { repositoryPath, branch, force: false },
      });
      await Promise.all([loadBranches(), loadStatus(), loadGraph()]);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      if (message.toLowerCase().includes("not fully merged")) {
        const force = confirm(`${branch} is not fully merged. Force delete?`);
        if (force) {
          try {
            await invoke("git_delete_branch", {
              request: { repositoryPath, branch, force: true },
            });
            await Promise.all([loadBranches(), loadStatus(), loadGraph()]);
          } catch (inner) {
            dispatchError(inner);
          }
        }
      } else {
        dispatchError(error);
      }
    } finally {
      deletingBranch = false;
    }
  }

  async function trackRemoteBranch(remoteRef: string) {
    if (!repositoryPath) {
      return;
    }
    const key = remoteRef;
    remoteBusy = { ...remoteBusy, [key]: true };
    const trimmed = remoteRef.replace(/^remotes\//, "");
    const parts = trimmed.split("/");
    if (parts.length < 2) {
      const { [key]: _removed, ...rest } = remoteBusy;
      remoteBusy = rest;
      return;
    }
    const localName = parts.slice(1).join("/");
    try {
      await invoke("git_switch_branch", {
        request: { repositoryPath, branch: localName, create: true, track: true },
      });
      selectedBranch = localName;
      await Promise.all([loadStatus(), loadBranches(), loadGraph()]);
    } catch (error) {
      dispatchError(error);
    } finally {
      const { [key]: _removed, ...rest } = remoteBusy;
      remoteBusy = rest;
    }
  }

  async function runRemoteCommand(command: "fetch" | "pull" | "push") {
    if (!repositoryPath) {
      return;
    }
    ensureListener();
    try {
      const args = { request: { repositoryPath } };
      const handleRaw =
        command === "fetch"
          ? await invoke("git_fetch_all", args)
          : command === "pull"
          ? await invoke("git_pull", args)
          : await invoke("git_push", args);
      const parsed = GitCommandHandleSchema.safeParse(handleRaw);
      if (parsed.success) {
        appendLogEntry({
          commandId: parsed.data.commandId,
          command: `${command.toUpperCase()} @ ${new Date().toLocaleTimeString()}`,
          lines: [],
          exitCode: null,
          success: null,
        });
      }
    } catch (error) {
      dispatchError(error);
    }
  }

  function renderGraph() {
    if (!graphContainer) {
      return;
    }
    graphContainer.innerHTML = "";
    if (!graph || graph.entries.length === 0) {
      return;
    }

    const gitgraph = createGitgraph(graphContainer, { template: GRAPH_TEMPLATE });
    const chronological = [...graph.entries].reverse();
    const laneBranches = new Map<number, Branch>();
    const commitLane = new Map<string, number>();
    const childrenCount = new Map<string, number>();
    const availableLanes: number[] = [];
    let nextLane = 1;

    const ensureBranch = (lane: number, parentHash?: string, reset = false): Branch => {
      if (!reset && laneBranches.has(lane)) {
        return laneBranches.get(lane)!;
      }
      const branchName =
        lane === 0 ? projectName ?? "main" : `${projectName ?? "branch"}-${lane + 1}`;
      let branch: Branch;
      if (lane === 0) {
        branch = laneBranches.get(lane) ?? gitgraph.branch({ name: branchName });
      } else {
        const originLane = parentHash ? commitLane.get(parentHash) ?? 0 : 0;
        const originBranch = laneBranches.get(originLane) ?? gitgraph.branch({ name: projectName ?? "main" });
        branch = originBranch.branch({ name: branchName, from: parentHash });
      }
      laneBranches.set(lane, branch);
      return branch;
    };

    for (const entry of chronological) {
      const primaryParent = entry.parents[0];
      let lane = 0;
      let reuseLane = false;
      if (primaryParent) {
        const parentLane = commitLane.get(primaryParent) ?? 0;
        const usage = (childrenCount.get(primaryParent) ?? 0) + 1;
        childrenCount.set(primaryParent, usage);
        if (usage === 1) {
          lane = parentLane;
        } else {
          lane = availableLanes.length > 0 ? availableLanes.shift()! : nextLane++;
          reuseLane = true;
        }
      }
      const branch = ensureBranch(lane, primaryParent, reuseLane);
      const author = entry.author && entry.author.trim().length > 0 ? entry.author : "Unknown";
      branch.commit({
        subject: entry.subject || entry.commit.substring(0, 7),
        author,
        hash: entry.commit,
        body: entry.date,
        parents: entry.parents,
        onClick: () => loadCommitDetails(entry.commit),
      });
      commitLane.set(entry.commit, lane);

      if (entry.parents.length > 1) {
        for (let i = 1; i < entry.parents.length; i += 1) {
          const parentLane = commitLane.get(entry.parents[i]);
          if (parentLane !== undefined && parentLane !== 0 && !availableLanes.includes(parentLane)) {
            availableLanes.push(parentLane);
          }
        }
        availableLanes.sort((a, b) => a - b);
      }
    }
  }

  onMount(() => {
    if (repositoryPath) {
      loadStatus();
      loadBranches();
      loadGraph();
    }
  });

  onDestroy(() => {
    if (streamUnlisten) {
      streamUnlisten.then((fn) => fn());
      streamUnlisten = null;
    }
  });

  $: if (repositoryPath) {
    loadStatus();
    loadBranches();
    loadGraph();
  } else {
    status = null;
    branches = null;
    graph = null;
    logEntries = [];
    selectedCommit = null;
    selectedBranch = null;
    remoteBusy = {};
  }

  $: {
    const locals = branches?.local ?? [];
    if (selectedBranch && !locals.includes(selectedBranch)) {
      localBranches = [selectedBranch, ...locals];
    } else {
      localBranches = locals;
    }
    remoteBranches = branches?.remote ?? [];
  }

  $: stagedFiles = status?.staged ?? [];
  $: unstagedFiles = status?.unstaged ?? [];
  $: conflictFiles = status?.conflicts ?? [];
  $: untrackedFiles = status?.untracked ?? [];

  $: if (graph && graphContainer) {
    renderGraph();
  } else if (graphContainer) {
    graphContainer.innerHTML = "";
  }
</script>

<section class="git-panel">
  <header>
    <h2>Git</h2>
    <div class="header-actions">
      <div class="branch-selector">
        <label>
          Branch
          <select
            bind:value={selectedBranch}
            disabled={!branches || loadingBranches}
            on:change={(event) => switchBranch((event.target as HTMLSelectElement).value)}
          >
            {#if !localBranches.length}
              <option value="" disabled>No branches</option>
            {:else}
              {#each localBranches as branch}
                <option value={branch}>{branch}</option>
              {/each}
            {/if}
          </select>
        </label>
      </div>
      <div class="branch-actions">
        <button type="button" on:click={createBranch} disabled={!repositoryPath || creatingBranch}>
          {creatingBranch ? "Creating…" : "New Branch"}
        </button>
        <button
          type="button"
          class="danger"
          on:click={() => selectedBranch && deleteBranch(selectedBranch)}
          disabled={!selectedBranch || deletingBranch || selectedBranch === branches?.current}
        >
          {deletingBranch ? "Deleting…" : "Delete Branch"}
        </button>
      </div>
      <div class="remote-actions">
        <button type="button" on:click={() => runRemoteCommand("fetch")}>Fetch</button>
        <button type="button" on:click={() => runRemoteCommand("pull")}>Pull</button>
        <button type="button" on:click={() => runRemoteCommand("push")}>Push</button>
      </div>
    </div>
  </header>

  <section class="status">
    <div class="column">
      <h3>Staged</h3>
      {#if stagedFiles.length === 0}
        <p class="empty">No staged changes</p>
      {:else}
        <ul>
          {#each stagedFiles as file}
            <li>
              <span>{file.path}</span>
              <button type="button" on:click={() => unstage([file.path])}>Unstage</button>
            </li>
          {/each}
        </ul>
        <button type="button" class="secondary" on:click={unstageAll}>Unstage All</button>
      {/if}
    </div>
    <div class="column">
      <h3>Changes</h3>
      {#if unstagedFiles.length === 0 && conflictFiles.length === 0 && untrackedFiles.length === 0}
        <p class="empty">Working tree clean</p>
      {:else}
        <ul>
          {#each unstagedFiles as file}
            <li>
              <span>{file.path}</span>
              <button type="button" on:click={() => stage([file.path])}>Stage</button>
            </li>
          {/each}
          {#each conflictFiles as file}
            <li class="conflict">
              <span>{file.path}</span>
              <button type="button" on:click={() => stage([file.path])}>Resolve & Stage</button>
            </li>
          {/each}
          {#each untrackedFiles as path}
            <li>
              <span>{path}</span>
              <button type="button" on:click={() => stage([path])}>Stage</button>
            </li>
          {/each}
        </ul>
        <button type="button" class="secondary" on:click={stageAll}>Stage All</button>
      {/if}
    </div>
  </section>

  <section class="commit">
    <h3>Commit</h3>
    <textarea
      placeholder="Commit message"
      bind:value={commitMessage}
      rows={3}
    ></textarea>
    <button type="button" class="primary" disabled={performingCommit || !commitMessage.trim()} on:click={commitChanges}>
      {performingCommit ? "Committing…" : "Commit"}
    </button>
  </section>

  <section class="graph">
    <h3>History</h3>
    {#if graph && graph.entries.length > 0}
      <div class="graph-container" bind:this={graphContainer}></div>
    {:else}
      <p class="empty">No commits to display.</p>
    {/if}
    {#if selectedCommit}
      <div class="commit-details">
        <header>
          <div>
            <h4>{selectedCommit.commit}</h4>
            <p>{selectedCommit.author} · {selectedCommit.date}</p>
          </div>
          <button type="button" class="close" on:click={() => (selectedCommit = null)}>Close</button>
        </header>
        <pre>{selectedCommit.message}</pre>
        <ul>
          {#each selectedCommit.files as file}
            <li>
              <span class="badge">{file.status}</span>
              <span>{file.path}</span>
            </li>
          {/each}
        </ul>
      </div>
    {/if}
  </section>

  <section class="remote-branches">
    <h3>Remote branches</h3>
    {#if remoteBranches.length === 0}
      <p class="empty">No remote branches</p>
    {:else}
      <ul>
        {#each remoteBranches as remote}
          <li>
            <span>{remote}</span>
            <button
              type="button"
              on:click={() => trackRemoteBranch(remote)}
              disabled={Boolean(remoteBusy[remote])}
            >
              {remoteBusy[remote] ? "Tracking…" : "Track"}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <section class="remote-log">
    <details>
      <summary>Remote Operations</summary>
      {#if logEntries.length === 0}
        <p class="empty">No remote operations yet.</p>
      {:else}
        {#each logEntries as entry}
          <article>
            <header>
              <strong>{entry.command}</strong>
              {#if entry.success !== null}
                <span class:success={entry.success} class:failure={!entry.success}>
                  {entry.success ? "Success" : "Failed"}
                  {#if entry.exitCode !== null} (exit {entry.exitCode}){/if}
                </span>
              {/if}
            </header>
            {#if entry.lines.length > 0}
              <pre>{entry.lines.join("\n")}</pre>
            {/if}
          </article>
        {/each}
      {/if}
    </details>
  </section>
</section>

<style>
  .git-panel {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    height: 100%;
    overflow: auto;
  }

  header {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  header h2 {
    margin: 0;
  }

  .header-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    align-items: center;
    justify-content: space-between;
  }

  .branch-selector label {
    display: flex;
    flex-direction: column;
    font-size: 0.85rem;
    gap: 0.25rem;
  }

  select {
    background: rgba(0, 0, 0, 0.2);
    color: inherit;
    border-radius: 0.5rem;
    border: 1px solid rgba(255, 255, 255, 0.1);
    padding: 0.25rem 0.5rem;
  }

  .branch-actions {
    display: flex;
    gap: 0.5rem;
  }

  .branch-actions button,
  .remote-actions button,
  .status button,
  .commit button,
  .remote-branches button {
    background: rgba(255, 255, 255, 0.1);
    border: none;
    padding: 0.35rem 0.75rem;
    border-radius: 0.5rem;
    cursor: pointer;
    color: inherit;
  }

  .branch-actions button.danger {
    background: rgba(239, 68, 68, 0.25);
  }

  .branch-actions button.danger:hover:not(:disabled) {
    background: rgba(239, 68, 68, 0.4);
  }

  .branch-actions button:disabled,
  .remote-actions button:disabled,
  .remote-branches button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .remote-actions {
    display: flex;
    gap: 0.5rem;
  }

  .primary {
    background: #2563eb;
    color: white;
  }

  .secondary {
    margin-top: 0.5rem;
  }

  .status {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 1rem;
  }

  .status ul {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .status li {
    display: flex;
    justify-content: space-between;
    gap: 0.5rem;
    align-items: center;
  }

  .status li span {
    flex: 1;
    word-break: break-all;
  }

  .conflict span {
    color: #f97316;
  }

  .commit textarea {
    width: 100%;
    background: rgba(0, 0, 0, 0.2);
    color: inherit;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 0.5rem;
    padding: 0.5rem;
  }

  .graph-container {
    min-height: 200px;
    background: rgba(0, 0, 0, 0.15);
    border-radius: 0.75rem;
    padding: 0.5rem;
  }

  .graph .empty {
    background: rgba(0, 0, 0, 0.15);
    border-radius: 0.75rem;
    padding: 0.75rem;
  }

  .commit-details {
    margin-top: 1rem;
    background: rgba(0, 0, 0, 0.2);
    border-radius: 0.75rem;
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .commit-details header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.75rem;
  }

  .commit-details pre {
    background: rgba(0, 0, 0, 0.25);
    padding: 0.5rem;
    border-radius: 0.5rem;
    white-space: pre-wrap;
  }

  .commit-details ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .commit-details li {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .commit-details button.close {
    background: rgba(255, 255, 255, 0.1);
    border: none;
    border-radius: 0.5rem;
    padding: 0.25rem 0.5rem;
    cursor: pointer;
    color: inherit;
  }

  .badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0.1rem 0.5rem;
    background: rgba(37, 99, 235, 0.2);
    border-radius: 0.5rem;
    font-size: 0.75rem;
  }

  .remote-branches {
    background: rgba(0, 0, 0, 0.2);
    border-radius: 0.75rem;
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .remote-branches ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .remote-branches li {
    display: flex;
    justify-content: space-between;
    gap: 0.5rem;
    align-items: center;
  }

  .remote-log details {
    background: rgba(0, 0, 0, 0.2);
    border-radius: 0.75rem;
    padding: 0.75rem;
  }

  .remote-log article {
    border-top: 1px solid rgba(255, 255, 255, 0.1);
    padding-top: 0.5rem;
    margin-top: 0.5rem;
  }

  .remote-log pre {
    background: rgba(0, 0, 0, 0.25);
    padding: 0.5rem;
    border-radius: 0.5rem;
    white-space: pre-wrap;
  }

  .success {
    color: #10b981;
  }

  .failure {
    color: #ef4444;
  }

  .empty {
    opacity: 0.7;
  }
</style>
