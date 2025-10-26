<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { Shell } from "@projectlib/ui";
  import { listProjects, upsertProject, type Project } from "@projectlib/db";
  import TerminalTabs from "./components/TerminalTabs.svelte";
  import { terminalService } from "./lib/terminal";
  import {
    PingSchema,
    type Ping,
    GitPathInfoSchema,
    type GitPathInfo,
    GitRepositoryInfoSchema,
    type GitRepositoryInfo,
    GitStatusResponseSchema,
    type GitStatusResponse
  } from "@projectlib/shared";

  let ping: Ping = { message: "Pinging backend..." };
  let error: string | null = null;
  let gitInfo: GitPathInfo | null = null;
  let repoPath = "";
  let repoInfo: GitRepositoryInfo | null = null;
  let repoError: string | null = null;
  let status: GitStatusResponse | null = null;
  let statusError: string | null = null;
  let projects: Project[] = [];
  let projectError: string | null = null;
  let projectMessage: string | null = null;
  let selectedProjectId: string | null = null;
  let terminalError: string | null = null;
  let savingProject = false;

  onMount(async () => {
    try {
      const response = await invoke<string>("ping", { message: "Hello from Svelte" });
      ping = PingSchema.parse({ message: response });
      const info = await invoke("git_path_info");
      gitInfo = GitPathInfoSchema.parse(info);
      await loadProjects();
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  });

  async function loadProjects() {
    try {
      projectError = null;
      const loaded = await listProjects();
      projects = loaded;
      if (projects.length > 0) {
        const firstId = projects[0].id;
        if (!selectedProjectId || !projects.some((project) => project.id === selectedProjectId)) {
          selectedProjectId = firstId;
        }
      }
    } catch (err) {
      projectError = err instanceof Error ? err.message : String(err);
    }
  }

  async function detectRepository() {
    repoError = null;
    repoInfo = null;
    projectMessage = null;
    projectError = null;
    if (!repoPath.trim()) {
      repoError = "Enter a folder path to inspect.";
      return;
    }

    try {
      const result = await invoke("git_detect_repository", {
        request: { repository_path: repoPath }
      });
      repoInfo = GitRepositoryInfoSchema.parse(result);
    } catch (err) {
      repoError = err instanceof Error ? err.message : String(err);
    }
  }

  async function loadStatus() {
    statusError = null;
    status = null;
    projectMessage = null;
    if (!repoPath.trim()) {
      statusError = "Enter a repository path first.";
      return;
    }

    try {
      const result = await invoke("git_status", { repository_path: repoPath });
      status = GitStatusResponseSchema.parse(result);
    } catch (err) {
      statusError = err instanceof Error ? err.message : String(err);
    }
  }

  async function saveRepositoryAsProject() {
    if (!repoInfo?.isRepository || !repoInfo.worktreeRoot) {
      projectMessage = "Detect a repository before saving it.";
      return;
    }

    projectMessage = null;
    savingProject = true;

    try {
      const existing = projects.find((project) => project.path === repoInfo.worktreeRoot);
      const now = Date.now();
      const projectId = existing?.id ?? crypto.randomUUID();
      const pathSegments = repoInfo.worktreeRoot.split(/[/\\]/).filter(Boolean);
      const name = existing?.name ?? pathSegments[pathSegments.length - 1] ?? "Repository";

      await upsertProject({
        id: projectId,
        name,
        path: repoInfo.worktreeRoot,
        detectedLang: existing?.detectedLang ?? null,
        createdAt: existing?.createdAt ?? now,
        updatedAt: now,
      });

      await loadProjects();
      selectedProjectId = projectId;
      projectMessage = existing ? "Project entry updated." : "Project saved.";
    } catch (err) {
      projectError = err instanceof Error ? err.message : String(err);
    } finally {
      savingProject = false;
    }
  }

  async function handleCreateTerminal(projectId: string) {
    terminalError = null;
    try {
      await terminalService.createTab(projectId);
    } catch (err) {
      terminalError = err instanceof Error ? err.message : String(err);
    }
  }
</script>

<main>
  <Shell title="Projectlib Desktop">
    {#if error}
      <p>Failed to reach backend: {error}</p>
    {:else}
      <p>{ping.message}</p>
    {/if}

    <section class="panel">
      <h2>Git runtime</h2>
      {#if gitInfo}
        <dl>
          <div>
            <dt>Detected git</dt>
            <dd>{gitInfo.detectedPath ?? "Unknown"}</dd>
          </div>
          <div>
            <dt>Configured override</dt>
            <dd>{gitInfo.configuredPath ?? "None"}</dd>
          </div>
          <div>
            <dt>Effective command</dt>
            <dd>{gitInfo.effectivePath ?? "Unavailable"}</dd>
          </div>
          <div>
            <dt>Uses wrapper</dt>
            <dd>{gitInfo.usesWrapper ? "Yes" : "No"}</dd>
          </div>
        </dl>
      {:else}
        <p>Detecting git command…</p>
      {/if}
    </section>

    <section class="panel">
      <h2>Repository tools</h2>
      <label>
        Repository path
        <input bind:value={repoPath} placeholder="/path/to/repository" />
      </label>
      <div class="actions">
        <button type="button" on:click={detectRepository}>Check for .git</button>
        <button type="button" on:click={loadStatus}>Load status</button>
        <button
          type="button"
          class="primary"
          on:click={saveRepositoryAsProject}
          disabled={!repoInfo?.isRepository || !repoInfo?.worktreeRoot || savingProject}
        >
          {savingProject ? "Saving…" : "Save as project"}
        </button>
      </div>
      {#if repoError}
        <p class="error">{repoError}</p>
      {/if}
      {#if repoInfo}
        <div class="result">
          <p>
            {#if repoInfo.isRepository}
              Repository detected at {repoInfo.worktreeRoot}
            {:else}
              No repository found
            {/if}
          </p>
        </div>
      {/if}
      {#if statusError}
        <p class="error">{statusError}</p>
      {/if}
      {#if status}
        <div class="result">
          <p>
            Branch:
            {#if status.branch}{status.branch}{:else}detached{/if}
            {#if status.upstream}
              (tracking {status.upstream})
            {/if}
          </p>
          <p>Ahead {status.ahead} / Behind {status.behind}</p>
          <p>{status.isClean ? "Working tree clean" : "Changes present"}</p>
        </div>
      {/if}
      {#if projectMessage}
        <p class="note">{projectMessage}</p>
      {/if}
    </section>

    <section class="panel">
      <h2>Project terminals</h2>
      {#if projectError}
        <p class="error">{projectError}</p>
      {:else}
        <TerminalTabs
          {projects}
          bind:selectedProjectId
          creatingError={terminalError}
          onCreateTab={handleCreateTerminal}
        />
      {/if}
    </section>
  </Shell>
</main>

<style>
  p {
    margin: 0;
    font-size: 1.1rem;
  }

  .panel {
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
    padding: 1.25rem;
    border: 1px solid color-mix(in srgb, currentColor 25%, transparent);
    border-radius: 0.75rem;
    background: color-mix(in srgb, var(--terminal-bg) 35%, transparent);
    backdrop-filter: blur(6px);
  }

  dl {
    display: grid;
    grid-template-columns: minmax(0, 12rem) 1fr;
    gap: 0.5rem 1rem;
    margin: 0;
  }

  dt {
    font-weight: 600;
  }

  dd {
    margin: 0;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  input {
    padding: 0.55rem 0.75rem;
    border: 1px solid color-mix(in srgb, currentColor 25%, transparent);
    border-radius: 0.5rem;
    font: inherit;
    background: color-mix(in srgb, var(--terminal-bg) 80%, transparent);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  button {
    padding: 0.5rem 1rem;
    border-radius: 0.6rem;
    border: 1px solid color-mix(in srgb, currentColor 25%, transparent);
    background: transparent;
    cursor: pointer;
    font: inherit;
    transition: background 0.2s ease, border-color 0.2s ease, color 0.2s ease;
  }

  button:hover:not(:disabled) {
    background: color-mix(in srgb, var(--terminal-bg) 85%, transparent);
    border-color: color-mix(in srgb, currentColor 35%, transparent);
  }

  button:disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }

  button.primary {
    background: color-mix(in srgb, var(--terminal-cursor) 80%, transparent);
    color: #fff;
    border-color: color-mix(in srgb, var(--terminal-cursor) 60%, transparent);
  }

  button.primary:hover:not(:disabled) {
    background: color-mix(in srgb, var(--terminal-cursor) 90%, transparent);
  }

  .error {
    color: #f87171;
  }

  .note {
    color: color-mix(in srgb, var(--terminal-cursor) 80%, currentColor 20%);
    font-size: 0.95rem;
  }

  .result {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
</style>
