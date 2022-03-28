<script lang="ts">
  import {
    AssetMap,
    GamePhase,
    MessageProcessor,
    PlayerGameStateView,
  } from "../../frontend-wasm";
  import { createEventDispatcher, onMount } from "svelte";
  import { fly } from "svelte/transition";

  import { writable, Writable } from "svelte/store";
  import Map from "./Map.svelte";
  import Programmer from "./Programmer.svelte";

  export let game_id: string;
  export let name: string;
  export let seat: number;

  let connection: WebSocket;
  let stateStore: Writable<PlayerGameStateView> = writable(undefined);
  let map: AssetMap;
  function handleProgrammingDone(e: CustomEvent) {
    connection.send(
      MessageProcessor.create_program_cards_message(...e.detail).buffer
    );
  }

  function mainHandler(e: MessageEvent) {
    MessageProcessor.handle_message(
      new Uint8Array(e.data),
      stateStore.set,
      alert
    );
  }

  onMount(() => {
    connection = new WebSocket(
      `${window.location.protocol.replace("http", "ws")}//${
        window.location.host
      }/websocket/game/${game_id}`
    );
    connection.binaryType = "arraybuffer";
    connection.onclose = () => {
      eventSource("disconnect");
    };
    connection.addEventListener(
      "message",
      (e) => {
        try {
          map = MessageProcessor.expect_init_message(
            new Uint8Array(e.data),
            stateStore.set
          );
          connection.addEventListener("message", mainHandler);
        } catch (e) {
          alert(e);
        }
      },
      { once: true }
    );

    connection.addEventListener(
      "open",
      () =>
        connection.send(
          MessageProcessor.create_init_message(name, seat).buffer
        ),
      { once: true }
    );

    return () => {
      connection.close();
      connection.removeEventListener("message", mainHandler);
    };
  });

  let eventSource = createEventDispatcher();
</script>

<div class="outer">
  {#if map === undefined}
    <p>Loading...</p>
  {:else}
    <div class="map">
      <Map {map} {stateStore} />
    </div>
    {#if $stateStore.phase === GamePhase.Programming}
      <div class="programmer" transition:fly={{ y: 200 }}>
        <Programmer
          {seat}
          initialCards={[...Array($stateStore.hand_len)].map((_, i) =>
            $stateStore.get_hand_card(i)
          )}
          on:programmingDone={handleProgrammingDone}
        />
      </div>
    {/if}
    <div class="player-infoboxes">
      {#each Array($stateStore.players) as _, player_i}
        {@const player = $stateStore.get_player(player_i)}
        {@const name = $stateStore.get_player(player_i).name}
        {#if $stateStore.phase !== GamePhase.Moving}
          <div style:--player-i={player_i} transition:fly={{ x: 100 }}>
            {#if player_i === seat}
              <div class="name self">You</div>
            {:else if name === undefined}
              <div class="name disconnected">
                Seat {player_i + 1} (disconnected)
              </div>
            {:else}
              <div class="name">{name}</div>
            {/if}
            <div>
              Ready: <div
                class="ready-indicator"
                class:ready={$stateStore.is_ready_programming(player_i)}
              />
            </div>
          </div>
        {/if}
      {/each}
    </div>
  {/if}
</div>

<style>
  .outer {
    position: relative;
    overflow: clip;
  }
  .map {
    height: 100vh;
  }
  .programmer {
    --programmer-width: 90vw;
    left: 5vw;
    position: absolute;
    bottom: 0;
  }

  .player-infoboxes {
    color: #eee;
    position: absolute;
    right: 0;
    top: 2rem;
  }

  .player-infoboxes > div {
    margin-top: 2rem;
    background-color: hsla(
      calc(3.979rad + var(--player-i) * 0.9rad),
      93%,
      22%,
      0.62
    );
    padding: 1.5rem;
    border-radius: 5px 0 0 5px;
  }

  .name.self {
    color: rgb(15, 187, 230);
  }
  .name.disconnected {
    color: rgb(255, 95, 37);
  }

  .ready-indicator {
    width: 1rem;
    background-color: red;
    display: inline flow-root;
    height: 1rem;
    vertical-align: middle;
  }

  .ready-indicator.ready {
    background-color: green;
  }
</style>
