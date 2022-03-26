<script lang="ts">
  import type {
    AssetMap,
    GameConnectionResult,
    PlayerGameStateView,
  } from "frontend-wasm/roborally_frontend_wasm";

  import type { Writable } from "svelte/store";
  import Map from "./Map.svelte";

  export let state_store: Writable<PlayerGameStateView>;
  export let program: typeof GameConnectionResult.prototype.program;
  export let asset_map: AssetMap;

  $: console.log($state_store);
</script>

<Map map={asset_map} />
