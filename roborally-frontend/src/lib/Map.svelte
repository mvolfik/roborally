<script lang="ts">
  import type { GameMap } from "../../game_simulation";
  import Zoomable from "svelte-layer-zoomable";

  const assets = import.meta.globEager("../assets/textures/*.png", {
    assert: { type: "url" },
  }) as Record<string, { default: string }>;

  function getAsset(name: string): string {
    return (
      assets["../assets/textures/" + name]?.default ??
      (console.warn(`Unknown asset ${name}, using floor as fallback`),
      assets["../assets/textures/floor.png"].default)
    );
  }

  export let map: GameMap;
  export let hovered = undefined;
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
            {#each map.get_assets_at(x, y) as asset}
              {@const assetUri = getAsset(asset.uri)}
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
    </div>
  </Zoomable>
</div>

<style>
  div.outer {
    position: absolute;
    height: 100vh;
    width: 100vw;
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
    transform-origin: center;
  }
  div.hoverMarker {
    height: 100%;
    width: 100%;
    outline: 2px dashed red;
    outline-offset: -2px;
  }
</style>
