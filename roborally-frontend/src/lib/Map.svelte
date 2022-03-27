<script lang="ts">
  import type {
    AssetMap,
    PlayerGameStateView,
    PlayerPublicStateWrapper,
  } from "frontend-wasm/roborally_frontend_wasm";
  import robot from "../assets/robot.png?url";
  import Zoomable from "svelte-layer-zoomable";
  import type { get, Writable } from "svelte/store";
  import { getTexture } from "./utils";

  export let map: AssetMap;
  export let hovered = undefined;
  export let stateStore: Writable<PlayerGameStateView>;

  let players: Map<string, Array<PlayerPublicStateWrapper>> = new Map();
  $: {
    players = new Map();
    const playersN = $stateStore.players;
    for (let i = 0; i < playersN; i++) {
      const player = $stateStore.get_player(i);
      const pos = player.position;
      const key = `${pos.x},${pos.y}`;
      if (!players.has(key)) {
        players.set(key, []);
      }
      players.get(key).push(player);
    }
  }
</script>

<div class="outer">
  <Zoomable>
    <!-- svelte-ignore a11y-mouse-events-have-key-events -->
    <div class="grid" on:mouseleave={() => (hovered = undefined)}>
      {#each Array(map.height) as _, y}
        {#each Array(map.width) as _, x}
          <!-- svelte-ignore a11y-mouse-events-have-key-events -->
          <div
            class="tile"
            style:grid-column={x + 1}
            style:grid-row={y + 1}
            on:mouseover={(e) => {
              if (e.buttons) return;
              hovered = { x, y };
            }}
          >
            {#each map.get(x, y)?.to_jsarray() ?? [] as asset}
              {@const assetUri = getTexture(asset.uri)}
              {#if assetUri !== undefined}
                <img
                  style:transform={asset.transform_string}
                  src={assetUri}
                  alt=""
                />
              {/if}
            {/each}
            {#if hovered && hovered.x === x && hovered.y === y}
              <div class="hoverMarker" />
            {/if}
          </div>
        {/each}
      {/each}
      {#each Array($stateStore.players) as _, i}
        {@const player = $stateStore.get_player(i)}
        {@const pos = player.position}
        <img
          src={robot}
          alt="Robot"
          class="robot"
          style:transform={player.transform_string}
          style:filter={player.filter_string}
          style:--x={pos.x}
          style:--y={pos.y}
        />
      {/each}
    </div>
  </Zoomable>
</div>

<style>
  .robot {
    position: absolute;
    top: calc(64px * var(--y));
    left: calc(64px * var(--x));
    transition: all 1s ease-in-out;
  }
  div.outer {
    height: 100%;
    width: 100%;
    background-image: radial-gradient(#222, #666);
  }
  div.grid {
    display: grid;
    grid-auto-rows: 64px;
    grid-auto-columns: 64px;
  }
  div.tile {
    height: 100%;
    width: 100%;
    position: relative;
    overflow: clip;
  }
  div.tile > * {
    position: absolute;
    transform-origin: 32px 32px;
  }
  div.hoverMarker {
    height: 100%;
    width: 100%;
    outline: 2px dashed red;
    outline-offset: -2px;
  }
</style>
