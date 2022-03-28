<script lang="ts">
  import Game from "./lib/Game.svelte";

  let state:
    | { state: "disconnected" }
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

  async function create_new_game() {
    const name = prompt("Name of your game:");
    let players: number;
    while (true) {
      players = parseInt(prompt("Number of players:"));
      if (Number.isNaN(players)) {
        alert("Need a number");
      } else if (players <= 0) {
        alert("Number must be greater than 0");
      } else {
        break;
      }
    }
    const map_name = prompt("Map name:");
    const r = await fetch("/api/new-game", {
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      body: new URLSearchParams({ players, map_name, name }),
    });
    let text = await r.text();
    if (r.status === 201) {
      alert(`Success. Game id: ${text}`);
    } else {
      alert(`Error: ${text}`);
    }
    games_promise = refresh_game_list();
  }
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
  <p><button on:click={create_new_game}>Create new game</button></p>
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
    <div class="seatChooser">
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

  .seatChooser {
    display: grid;
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    background-color: #fff;
    padding: 2rem;
    grid-gap: 0.5rem;
  }
</style>
