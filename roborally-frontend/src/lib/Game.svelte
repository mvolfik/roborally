<script lang="ts">
  import {
    AssetMap,
    CardWrapper,
    create_init_message,
    create_program_cards_message,
    GamePhase,
    parse_message,
    PlayerGameStateView,
    StateArrayItem,
  } from "frontend-wasm";
  import { createEventDispatcher, onMount } from "svelte";
  import { fly } from "svelte/transition";

  import { writable } from "svelte/store";
  import Map from "./Map.svelte";
  import Programmer from "./Programmer.svelte";
  import { fetchMap, getCardAsset } from "./utils";
  import Collapsible from "./Collapsible.svelte";

  export let game_name: string;
  export let name: string;
  export let seat: number;
  export let map_name: string;

  let connection: WebSocket;
  let map: AssetMap;
  let mapComponent: Map;

  let stateArray: Array<StateArrayItem> = [];
  let currentSimpleState: PlayerGameStateView;
  /**
   * number => index in stateArray
   * undefined => currentSimpleState
   */
  let stateIndicator: number | undefined;
  let state: PlayerGameStateView;
  let autoplay = true;
  let automaticPlaybackDelay = 700;
  let currentAnimationDuration = automaticPlaybackDelay;

  function handleProgrammingDone(
    e: CustomEvent<
      [CardWrapper, CardWrapper, CardWrapper, CardWrapper, CardWrapper]
    >
  ) {
    connection.send(create_program_cards_message(...e.detail).buffer);
  }

  let timeoutHandle: number | undefined;

  function step() {
    clearTimeout(timeoutHandle);
    timeoutHandle = undefined;

    if (stateIndicator === stateArray.length - 1) {
      gamePhaseExpandedStore.set(true);
    }

    if (
      stateIndicator === undefined ||
      stateIndicator === stateArray.length - 1
    )
      return;

    currentAnimationDuration = automaticPlaybackDelay;
    const item = stateArray[++stateIndicator];
    item.process_animations(mapComponent?.handleBullet ?? (() => {}));

    if (item.has_state) {
      state = item.state;
    }

    if (autoplay) {
      timeoutHandle = setTimeout(step, automaticPlaybackDelay);
    }
  }

  function messageHandler(e: MessageEvent) {
    let msg = parse_message(new Uint8Array(e.data));
    if (typeof msg === "string") {
      alert(msg);
    } else if (Array.isArray(msg)) {
      stateArray = msg;
      stateIndicator = 0;
      state = stateArray[0].state;
      onAutoplayChange(autoplay);
    } else {
      currentSimpleState = msg;
      if (stateIndicator === undefined) {
        state = currentSimpleState;
      }
    }
  }

  onMount(() => {
    fetchMap(map_name).then((m) => (map = m.assets));
    connection = new WebSocket(
      `${window.location.protocol.replace("http", "ws")}//${
        window.location.host
      }/websocket/game?name=${encodeURIComponent(game_name)}`
    );
    connection.binaryType = "arraybuffer";
    connection.onclose = () => {
      disconnect();
    };
    connection.addEventListener("message", messageHandler);
    connection.addEventListener(
      "open",
      () => connection.send(create_init_message(name, seat).buffer),
      { once: true }
    );

    return () => {
      connection.close();
      connection.removeEventListener("message", messageHandler);
    };
  });

  let eventSource = createEventDispatcher();
  let disconnect = () => {
    eventSource("disconnect");
    disconnect = () => {};
  };
  /**
   * This value is later updated by the reactive block below
   */
  let phase: GamePhase;

  let programmerExpandedStore = writable(true);
  let playersInfoExpandedStore = writable(false);
  let gamePhaseExpandedStore = writable(false);

  $: {
    if (state === undefined) break $;
    const newPhase = state.phase;
    if (newPhase !== phase) {
      if (newPhase === GamePhase.Programming) {
        programmerExpandedStore.set(true);
      } else if (newPhase === GamePhase.Moving) {
        gamePhaseExpandedStore.set(true);
      } else if (newPhase === GamePhase.ProgrammingMyselfDone) {
        playersInfoExpandedStore.set(true);
      }
      phase = newPhase;
    }
  }
  $: console.log({ state, stateArray, stateIndicator });

  function onAutoplayChange(newAutoplay: boolean) {
    if (newAutoplay) {
      if (timeoutHandle === undefined) {
        timeoutHandle = setTimeout(step, automaticPlaybackDelay);
      }
    } else {
      if (timeoutHandle !== undefined) {
        clearTimeout(timeoutHandle);
        timeoutHandle = undefined;
      }
    }
  }

  $: onAutoplayChange(autoplay);
