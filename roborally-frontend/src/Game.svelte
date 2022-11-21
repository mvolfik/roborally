<script lang="ts">
  import {
    AnimationItem,
    AssetMap,
    create_program_cards_message,
    GeneralState,
    parse_message,
    ProgrammingState,
    ServerMessageType,
  } from "frontend-wasm";
  import { createEventDispatcher, onMount } from "svelte";

  import { writable } from "svelte/store";
  import Map from "./Map.svelte";
  import Programmer from "./Programmer.svelte";
  import { fetchMap } from "./utils";
  import Collapsible from "./Collapsible.svelte";

  export let game_name: string;
  export let name: string;
  export let seat: number;
  export let map_name: string;
  export let cards_assets_names: [string, string][];
  export let player_count: number;
  export let round_registers: number;

  let log = "";
  let connection: WebSocket;
  let map: AssetMap;
  let mapComponent: Map;

  let generalState: GeneralState;

  /** If playing a sequence of state updates in the moving phase, they are all stored here */
  let stateArray: Array<AnimationItem> = [];
  /** If no sequence is playing, simply the current state. Otherwise, the game
   * should continue to this state after the sequence finishes */
  let programmingState: ProgrammingState;
  let nextProgrammingState: ProgrammingState | undefined;

  /**
   * number => index in stateArray
   * undefined => currentSimpleState
   */
  let stateIndicator: number | undefined;

  /** The actual current state, as selected by `stateIndicator`; will always have state, not just animations */
  let currentAnimationState: AnimationItem | undefined;

  let autoplay = true;
  let automaticPlaybackDelay = 700;
  /** This is only updated each time the animation "steps", to prevent changing
   * the duration in the middle of a running animation */
  let currentAnimationDuration = automaticPlaybackDelay;

  function handleProgrammingDone(e: CustomEvent<number[]>) {
    connection.send(
      create_program_cards_message(new Uint8Array(e.detail)).buffer
    );
  }

  let timeoutHandle: number | undefined;

  /** Move a step forward in the stateArray
   *
   * This function also clears any timeout in case it was called from manual
   * button click to prevent two updates in quick succession.
   * If autoplay is on, a timeout is started to schedule the next update
   */
  function step() {
    clearTimeout(timeoutHandle);
    timeoutHandle = undefined;

    if (stateIndicator === undefined) return;

    if (stateIndicator === stateArray.length - 1) {
      gamePhaseExpandedStore.set(true);
      return;
    }

    currentAnimationDuration = automaticPlaybackDelay;
    const item = stateArray[++stateIndicator];
    item.process_animations(
      mapComponent.handleBullet,
      mapComponent.handleCheckpointVisited
    );

    if (item.has_state) {
      currentAnimationState = item;
    }

    scheduleNextStep();
  }

  function handleMessage(e: MessageEvent) {
    let msg = parse_message(new Uint8Array(e.data));
    if (msg.typ === ServerMessageType.Notice) {
      alert(msg.notice);
    } else if (msg.typ === ServerMessageType.GameLog) {
      log += msg.game_log;
    } else if (msg.typ === ServerMessageType.GeneralState) {
      generalState = msg.general_state;
    } else if (msg.typ === ServerMessageType.ProgrammingState) {
      if (currentAnimationState === undefined) {
        programmingState = msg.programming_state;
      } else {
        nextProgrammingState = msg.programming_state;
      }
    } else if (msg.typ === ServerMessageType.AnimatedState) {
      stateArray = [...stateArray, msg.animated_state];

      if (stateIndicator === undefined) {
        stateIndicator = 0;
        currentAnimationState = stateArray[0];
      }

      scheduleNextStep();
    } else {
      alert("Unknown message type");
    }
  }

  onMount(() => {
    fetchMap(map_name).then((m) => (map = m.assets));
    connection = new WebSocket(
      `${window.location.protocol.replace("http", "ws")}//${
        window.location.host
      }/websocket/game?${new URLSearchParams({
        game_name,
        name,
        seat: seat.toString(),
      }).toString()}`
    );
    connection.binaryType = "arraybuffer";
    connection.onclose = (e) => {
      if (e.code === 1000) {
        alert(`Server closed connection: ${e.reason}`);
      } else {
        if (disconnect !== undefined)
          alert(`Server abruptly closed connection`);
      }
      disconnect?.();
    };
    connection.addEventListener("message", handleMessage);

    return () => {
      connection.close();
      connection.removeEventListener("message", handleMessage);
    };
  });

  let eventSource = createEventDispatcher();
  let disconnect = () => {
    eventSource("disconnect");
    // prevent repeated disconnect event
    // (that can happen when client initiates disconnect and later a close frame comes from server to fully close the socket)
    disconnect = undefined;
  };

  enum GamePhase {
    Programming,
    ProgrammingMyselfDone,
    Moving,
  }

  // updated by the reactive block below
  let phase: GamePhase;

  let programmerExpandedStore = writable(true);
  let playersInfoExpandedStore = writable(false);
  let gamePhaseExpandedStore = writable(false);

  $: {
    const newPhase =
      currentAnimationState !== undefined
        ? GamePhase.Moving
        : programmingState?.prepared_cards === undefined
        ? GamePhase.Programming
        : GamePhase.ProgrammingMyselfDone;
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

  function scheduleNextStep() {
    if (autoplay && timeoutHandle === undefined)
      timeoutHandle = window.setTimeout(step, currentAnimationDuration);
  }

  function onAutoplayChange(_: boolean) {
    if (timeoutHandle !== undefined && !autoplay) {
      clearTimeout(timeoutHandle);
      timeoutHandle = undefined;
    } else {
      scheduleNextStep();
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
  {#if map === undefined || generalState === undefined}
    <p style:text-align="center">Connecting...</p>
  {:else}
    <div class="map">
      <Map
        {map}
        players={phase == GamePhase.Moving
          ? currentAnimationState.player_states
          : programmingState.player_states}
        player_names={new Array(player_count)
          .fill(undefined)
          .map((_, i) => generalState.get_player_name(i))}
        bind:this={mapComponent}
      />
    </div>

    <!-- Top panel: phase indicator -->
    <Collapsible
      side="top"
      label="Game phase"
      key={phase === GamePhase.Moving}
      expandedStore={gamePhaseExpandedStore}
    >
      <div style:padding="0.7rem 1rem">
        <p class="phase-simple-text">
          {generalState.status}
        </p>
        {#if phase === GamePhase.Moving}
          <div>
            Register: {currentAnimationState.register + 1}
          </div>
          <div
            class="register-move-phase-indicator"
            style:--register-phase={currentAnimationState.register_phase + 1}
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
        {/if}
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
                currentAnimationState = stateArray[stateIndicator];
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
                currentAnimationState = undefined;
                stateArray = [];
                programmingState = nextProgrammingState;
                nextProgrammingState = undefined;
              }}
              disabled={stateIndicator === undefined ||
                stateIndicator < stateArray.length - 1 ||
                nextProgrammingState === undefined}
              >Continue to next round</button
            >
          </p>
        </div>
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
      {#each phase === GamePhase.Moving ? currentAnimationState.player_states : programmingState.player_states as player, player_i}
        {@const name = generalState.get_player_name(player_i)}
        <div class="player-infobox" style:--player-i={player_i}>
          {#if player_i === seat}
            <div class="name self">
              You ({name})
            </div>
            <button on:click={() => disconnect?.()}>Disconnect</button>
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
            <div class="revealed-cards">
              {#each currentAnimationState.get_revealed_cards(player_i) as card}
                <div>
                  <img
                    src={cards_assets_names[card][0]}
                    alt={cards_assets_names[card][1]}
                  />
                </div>
              {/each}
            </div>
            <div>
              Rebooting: <div
                class="indicator"
                class:true={player.is_rebooting}
              />
            </div>
          {:else}
            <div>
              Ready: <div
                class="indicator"
                class:true={programmingState.ready_players[player_i] === 1}
              />
            </div>
          {/if}
        </div>
      {/each}
    </Collapsible>

    <!-- Bottom panel: programmer interface -->
    <Collapsible
      side="bottom"
      label="Your cards"
      key={phase === GamePhase.Moving}
      expandedStore={programmerExpandedStore}
    >
      {#key [phase, programmingState, currentAnimationState]}
        <Programmer
          initialCards={phase === GamePhase.Moving
            ? []
            : [...programmingState.hand]}
          {round_registers}
          {cards_assets_names}
          selected={phase === GamePhase.Programming
            ? undefined
            : phase === GamePhase.ProgrammingMyselfDone
            ? [...programmingState.prepared_cards]
            : [...currentAnimationState.my_cards]}
          on:programmingDone={handleProgrammingDone}
        />
      {/key}
    </Collapsible>

    <!-- Left panel: rule hints -->
    <Collapsible side="left" label="Game execution log">
      <div
        style:width="min(20rem, 80vw)"
        style:padding="1rem"
        style:font-family="monospaced"
        style:white-space="pre"
      >
        {log}
      </div>
    </Collapsible>
  {/if}
</div>

<style>
  .outer {
    --card-width: 4rem;
    --card-border-radius: 5px;
    --card-margin: 0.5rem;
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

  .player-infobox .revealed-cards img {
    width: var(--card-width);
    height: calc(var(--card-width) * 10 / 7);
    object-fit: cover;
    border-radius: var(--card-border-radius);
  }
  .player-infobox .revealed-cards div {
    margin: 0 calc(var(--card-margin) / 2);
    display: inline-block;
    transition: width 0.5s;
  }
  .player-infobox .revealed-cards div:not(:last-child):not(:hover),
  .player-infobox .revealed-cards div:hover ~ :last-child {
    width: 2rem;
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
</style>
