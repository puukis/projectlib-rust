<script lang="ts">
  import { createEventDispatcher, onMount } from "svelte";

  export let projectName: string;
  export let initialCommand = "";
  export let initialArgs = "";
  export let initialEnv = "";
  export let initialCwd = "";
  export let remember = true;
  export let mode: "run" | "edit" = "run";

  const dispatch = createEventDispatcher();

  let command = initialCommand;
  let argsText = initialArgs;
  let envText = initialEnv;
  let cwd = initialCwd;
  let rememberChoice = remember;
  let modalElement: HTMLDivElement | null = null;

  function close() {
    dispatch("cancel");
  }

  function handleSubmit(event: Event) {
    event.preventDefault();
    dispatch("submit", { command, argsText, envText, cwd, remember: rememberChoice });
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.stopPropagation();
      close();
    }
  }

  onMount(() => {
    modalElement?.focus();
  });
</script>

<div class="backdrop" role="presentation" on:click={close}></div>
<div
  class="modal"
  role="dialog"
  aria-modal="true"
  aria-label={`Run configuration for ${projectName}`}
  tabindex="-1"
  bind:this={modalElement}
  on:keydown={handleKeydown}
>
  <header>
    <h2>{mode === "run" ? `Run ${projectName}` : `Edit run for ${projectName}`}</h2>
  </header>
  <form on:submit={handleSubmit}>
    <label>
      Command
      <input bind:value={command} placeholder="pnpm" required />
    </label>
    <label>
      Arguments (one per line)
      <textarea bind:value={argsText} rows="3" placeholder="dev"></textarea>
    </label>
    <label>
      Environment (KEY=VALUE per line)
      <textarea bind:value={envText} rows="3" placeholder="PORT=3000"></textarea>
    </label>
    <label>
      Working directory
      <input bind:value={cwd} placeholder="/path/to/project" />
    </label>
    {#if mode !== "edit"}
      <label class="remember">
        <input type="checkbox" bind:checked={rememberChoice} /> Remember as default
      </label>
    {/if}
    <footer>
      <button type="button" on:click={close}>Cancel</button>
      <button type="submit" class="primary">{mode === "run" ? "Save & Run" : "Save"}</button>
    </footer>
  </form>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(2px);
    z-index: 1200;
  }

  .modal {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: min(420px, 92vw);
    background: color-mix(in srgb, var(--terminal-bg) 90%, transparent);
    border-radius: 1rem;
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    z-index: 1201;
    outline: none;
    box-shadow: 0 24px 48px rgba(0, 0, 0, 0.35);
  }

  header h2 {
    margin: 0;
    font-size: 1.35rem;
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    font-weight: 600;
  }

  input,
  textarea {
    font: inherit;
    padding: 0.55rem 0.75rem;
    border-radius: 0.65rem;
    border: 1px solid color-mix(in srgb, currentColor 25%, transparent);
    background: color-mix(in srgb, var(--terminal-bg) 75%, transparent);
    color: inherit;
  }

  textarea {
    resize: vertical;
    min-height: 5rem;
  }

  .remember {
    flex-direction: row;
    align-items: center;
    gap: 0.5rem;
    font-weight: 500;
  }

  footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }

  footer button {
    border-radius: 999px;
    padding: 0.45rem 1rem;
    border: 1px solid color-mix(in srgb, currentColor 20%, transparent);
    background: transparent;
    cursor: pointer;
    font: inherit;
  }

  footer button.primary {
    background: #22c55e;
    border-color: rgba(34, 197, 94, 0.25);
    color: white;
  }

  footer button.primary:hover {
    background: #16a34a;
  }

  footer button:focus-visible {
    outline: none;
    box-shadow: 0 0 0 3px rgba(34, 197, 94, 0.25);
  }
</style>