</script>

<svelte:window
  on:beforeunload={(e) => {
    // show a confirmation prompt
    e.preventDefault();
  }}
/>
<div
  class="outer"
  style:--seat-color="hsla({3.979 + seat * 0.9}rad, 93%, 22%, 0.62)"
  style:--animation-duration="{currentAnimationDuration}ms"
>
  {#if map === undefined || state === undefined}
    <p style:text-align="center">Connecting...</p>
  {:else}
    <div class="map">
      <Map {map} {state} bind:this={mapComponent} />
    </div>

    <!-- Top panel: phase indicator -->
    <Collapsible
      side="top"
      label="Game phase"
      key={phase === GamePhase.Moving}
      expandedStore={gamePhaseExpandedStore}
    >
      <div style:padding="0.7rem 1rem">
        {#if phase === GamePhase.HasWinner}
          <p class="phase-simple-text">
            Game won by {state.get_winner_name()}
          </p>
        {:else if phase === GamePhase.Moving}
          <div>
            Executing movement register: {state.moving_phase_register_number +
              1}
          </div>
          <div
            class="register-move-phase-indicator"
            style:--register-phase={state.moving_phase_register_phase + 1}
          >
            <span class="marker">&gt;</span>
            <span>Programmed cards</span>
            <span>Fast belts</span>
            <span>Slow belts</span>
            <span>Push panels</span>
            <span>Rotations</span>
            <span>Lasers</span>
            <span>Checkpoints</span>
          </div>
        {:else}
          <p class="phase-simple-text">Get ready for the next round!</p>
        {/if}
        {#if phase !== GamePhase.HasWinner}
          <div class="animation-settings">
            <p>Show player movement:</p>
            <p>
              <label>
                <input type="checkbox" bind:checked={autoplay} />
                Autoplay
              </label>
              <label>
                with delay:
                <input
                  type="number"
                  min="100"
                  max="5000"
                  step="100"
                  size="4"
                  bind:value={automaticPlaybackDelay}
                />
                ms
              </label>
            </p>
            <p>
              <button
                on:click={() => {
                  autoplay = false;
                  do {
                    stateIndicator -= 1;
                  } while (!stateArray[stateIndicator].has_state);
                  state = stateArray[stateIndicator].state;
                }}
                disabled={stateIndicator === undefined || stateIndicator <= 0}
                >Previous</button
              >
              <button
                on:click={step}
                disabled={stateIndicator === undefined ||
                  stateIndicator >= stateArray.length - 1}>Next</button
              >
              <button
                on:click={() => {
                  stateIndicator = undefined;
                  state = currentSimpleState;
                }}
                disabled={stateIndicator === undefined ||
                  stateIndicator < stateArray.length - 1}
                >Continue to next round</button
              >
            </p>
          </div>
        {/if}
      </div>
    </Collapsible>

    <!-- Right panel: player infoboxes -->
    <Collapsible
      side="right"
      label="Players info"
      noBackground
      expandedStore={playersInfoExpandedStore}
      key={phase === GamePhase.Moving}
    >
      {#each [...Array(state.players)].map( (_, i) => state.get_player(i) ) as player, player_i}
        {@const name = player.name}
        <div class="player-infobox" style:--player-i={player_i}>
          {#if player_i === seat}
            <div class="name self">
              You ({name})
            </div>
            <button on:click={() => disconnect()}>Disconnect</button>
          {:else if name === undefined}
            <div class="name disconnected">
              Seat {player_i + 1} (disconnected)
            </div>
          {:else}
            <div class="name">{name}</div>
          {/if}
          <div class="checkpoints">
            Checkpoints
            <div>
              {#each [...Array(map.checkpoints)].map((_, i) => player.checkpoint > i) as checkpoint_reached}
                <div class="indicator" class:true={checkpoint_reached} />
              {/each}
            </div>
          </div>
          {#if phase === GamePhase.Moving}
            <img
              src={getCardAsset(
                state.get_player_card_for_current_register(player_i).asset_name
              )}
              alt="Card"
            />
          {:else if phase !== GamePhase.HasWinner}
            <div>
              Ready: <div
                class="indicator"
                class:true={state.is_ready_programming(player_i)}
              />
            </div>
          {/if}
        </div>
      {/each}
    </Collapsible>

    <!-- Bottom panel: programmer interface -->
    {#if phase !== GamePhase.HasWinner}
      <Collapsible
        side="bottom"
        label="Your cards"
        key={phase === GamePhase.Programming}
        expandedStore={programmerExpandedStore}
      >
        {#if phase === GamePhase.Programming}
          <Programmer
            initialCards={[...Array(state.hand_len)].map((_, i) =>
              state.get_hand_card(i)
            )}
            on:programmingDone={handleProgrammingDone}
          />
        {:else}
          <div class="my-registers" transition:fly={{ y: 200 }}>
            <span>Your programmed cards</span>
            {#each [...Array(5)].map( (_, i) => state.get_my_register_card(i) ) as card}
              <img src={getCardAsset(card.asset_name)} alt="" />
            {/each}
          </div>
        {/if}
      </Collapsible>
    {/if}

    <!-- Left panel: rule hints -->
    <Collapsible side="left" label="Rule hints">
      <div style:max-width="min(20rem, 80vw)" style:padding="1rem">
        <p>Oh, hi there!</p>
        <p>
          Hopefully, one day, in this panel you will find various hints for move
          execution order, tile effects etc
        </p>
      </div>
    </Collapsible>
  {/if}
</div>

<style>
  .outer {
    --card-width: 4rem;
    --card-border-radius: 5px;
    overflow: hidden;
    position: relative;
    height: 100%;
    width: 100%;
  }

  .map {
    height: 100%;
    width: 100%;
  }

  .phase-simple-text {
    margin: 0;
  }

  .animation-settings {
    border-top: 1px solid black;
  }
  .animation-settings > p {
    margin: 0.4rem 0;
  }

  .player-infobox {
    background-color: hsla(
      calc(3.979rad + var(--player-i) * 0.9rad),
      93%,
      22%,
      0.62
    );
    color: white;
    padding: 0.7rem 1rem;
    max-width: 60vw;
  }

  .player-infobox img {
    height: auto;
    width: var(--card-width);
    border-radius: var(--card-border-radius);
  }

  .name {
    text-overflow: ellipsis;
    overflow: clip;
    white-space: nowrap;
  }
  .name.self {
    color: rgb(15, 187, 230);
  }
  .name.disconnected {
    color: rgb(255, 95, 37);
  }

  .checkpoints > div {
    display: inline-flex;
  }

  .indicator {
    display: inline-block;
    width: 1em;
    height: 1.1em;
    box-sizing: border-box;
    margin: 0 0.2em;
    border: 0.2em solid black;
    border-radius: 0.3rem;
    background-color: red;
    vertical-align: text-top;
  }

  .indicator.true {
    background-color: green;
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
    padding: 0.5rem 2rem;
    display: grid;
    column-gap: 0.5rem;
    grid-template-columns: auto auto auto auto auto;
  }
  .my-registers span {
    grid-column: 1/-1;
    margin-bottom: 0.3rem;
    text-align: center;
  }
  .my-registers > img {
    border-radius: var(--card-border-radius);
    width: var(--card-width);
  }
</style>
