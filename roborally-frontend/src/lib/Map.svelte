<script lang="ts">
  import type {
    AssetMap,
    PlayerGameStateView,
    PlayerPublicStateWrapper,
    Position,
  } from "frontend-wasm";
  import robot from "../assets/robot.png?url";
  import Zoomable from "svelte-layer-zoomable";
  import { readable, Readable, Writable } from "svelte/store";
  import { getTexture } from "./utils";

  export let map: AssetMap;
  export let hovered = undefined;
  export let stateStore: Readable<{
    players: number;
    get_player: (number) => PlayerPublicStateWrapper;
    process_animations(process_bullet_closure: typeof processBullet): void;
  }> = readable({
    players: 0,
    process_animations() {},
    get_player(): any {},
  });

  let innerDiv: HTMLDivElement;

  function processBullet(from: Position, to: Position) {
    const bullet = document.createElement("img");
    bullet.src = new URL("../assets/bullet.png", import.meta.url);
    bullet.style = `
      position: absolute;
      left: ${(from.x + 0.5) * 64}px;
      top: ${(from.y + 0.5) * 64}px;
      transform: translate(-50%, -50%);
      transition-property: left, top;
      transition: 1s linear;`;
    innerDiv.appendChild(bullet);
    bullet.addEventListener("transitionend", () => {
      innerDiv.removeChild(bullet);
    });
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        bullet.style.left = `${(to.x + 0.5) * 64}px`;
        bullet.style.top = `${(to.y + 0.5) * 64}px`;
      });
    });
  }

  $: $stateStore.process_animations(processBullet);

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
  <Zoomable maxScale={2}>
    <!-- svelte-ignore a11y-mouse-events-have-key-events -->
    <div
      class="grid"
      on:mouseleave={() => (hovered = undefined)}
      bind:this={innerDiv}
    >
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
            {#each map.get(x, y).into_jsarray() as asset}
              {@const assetUri = getTexture(asset.uri)}
              {#if assetUri !== undefined}
                <img
                  style={asset.style}
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
      {#each [...Array($stateStore.players)].map( (_, i) => $stateStore.get_player(i) ) as player}
        {@const pos = player.position}
        <img
          src={robot}
          alt="Robot"
          class="robot"
          style={player.style}
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
    pointer-events: none;
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
