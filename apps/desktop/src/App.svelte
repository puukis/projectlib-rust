<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { Shell } from "@projectlib/ui";
  import { PingSchema, type Ping } from "@projectlib/shared";

  let ping: Ping = { message: "Pinging backend..." };
  let error: string | null = null;

  onMount(async () => {
    try {
      const response = await invoke<string>("ping", { message: "Hello from Svelte" });
      ping = PingSchema.parse({ message: response });
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  });
</script>

<main>
  <Shell title="Projectlib Desktop">
    {#if error}
      <p>Failed to reach backend: {error}</p>
    {:else}
      <p>{ping.message}</p>
    {/if}
  </Shell>
</main>

<style>
  p {
    margin: 0;
    font-size: 1.1rem;
  }
</style>
