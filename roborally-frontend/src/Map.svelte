<script lang="ts">
  import type { AssetMap, PlayerGameStateView, Position } from "frontend-wasm";
  import robot from "./assets/robot.png?url";
  import Zoomable from "svelte-layer-zoomable";
  import { getTexture } from "./utils";

  export let map: AssetMap;
  export let state: Pick<PlayerGameStateView, "players">;

  let innerDiv: HTMLDivElement;

  /** Run a bullet animation */
  export function handleBullet(
    from: Position,
    to: Position,
    direction: 0 | 1 | 2 | 3,
    isFromTank: boolean
  ) {
    let fromX = from.x;
    let fromY = from.y;
    if (!isFromTank) {
      // apart from making the shots look a bit better, this also fixes a bug when robot stands
      // directly on the tile with the laser, and therefore no transition happens
      if (direction === 0) {
        // shooting up -> start a bit more down
        fromY += 0.4;
      } else if (direction === 1) {
        // right
        fromX -= 0.4;
      } else if (direction === 2) {
        // down
        fromY -= 0.4;
      } else {
        // left
        fromX += 0.4;
      }
    }

    requestAnimationFrame(() => {
      const bullet = document.createElement("img");
      bullet.src = new URL("./assets/bullet.png", import.meta.url).toString();
      bullet.style.cssText = `
        position: absolute;
        --tile-x: ${fromX};
        --tile-y: ${fromY};
        left: calc((var(--tile-x) + 0.5) * var(--tile-size));
        top: calc((var(--tile-y) + 0.5) * var(--tile-size));
        transform: translate(-50%, -50%);
        transition-property: left, top;
        transition: var(--animation-duration) linear;`;

      innerDiv.appendChild(bullet);
      bullet.addEventListener("transitionend", () => {
        innerDiv.removeChild(bullet);
      });
      requestAnimationFrame(() => {
        bullet.style.setProperty("--tile-x", to.x.toString());
        bullet.style.setProperty("--tile-y", to.y.toString());
      });
    });
  }
</script>

<div class="outer">
  <Zoomable maxScale={2}>
    <!-- svelte-ignore a11y-mouse-events-have-key-events -->
    <div class="grid" bind:this={innerDiv}>
      {#each Array(map.height) as _, y}
        {#each Array(map.width) as _, x}
          <!-- svelte-ignore a11y-mouse-events-have-key-events -->
          <div class="tile" style:grid-column={x + 1} style:grid-row={y + 1}>
            {#each map.get(x, y).into_jsarray() as asset}
              {@const assetUri = getTexture(asset.uri)}
              {#if assetUri !== undefined}
                <img style={asset.style} src={assetUri} alt="" />
              {/if}
            {/each}
          </div>
        {/each}
      {/each}
      {#each state.players as player}
        {@const pos = player.position}
        <div
          class="robot"
          style:--x={pos.x}
          style:--y={pos.y}
          class:hidden={player.is_hidden}
        >
          <img src={robot} alt="Robot" style={player.style} />
          {#if player.name !== undefined}
            <div>{player.name}</div>
          {/if}
        </div>
      {/each}
    </div>
  </Zoomable>
</div>

<style>
  .robot {
    top: calc(var(--tile-size) * var(--y));
    left: calc(var(--tile-size) * var(--x));
    pointer-events: none;
  }
  .robot.hidden {
    transform: scale(0);
  }
  .robot,
  .robot img {
    position: absolute;
    transition: all var(--animation-duration) ease-in-out;
    transform-origin: calc(var(--tile-size) / 2) calc(var(--tile-size) / 2);
  }
  .robot > div {
    position: absolute;
    width: max-content;
    background-color: #666b;
    color: white;
    padding: 0.1em 0.4em;
    border-radius: 0.2em;
    top: calc(var(--tile-size) * -0.12);
    max-width: 6rem;
    overflow: hidden;
    text-overflow: ellipsis;

    /* centering: left moves top left corner of child relative to parent size, translate moves relative to child size */
    left: calc(var(--tile-size) / 2);
    transform: translateX(-50%);
  }
  div.outer {
    height: 100%;
    width: 100%;
    background-image: radial-gradient(#222, #666);
    --tile-size: 64px;
  }
  div.grid {
    display: grid;
    grid-auto-rows: var(--tile-size);
    grid-auto-columns: var(--tile-size);
  }
  div.tile {
    height: 100%;
    width: 100%;
    position: relative;
    overflow: clip;
  }
  div.tile > * {
    position: absolute;
    transform-origin: calc(var(--tile-size) / 2) calc(var(--tile-size) / 2);
  }
</style>
