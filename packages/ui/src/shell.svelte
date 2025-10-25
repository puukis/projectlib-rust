<script lang="ts">
  export let title: string = "Projectlib";
</script>

<section class="shell">
  <h1>{title}</h1>
  <slot />
</section>

<style>
  .shell {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding: 1.5rem;
    border: 1px solid currentColor;
    border-radius: 0.75rem;
  }

  h1 {
    font-size: 1.5rem;
    margin: 0;
  }
</style>
