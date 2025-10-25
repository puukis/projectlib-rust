<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { Shell } from "@projectlib/ui";
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

  onMount(async () => {
    try {
      const response = await invoke<string>("ping", { message: "Hello from Svelte" });
      ping = PingSchema.parse({ message: response });
      const info = await invoke("git_path_info");
      gitInfo = GitPathInfoSchema.parse(info);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  });

  async function detectRepository() {
    repoError = null;
    repoInfo = null;
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
    gap: 0.75rem;
    padding: 1rem;
    border: 1px solid currentColor;
    border-radius: 0.5rem;
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
    padding: 0.5rem 0.75rem;
    border: 1px solid currentColor;
    border-radius: 0.5rem;
    font: inherit;
  }

  .actions {
    display: flex;
    gap: 0.5rem;
  }

  button {
    padding: 0.5rem 1rem;
    border-radius: 0.5rem;
    border: 1px solid currentColor;
    background: transparent;
    cursor: pointer;
    font: inherit;
  }

  button:hover {
    background: rgba(0, 0, 0, 0.05);
  }

  .error {
    color: #b91c1c;
  }

  .result {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
</style>
