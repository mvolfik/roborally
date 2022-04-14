<script lang="ts">
  import { createEventDispatcher } from "svelte";

  const eventSource = createEventDispatcher();
  export let closeLabel = "Close";
  export let title: string;
</script>

<div class="backdrop" on:click|self={() => eventSource("close")}>
  <div class="dialog">
    <div class="header">
      <p>{title}</p>
      <p><button on:click={() => eventSource("close")}>{closeLabel}</button></p>
    </div>
    <slot />
  </div>
</div>

<style>
  .backdrop {
    background-image: radial-gradient(#222a, #666a);
    position: fixed;
    height: 100vh;
    width: 100vw;
    top: 0;
    left: 0;
  }

  .dialog {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    background-color: #fff;
    padding: 0.7rem 2rem 2rem;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    border-bottom: 1px solid #999;
    padding-bottom: 0.3rem;
    margin: 0 -0.5rem 0.5rem;
  }
  .header > p {
    margin: 0;
  }
</style>
