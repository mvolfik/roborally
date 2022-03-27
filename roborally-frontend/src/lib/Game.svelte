<script lang="ts">
  import {
    AssetMap,
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
    {#if $stateStore.is_programming}
      <div class="programmer" transition:fly={{y: 200}}>
        <Programmer
          initialCards={[...Array($stateStore.hand_len)].map((_, i) =>
            $stateStore.get_hand_card(i)
          )}
          on:programmingDone={handleProgrammingDone}
        />
      </div>
    {/if}
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
</style>
