<script lang="ts">
  import { onMount } from "svelte";

  import Game from "./lib/Game.svelte";
  import Map from "./lib/Map.svelte";
  import { fetchMap } from "./lib/utils";

  let state:
    | { state: "disconnected" }
    | {
        state: "creatingGame";
        chosenMap: string | undefined;
        players_n: number;
        name: string;
      }
    | {
        state: "choosingSeat";
        game_id: string;
        seats: Array<string | null>;
        chosenSeat: number | undefined;
        name: string;
      }
    | { state: "inGame"; game_id: string; seat: number; name: string } = {
    state: "disconnected",
  };
  let games_promise = refresh_game_list();

  async function refresh_game_list(): Promise<
    {
      id: string;
      seats: Array<string | null>;
      name: string;
    }[]
  > {
    const r = await fetch("/api/list-games");
    return await r.json();
  }

  async function fetchMaps(): Promise<string[]> {
    const r = await fetch("/api/list-maps");
    return await r.json();
  }

  async function handleCreateGame() {
    const r = await fetch("/api/new-game", {
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      body: new URLSearchParams({
        players: state.players_n,
        map_name: state.chosenMap,
        name: state.name,
      }),
    });
    let text = await r.text();
    if (r.status === 201) {
      alert(`Success. Game id: ${text}`);
      games_promise = refresh_game_list();
      state = { state: "disconnected" };
    } else {
      alert(`Error: ${text}`);
    }
  }

  let previewedMap = undefined;

  onMount(() => {
    const interval = setInterval(() => {
      if (state.state !== "inGame") {
        // await the fetch, and then set the promise to a "dummy" immediately-resolved one
        refresh_game_list().then(
          (val) => (games_promise = Promise.resolve(val))
        );
      }
    }, 10000);
    return () => clearInterval(interval);
  });
</script>

{#if state.state === "inGame"}
  <Game
    game_id={state.game_id}
    name={state.name}
    seat={state.seat}
    on:disconnect={() => {
      state = { state: "disconnected" };
      games_promise = refresh_game_list();
    }}
  />
{:else}
  <p>
    <button on:click={() => (games_promise = refresh_game_list())}
      >Refresh list of games</button
    >
  </p>
  <p>
    <button
      on:click={() =>
        (state = {
          state: "creatingGame",
          chosenMap: undefined,
          name: "",
          players_n: 3,
        })}>Create new game</button
    >
  </p>
  <table>
    <thead>
      <tr>
        <th>Name</th>
        <th>Players (Connected / Total)</th>
        <th>Actions</th>
      </tr>
    </thead>
    <tbody>
      {#await games_promise}
        <tr><td colspan="4">Loading...</td></tr>
      {:then games}
        {#if games.length === 0}
          <tr><td colspan="4">No current games</td></tr>
        {:else}
          {#each games as game (game.id)}
            <tr>
              <td>{game.name}</td>
              <td
                >{game.seats.filter((x) => x !== null).length}/{game.seats
                  .length}</td
              >
              <td>
                <button
                  on:click={() => {
                    state = {
                      state: "choosingSeat",
                      game_id: game.id,
                      seats: game.seats,
                      chosenSeat: undefined,
                      name: "",
                    };
                  }}>Connect</button
                >
              </td>
            </tr>
          {/each}
        {/if}
      {:catch}
        <tr
          ><td colspan="4"
            >Loading games failed. Please try refreshing the list</td
          ></tr
        >
      {/await}
    </tbody>
  </table>
{/if}

{#if state.state === "choosingSeat"}
  <div
    class="backdrop"
    on:click|self={() => (state = { state: "disconnected" })}
  >
    <div class="dialog">
      <label>
        Select game seat:
        <select bind:value={state.chosenSeat}>
          <option disabled value={undefined}>&lt;choose&gt;</option>
          {#each state.seats as seat, i}
            <option disabled={seat !== null} value={i}
              >Player {i + 1}{seat !== null
                ? ` (connected: ${seat})`
                : ""}</option
            >
          {/each}
        </select>
      </label>
      <label>
        Your player name: <input type="text" bind:value={state.name} />
      </label>
      <button
        disabled={state.seats[state.chosenSeat] !== null ||
          state.name.length === 0}
        on:click={() => {
          state = {
            state: "inGame",
            name: state.name,
            game_id: state.game_id,
            seat: state.chosenSeat,
          };
        }}>Connect</button
      >
    </div>
  </div>
{/if}

{#if state.state === "creatingGame"}
  <div
    class="backdrop"
    on:click|self={() => (state = { state: "disconnected" })}
  >
    <div class="dialog">
      <label>
        Game name: <input type="text" bind:value={state.name} />
      </label>
      <label>
        Number of players: <input
          type="number"
          min="1"
          max="6"
          step="1"
          bind:value={state.players_n}
        />
      </label>
      {#await fetchMaps()}
        <span>Please wait, loading available maps</span>
      {:then maps}
        <label>
          Select map:
          <select bind:value={state.chosenMap}>
            <option disabled value={undefined}>&lt;choose&gt;</option>
            {#each maps as map}
              <option value={map}>{map}</option>
            {/each}
          </select>
        </label>
        <button
          disabled={state.chosenMap === undefined}
          on:click={() => (previewedMap = state.chosenMap)}>Preview map</button
        >
      {/await}
      <button
        disabled={state.chosenMap === undefined || state.name.length === 0}
        on:click={handleCreateGame}>Create</button
      >
    </div>
  </div>
{/if}

{#if previewedMap !== undefined}
  <div class="backdrop" on:click|self={() => (previewedMap = undefined)}>
    <div class="dialog">
      {#await fetchMap(previewedMap)}
        <span>Please wait, loading map preview</span>
      {:then map}
        <div class="map-preview">
          <Map {map} />
        </div>
      {/await}
    </div>
  </div>
{/if}

<style>
  table {
    width: 100%;
    border-collapse: collapse;
  }
  table tr > * {
    padding: 0.3rem 1rem;
    text-align: center;
  }
  thead > tr:last-child > * {
    border-bottom: 1px solid #111;
  }
  td[colspan="4"] {
    text-align: center;
  }

  .backdrop {
    background-image: radial-gradient(#222a, #666a);
    position: fixed;
    height: 100vh;
    width: 100vw;
    top: 0;
  }

  .dialog {
    display: grid;
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    background-color: #fff;
    padding: 2rem;
    grid-gap: 0.5rem;
  }

  .map-preview {
    width: 70vw;
    height: 70vh;
  }
</style>
