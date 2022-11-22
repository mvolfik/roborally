<script lang="ts">
  import "@fontsource/vt323";
  import {
    Direction,
    type AssetMap,
    type PlayerPublicState,
    type Position,
  } from "frontend-wasm";
  import robot from "./assets/robot.png?url";
  import Zoomable from "svelte-layer-zoomable";
  import { getTexture } from "./utils";

  export let map: AssetMap;
  export let players: Array<PlayerPublicState>;
  export let player_names: Array<string>;
  export let animationDuration: number;

  let innerDiv: HTMLDivElement;

  /** Run a bullet animation */
  export function handleBullet(
    from: Position,
    to: Position,
    direction: Direction,
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

  export function handleCheckpointVisited(player_i: number) {
    const el = innerDiv.querySelectorAll(".robot")[player_i];
    el.animate(
      [
        { scale: 1, offset: 0 },
        { scale: 1.8, offset: 0.5 },
        { scale: 1, offset: 1 },
      ],
      {
        duration: animationDuration,
        easing: "linear",
      }
    );
  }

  export function handleAttemptedMove(player_i: number, direction: Direction) {
    const el = innerDiv.querySelectorAll(".robot")[player_i];
    const translate = (
      direction === Direction.Up
        ? "0 -X"
        : direction === Direction.Right
        ? "X 0"
        : direction === Direction.Down
        ? "0 X"
        : "-X 0"
    )
      .replace("-X", "calc(var(--tile-size) / -2)")
      .replace("X", "calc(var(--tile-size) / 2)");
    el.animate(
      [
        { translate: "0 0", offset: 0 },
        { translate, offset: 0.5 },
        { translate: "0 0", offset: 1 },
      ],
      {
        duration: animationDuration,
        easing: "ease-out",
        id: `player-bounce-${player_i}`,
      }
    );
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
              {@const assetUri = asset.is_text
                ? undefined
                : getTexture(asset.value)}
              {#if asset.is_text}
                <span style={asset.style}>{asset.value}</span>
              {:else if assetUri !== undefined}
                <img style={asset.style} src={assetUri} alt="" />
              {/if}
            {/each}
          </div>
        {/each}
      {/each}
      {#each players as player, i}
        {@const pos = player.position}
        <div
          class="robot"
          style:--x={pos.x}
          style:--y={pos.y}
          class:hidden={player.is_hidden}
        >
          <img src={robot} alt="Robot" style={player.style} />
          {#if player_names[i] !== undefined}
            <div>{player_names[i]}</div>
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
    max-width: 6.1rem;
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
  div.tile > span {
    font-family: "VT323";
  }
</style>
