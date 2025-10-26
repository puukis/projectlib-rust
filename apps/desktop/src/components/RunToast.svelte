<script lang="ts">
  import { createEventDispatcher } from "svelte";

  export let message: string;
  export let actionLabel: string | undefined;

  const dispatch = createEventDispatcher();

  function handleAction() {
    dispatch("action");
  }

  function handleDismiss() {
    dispatch("dismiss");
  }
</script>

<div class="toast" role="alert">
  <span>{message}</span>
  <div class="actions">
    {#if actionLabel}
      <button type="button" class="link" on:click={handleAction}>{actionLabel}</button>
    {/if}
    <button type="button" class="icon" aria-label="Dismiss" on:click={handleDismiss}>
      ×
    </button>
  </div>
</div>

<style>
  .toast {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.65rem 0.9rem;
    border-radius: 0.75rem;
    background: color-mix(in srgb, #ef4444 25%, var(--terminal-bg) 85%);
    color: white;
    box-shadow: 0 16px 32px rgba(239, 68, 68, 0.35);
    min-width: 18rem;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-left: auto;
  }

  button {
    border: none;
    background: transparent;
    color: inherit;
    cursor: pointer;
    font: inherit;
  }

  button.link {
    text-decoration: underline;
    font-weight: 600;
  }

  button.icon {
    font-size: 1.1rem;
    line-height: 1;
  }

  button:focus-visible {
    outline: none;
    text-decoration: underline;
  }
</style>
