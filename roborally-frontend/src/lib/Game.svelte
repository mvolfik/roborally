<script lang="ts">
  import {
    AssetMap,
    GamePhase,
    MessageProcessor,
    PlayerGameStateView,
  } from "frontend-wasm";
  import { createEventDispatcher, onMount } from "svelte";
  import { fly } from "svelte/transition";

  import { writable, Writable } from "svelte/store";
  import Map from "./Map.svelte";
  import Programmer from "./Programmer.svelte";
  import { getCardAsset } from "./utils";

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
          map = MessageProcessor.expect_init_message(new Uint8Array(e.data));
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
  $: console.log($stateStore);
</script>

<div
  class="outer"
  style:--seat-color={`hsla(${3.979 + seat * 0.9}rad, 93%, 22%, 0.62)`}
>
  {#if map === undefined || $stateStore === undefined}
    <p>Loading...</p>
  {:else}
    <div class="map">
      <Map {map} {stateStore} />
    </div>
    {#key $stateStore.phase === GamePhase.Moving}
      <div class="phase-indicator" transition:fly={{ x: -200 }}>
        <div>
          Current phase: {$stateStore.phase === GamePhase.Moving
            ? "Moving"
            : "Programming"}
        </div>
        {#if $stateStore.phase === GamePhase.Moving}
          <div>
            Current register: {$stateStore.moving_phase_register_number + 1}
          </div>
          <div
            class="register-move-phase-indicator"
            style:--register-phase={$stateStore.moving_phase_register_phase}
          >
            <span class="marker">&gt;</span>
            <span>Programmed cards</span>
            <span>Fast belts</span>
            <span>Slow belts</span>
            <span>Push panels</span>
            <span>Rotations</span>
            <span>Lasers</span>
            <span>Robot lasers</span>
            <span>Checkpoints</span>
          </div>
        {/if}
      </div>
    {/key}
    {#if $stateStore.phase === GamePhase.Programming}
      <div class="programmer" transition:fly={{ y: 200 }}>
        <Programmer
          initialCards={[...Array($stateStore.hand_len)].map((_, i) =>
            $stateStore.get_hand_card(i)
          )}
          on:programmingDone={handleProgrammingDone}
        />
      </div>
    {:else}
      <div class="my-registers" transition:fly={{ y: 200 }}>
        <span>Your programmed cards</span>
        {#each [...Array(5)].map( (_, i) => $stateStore.get_my_register_card(i) ) as card}
          <img src={getCardAsset(card.asset_name)} alt="" />
        {/each}
      </div>
    {/if}
    {#key $stateStore.phase == GamePhase.Moving}
      <div class="player-infoboxes" transition:fly={{ x: 100 }}>
        {#each [...Array($stateStore.players)].map( (_, i) => $stateStore.get_player(i) ) as player, player_i}
          {@const name = player.name}
          <div style:--player-i={player_i}>
            {#if player_i === seat}
              <div class="name self">You</div>
            {:else if name === undefined}
              <div class="name disconnected">
                Seat {player_i + 1} (disconnected)
              </div>
            {:else}
              <div class="name">{name}</div>
            {/if}
            {#if $stateStore.phase == GamePhase.Moving}
              <img
                src={getCardAsset(
                  $stateStore.get_player_card_for_current_register(player_i)
                    .asset_name
                )}
                alt="Card"
              />
            {:else}
              <div>
                Ready: <div
                  class="ready-indicator"
                  class:ready={$stateStore.is_ready_programming(player_i)}
                />
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {/key}
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
    top: 0;
  }

  .player-infoboxes > div {
    margin-top: 40px;
    background-color: hsla(
      calc(3.979rad + var(--player-i) * 0.9rad),
      93%,
      22%,
      0.62
    );
    padding: 20px;
    border-radius: 5px 0 0 5px;
  }

  .player-infoboxes img {
    height: 80px;
    margin-top: 6px;
    border-radius: 4px;
  }

  .name.self {
    color: rgb(15, 187, 230);
  }
  .name.disconnected {
    color: rgb(255, 95, 37);
  }

  .ready-indicator {
    width: 1em;
    background-color: red;
    display: inline flow-root;
    height: 1em;
    vertical-align: middle;
  }

  .ready-indicator.ready {
    background-color: green;
  }

  .phase-indicator {
    color: #eee;
    position: absolute;
    left: 40px;
    top: 0;
    background-color: var(--seat-color);
    padding: 15px;
    font-size: 0.9em;
    border-radius: 0 0 5px 5px;
  }

  .register-move-phase-indicator {
    display: grid;
  }
  .register-move-phase-indicator > span:not(.marker) {
    grid-column: 2;
  }
  .marker {
    grid-row: var(--register-phase);
  }

  .my-registers {
    color: #eee;
    position: absolute;
    bottom: 0;
    left: 100px;
    background-color: var(--seat-color);
    padding: 15px;
    font-size: 0.9em;
    border-radius: 5px 5px 0 0;
    display: grid;
    column-gap: 20px;
    grid-template-columns: auto auto auto auto auto;
  }
  .my-registers span {
    grid-column: 1/-1;
    text-align: center;
    margin-bottom: 10px;
    font-size: 1.3em;
  }
  .my-registers > img {
    border-radius: 8px;
    width: 80px;
  }
</style>
