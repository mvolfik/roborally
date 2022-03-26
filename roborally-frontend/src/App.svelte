<script lang="ts">
  import Game from "./lib/Game.svelte";
  import {
    ConnectInfo,
    connect_to_game,
    GameConnectionResult,
  } from "../frontend-wasm";

  let connection: GameConnectionResult | undefined = undefined;
  let games_promise = refresh_game_list();

  async function refresh_game_list(): Promise<
    {
      id: string;
      players_connected: number;
      players_n: number;
      name: string;
    }[]
  > {
    const r = await fetch("/list-games");
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
    const r = await fetch("/new-game", {
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      body: new URLSearchParams({ players, map_name, name }),
    });
    let text = await r.text();
    if (r.status === 200) {
      alert(`Success. Game id: ${text}`);
    } else {
      alert(`Error: ${text}`);
    }
    games_promise = refresh_game_list();
  }

  async function connect(id: string) {
    connection = await connect_to_game(
      new ConnectInfo(BigInt(id), 0, prompt("Your name:"))
    );
    console.log(connection);
  }
</script>

{#if connection !== undefined}
  <Game
    state_store={connection.store}
    program={connection.program}
    asset_map={connection.assets_map}
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
        <th>
          <span
            style:text-decoration="dotted underline"
            title="Connected / total">Players</span
          >
        </th>
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
              <td>{game.players_connected}/{game.players_n}</td>
              <td>
                <button on:click={() => connect(game.id)}>Connect</button>
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

<style>
  table {
    width: 100%;
    border-collapse: collapse;
  }
  table tr > * {
    padding: 0.3rem 1rem;
  }
  thead > tr:last-child > * {
    border-bottom: 1px solid #111;
  }
  td[colspan="4"] {
    text-align: center;
  }
</style>
