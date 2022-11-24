<script lang="ts">
  import { onMount } from "svelte";
  import { cardExamples } from "./cardExamples";
  import { DEFAULT_CARDS, NEW_CARD } from "./defaultCards";
  import Dialog from "./Dialog.svelte";

  import Game from "./Game.svelte";
  import Map from "./Map.svelte";
  import { fetchMap } from "./utils";

  type CardDefinition = {
    asset: string;
    code: string;
    count: number;
    name: string;
  };
  let state:
    | { state: "disconnected" }
    | {
        state: "creatingGame";
        chosenMap: string | undefined;
        players_n: number;
        name: string;
        round_registers: number;
        draw_cards: number;
        card_pack: {
          again_count: number;
          cards: CardDefinition[];
        };
      }
    | {
        state: "choosingSeat";
        game_name: string;
        map_name: string;
        seats: Array<string | null>;
        chosenSeat: number | undefined;
        name: string;
        cards_assets_names: [string, string][];
        round_registers: number;
      }
    | {
        state: "inGame";
        game_name: string;
        map_name: string;
        seat: number;
        name: string;
        cards_assets_names: [string, string][];
        player_count: number;
        round_registers: number;
      } = {
    state: "disconnected",
  };

  /** Promise which should resolve to the list of games
   *
   * When this promise isn't resolved yet, a loading state is showed.
   * Therefore, this should be updated in 2 ways:
   * - if user expects an explicit refresh (after map creation or on refresh button click)
   *   - in that case, set this to a promise immediately
   * - silently, in background (periodic refresh)
   *   - create a separate promise, wait for it to resolve, and only then update the value (and thus the list)
   */
  let games_promise = refresh_game_list();
  let previewedMap = undefined;
  let editingPack:
    | undefined
    | {
        again_count: number;
        cards: CardDefinition[];
      } = undefined;

  async function refresh_game_list(): Promise<
    {
      seats: (string | null)[];
      name: string;
      map_name: string;
      cards_assets_names: [string, string][];
      card_pack_size: number;
      round_registers: number;
      draw_cards: number;
    }[]
  > {
    const r = await fetch("/api/list-games");
    return await r.json();
  }

  async function fetchMaps(): Promise<string[]> {
    try {
      const r = await fetch("/api/list-maps");
      return await r.json();
    } catch (e) {
      alert(`Error loading available maps: ${e}. Please try again`);
      state = { state: "disconnected" };
      return [];
    }
  }

  async function handleCreateGame() {
    if (state.state !== "creatingGame") return;

    const r = await fetch("/api/new-game", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        map_name: state.chosenMap,
        name: state.name,
        player_count: state.players_n,
        card_definitions: state.card_pack.cards,
        round_registers: state.round_registers,
        draw_cards: state.draw_cards,
        again_count: state.card_pack.again_count,
      }),
    });
    let text = await r.text();
    if (r.status === 201) {
      alert(`Game created`);
      games_promise = refresh_game_list();
      state = { state: "disconnected" };
    } else {
      alert(`Error: ${text}`);
    }
  }

  onMount(() => {
    const interval = setInterval(() => {
      if (state.state !== "inGame") {
        let promise = refresh_game_list();
        promise.finally(() => (games_promise = promise));
      }
    }, 10000);
    return () => clearInterval(interval);
  });

  const fetchMapWithErrorHandler = (mapName: string) =>
    fetchMap(mapName).catch((e) => {
      alert(`Error loading map preview: ${e}. Please try again`);
      previewedMap = undefined;
      throw e;
    });
</script>

{#if state.state === "inGame"}
  <Game
    game_name={state.game_name}
    name={state.name}
    seat={state.seat}
    map_name={state.map_name}
    cards_assets_names={state.cards_assets_names}
    player_count={state.player_count}
    round_registers={state.round_registers}
    on:disconnect={() => {
      state = { state: "disconnected" };
      games_promise = refresh_game_list();
      // refresh again soon after, the seat this player just left should be empty then
      setTimeout(
        () =>
          refresh_game_list().then(
            (val) => (games_promise = Promise.resolve(val))
          ),
        1000
      );
    }}
  />
{:else}
  <div class="menu-wrapper">
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
            card_pack: DEFAULT_CARDS,
            round_registers: 5,
            draw_cards: 9,
          })}>Create new game</button
      >
    </p>
    <table>
      <thead>
        <tr>
          <th>Name</th>
          <th>Players (Connected / Total)</th>
          <th>Map</th>
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
            {#each games as game (game.name)}
              <tr>
                <td>{game.name}</td>
                <td
                  >{game.seats.filter((x) => x !== null).length}/{game.seats
                    .length}</td
                >
                <td>{game.map_name}</td>
                <td>
                  <button
                    on:click={() => {
                      state = {
                        state: "choosingSeat",
                        game_name: game.name,
                        map_name: game.map_name,
                        seats: game.seats,
                        cards_assets_names: game.cards_assets_names,
                        chosenSeat: undefined,
                        name: "",
                        round_registers: game.round_registers,
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
    <div class="intro-info">
      <p>
        This is a remake of the board game RoboRally, originally published by
        <a target="_blank" rel="noopener" href="https://wizards.com"
          >Wizards of the Coast</a
        >, a game studio now owned by Hasbro. First version of this project was
        created in March &ndash; April 2022 as a high school graduation project
        by
        <a target="_blank" href="https://mvolfik.github.io">Matěj Volf</a>.
        Since then, occasional updates are made, usually (again) as school work.
        A few more links:
      </p>
      <ul>
        <li>
          <a
            target="_blank"
            rel="noopener"
            href="https://en.wikipedia.org/wiki/RoboRally"
            >RoboRally on Wikipedia</a
          >
        </li>
        <li>
          <a
            target="_blank"
            rel="noopener"
            href="https://www.hasbro.com/common/documents/60D52426B94D40B98A9E78EE4DD8BF94/3EA9626BCAE94683B6184BD7EA3F1779.pdf"
            >Original rules PDF from Hasbro website</a
          >
        </li>
        <li>
          Source code (<a href="/source-code.tar.gz">.tar.gz</a>,
          <a href="/source-code.zip">.zip</a>,
          <a
            target="_blank"
            rel="noopener"
            href="https://github.com/mvolfik/roborally">GitHub</a
          >)
        </li>
      </ul>
      <details>
        <summary>Differences from original game</summary>
        <ul>
          <li>
            There's no energy cubes and powerups &ndash; powerups break many
            assumptions about movement rules and checks for installed player
            powerups would need to be all over the code-base.
          </li>
          <li>
            There's only 1 reboot token for the whole map.
            <ul>
              <li>
                It wouldn't be that difficult to implement multiple reboot
                tokens, with each of them having a covered area. However,
                there's no notion of "one game map is made of multiple boards"
                in the web version, so the reboot tokens would need some other
                way of showing these areas.
              </li>
            </ul>
          </li>
          <li>Spawn points are assigned randomly during game creation.</li>
          <li>
            The backend used to be written in a bit of a spaghetti-code and it
            would be very difficult to allow players to make any choices during
            the card execution phase. Therefore the following choices have been
            made:
            <ul>
              <li>
                There's only 1 type of damage cards (SPAM cards). However,
                there's unlimited amount of them.
              </li>
              <li>
                Players can't choose which way to turn their robot after reboot.
                Instead, the robots automatically face in the direction of the
                reboot token.
              </li>
            </ul>
            After I rewrote the backend in November 2022, this would be quite easy
            to add, but I didn't yet find a compelling reason to actually do it.
          </li>
          <li>
            While programming a board game usually requires handling many more
            edge-cases that the original rules didn't think of, there's one
            weird situation that the Roborally rules specify, but I decided to
            implement in a different way: if you program "Again" after a SPAM
            card, according to original rules, you should <i>again</i> draw a random
            card and execute it. In my implementation, the SPAM card is replaced
            in the register that it was programmed in, and again just re-executes
            that action
          </li>
        </ul>
      </details>
      <details>
        <summary>Custom cards programming documentation</summary>
        <p>
          When you create a new game, by default the same set of movement cards
          as in the original game will be used. But if you want to make your
          game more interesting, you can create your own cards - click the <em
            >Edit</em
          >
          button in the <em>New game</em> dialog window. The cards are
          programmed with the
          <a target="_blank" rel="noopener" href="https://rhai.rs/"
            >Rhai scripting language</a
          >, and all the default cards are implemented with it, so you can use
          these as a starting point and practical examples. Here is a
          documentation of what should the card scripts look like.
        </p>
        <p>
          Your cards script must provide a function <code
            >execute(player_i, register_i) &lbrace;...&rbrace;</code
          >. Both arguments are integers (and obviously index from zero).
          <code>register_i</code> is probably rarely useful, but if you want your
          card to do different things on different turns, there you go. The function
          shouldn't return any value.
        </p>
        <p>
          The following variables are available to the <code>execute</code> function:
        </p>
        <ul>
          <li>
            <code>const GAME: Game</code>
            <p>
              Current state of the game. You can't modify it directly, only
              using different methods, see below.
            </p>
          </li>
          <li>
            <code>const PLAYER_COUNT: int</code>
            <p>Number of players in this game.</p>
          </li>
          <li>
            <code>const ROUND_REGISTERS: int</code>
            <p>
              Number of registers (cards) programmed and executed each round.
            </p>
          </li>
          <li>
            <code>const MAP_WIDTH: int</code>
            <p>Width of the game map.</p>
          </li>
          <li>
            <code>const MAP_HEIGHT: int</code>
            <p>Height of the game map.</p>
          </li>
        </ul>
        <p>
          Note that all these variables are in a custom scope, so they are <em
            >&ldquo;global&rdquo;</em
          >
          (you can access them directly, without anything like
          <code>this.GAME</code>), but are only in scope for the
          <code>execute</code> function, so if needed, you will need to pass them
          as arguments to any other functions you declare.
        </p>
        <p>The following functions and operations are available:</p>
        <ul>
          <li>
            <code>
              Game.move_player_in_direction(player_i: int, direction:
              PlayerDirection): MoveResult
            </code>
            <p>
              Moves the player by 1 tile in the given direction, eventually
              pushing any players in the way and executing reboots immediately
              after.
            </p>
          </li>
          <li>
            <code>
              Game.force_move_player_to(player_i: int, pos: MapPosition,
              pushing_direction: PlayerDirection): MoveResult
            </code>
            <p>
              Moves the player to the given position, eventually pushing away a
              player already standing on that tile in the given direction, and
              executing reboots immediately after.
            </p>
          </li>
          <li>
            <code>Game.get_player_at_position(pos: MapPosition): () | int</code>
            <p>
              Returns the number of player at given position, or the unit type
              if there's none.
            </p>
          </li>
          <li>
            <code>Game.get_player_position(player_i: int): MapPosition</code>
            <p>Returns the position of given player.</p>
          </li>
          <li>
            <code>
              Game.get_player_direction(player_i: int): PlayerDirection
            </code>
            <p>Returns the direction of given player.</p>
          </li>
          <li>
            <code>
              Game.set_player_direction(player_i: int, direction:
              PlayerDirection)
            </code>
            <p>
              Sets the direction of given player. (Note that
              <code>PlayerDirection</code>
              is sensitive to rotation by multiples of 360 degrees, so always reuse
              player's direction and only rotate it a few times.)
            </p>
          </li>
          <li>
            <code>Game.is_void_at(position: MapPosition): bool</code>
            <p>
              Returns true if the given position is a void tile (hole in map or
              out of bounds).
            </p>
          </li>
          <li>
            <code>(getter) MoveResult.moved: bool</code>
            <p>If the player moved.</p>
          </li>
          <li>
            <code>(getter) MoveResult.rebooted: bool</code>
            <p>If the player rebooted as a result of this move.</p>
          </li>
          <li>
            <code>direction_up(): PlayerDirection</code>
            <p>Creates a new direction upwards.</p>
          </li>
          <li>
            <code>(operator) PlayerDirection + int: PlayerDirection</code>
            <p>
              Creates a new direction rotated N times 90 degrees clockwise:
              <code>direction_up() + 3</code> returns a direction to the left.
            </p>
          </li>
          <li>
            <code>(operator) PlayerDirection - int: PlayerDirection</code>
            <p>Same as above but counter-clockwise.</p>
          </li>
          <li>
            <code>(getter) PlayerDirection.direction: int</code>
            <p>
              Returns a number indicating the direction: Up&rarr;0,
              Right&rarr;1, Down&rarr;2, Left&rarr;3.
            </p>
          </li>
          <li>
            <code>position_from_xy(x: int, y: int): MapPosition</code>
            <p>
              Creates a new position object from <code>x</code> and
              <code>y</code> coordinates (top left corner tile of the game map
              has coordinates <code>0,0</code>; <code>y</code> increases down).
            </p>
          </li>
          <li>
            <code>(getter) MapPosition.x: int</code>
            <p>
              Gets the <code>x</code> component of the position coordinates.
            </p>
          </li>
          <li>
            <code>(getter) MapPosition.y: int</code>
            <p>
              Gets the <code>y</code> component of the position coordinates.
            </p>
          </li>
          <li>
            <code>(operator) MapPosition + PlayerDirection: MapPosition</code>
            <p>
              Creates a new position moved by 1 tile in the given direction.
            </p>
          </li>
        </ul>
        <p>
          This API obviously doesn't allow you to do everything – you can't
          interact with game tiles, shoot, atomically move multiple players at
          once or replace or execute other cards. I'm not opposed to adding
          this, I just didn't (yet?) find the time for it. The game is
          open-source, so pull requests are welcome :)
        </p>
        <p>
          Here's a few examples of interesting card scripts (click on the code
          block to copy it to your clipboard):
        </p>
        <ul>
          {#each cardExamples as { name, code }}
            <li>
              <p>
                {name}
              </p>
              <div
                class="code"
                on:click={function () {
                  const selection = window.getSelection();
                  selection.removeAllRanges();
                  selection.selectAllChildren(this);
                  navigator.clipboard.writeText(selection.toString());
                }}
              >
                {@html code
                  .split("\n")
                  .map((l) =>
                    l.replace(/^ */, (m) =>
                      Array(m.length).fill("&nbsp;").join("")
                    )
                  )
                  .join("<br />")}
              </div>
            </li>
          {/each}
          <li>
            <a
              href="https://github.com/mvolfik/roborally/blob/master/roborally-frontend/src/cardExamples.ts"
              target="_blank"
              rel="noopener">Contribute yours!</a
            >
          </li>
        </ul>
      </details>
    </div>
  </div>
{/if}

{#if state.state === "choosingSeat"}
  <Dialog
    on:close={() => (state = { state: "disconnected" })}
    title="Connect to game {state.game_name}"
  >
    <form
      on:submit|preventDefault={() => {
        if (state.state !== "choosingSeat") return;
        state = {
          state: "inGame",
          name: state.name,
          game_name: state.game_name,
          seat: state.chosenSeat,
          map_name: state.map_name,
          cards_assets_names: state.cards_assets_names,
          player_count: state.seats.length,
          round_registers: state.round_registers,
        };
      }}
    >
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
        type="submit">Connect</button
      >
    </form>
  </Dialog>
{/if}

{#if state.state === "creatingGame"}
  <Dialog
    on:close={() => (state = { state: "disconnected" })}
    title="Create new game"
  >
    <form on:submit|preventDefault={handleCreateGame}>
      <label>
        Game name: <input type="text" bind:value={state.name} />
      </label>
      <label>
        Number of players: <input
          type="number"
          min="1"
          step="1"
          bind:value={state.players_n}
        />
      </label>
      <label>
        Draw cards: <input
          type="number"
          min="1"
          step="1"
          bind:value={state.draw_cards}
        />
      </label>
      <label>
        Registers executed per round: <input
          type="number"
          min="1"
          step="1"
          bind:value={state.round_registers}
        />
      </label>
      {#await fetchMaps()}
        <span style:grid-column="1/-1" style:text-align="center"
          >Please wait, loading available maps</span
        >
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
          type="button"
          style:grid-column="2"
          on:click={() => {
            if (state.state === "creatingGame") previewedMap = state.chosenMap;
          }}>Preview map</button
        >
      {/await}
      <span style:grid-column="1/-1">
        Starting pack of cards: {state.card_pack.cards.reduce(
          (sum, next) => next.count + sum,
          0
        ) + state.card_pack.again_count} cards
        <button
          type="button"
          on:click={() => {
            if (state.state === "creatingGame")
              editingPack = JSON.parse(JSON.stringify(state.card_pack));
          }}
        >
          Edit
        </button>
      </span>
      <button
        type="submit"
        disabled={state.chosenMap === undefined || state.name.length === 0}
      >
        Create
      </button>
    </form>
  </Dialog>
{/if}

{#if previewedMap !== undefined}
  <Dialog
    on:close={() => (previewedMap = undefined)}
    title="Map preview: {previewedMap}"
  >
    {#await fetchMapWithErrorHandler(previewedMap)}
      <span>Please wait, loading map preview</span>
    {:then map}
      {@const artificialPlayers = map.get_artificial_spawn_state()}
      <div class="map-preview">
        <Map
          map={map.assets}
          players={artificialPlayers}
          player_names={artificialPlayers.map(() => "Spawnpoint")}
          animationDuration={700}
        />
      </div>
    {/await}
  </Dialog>
{/if}

{#if editingPack !== undefined}
  <Dialog
    on:close={() => {
      if (state.state === "creatingGame") state.card_pack = editingPack;
      editingPack = undefined;
    }}
    title="Edit starting card pack"
  >
    <div class="card-pack-editor">
      <span class="header" style:grid-row="3" style:align-self="center"
        >Code</span
      >
      <span class="header" style:grid-column="2">Name</span>
      <span class="header" style:grid-column="2">Asset URL</span>
      <span class="header" style:grid-column="2">Count</span>

      <textarea
        disabled
        style:text-align="center"
        style:resize="none"
        style:height="2.5rem"
        style:min-height="0"
        style:grid-row="4/6">&lt;Special card: Again&gt;</textarea
      >
      <input
        type="url"
        disabled
        value={new URL("/assets/again.png", window.location.href).toString()}
      />
      <button disabled class="x" style:grid-row="4/6">X</button>
      <input
        type="number"
        bind:value={editingPack.again_count}
        min="0"
        step="1"
      />
      {#each editingPack.cards as _, i}
        <textarea bind:value={editingPack.cards[i].code} />
        <input type="text" bind:value={editingPack.cards[i].name} />
        <button
          class="x"
          on:click={() => {
            editingPack.cards.splice(i, 1);
            editingPack.cards = editingPack.cards;
          }}>X</button
        >
        <input type="url" bind:value={editingPack.cards[i].asset} />
        <input
          type="number"
          min="0"
          step="1"
          bind:value={editingPack.cards[i].count}
        />
      {/each}

      <button
        style:grid-column="1/-1"
        on:click={() => {
          editingPack.cards.push(NEW_CARD);
          editingPack.cards = editingPack.cards;
        }}
      >
        Add card
      </button>
    </div>
    <div class="card-pack-buttons">
      <button
        on:click={() => {
          editingPack = undefined;
        }}
      >
        Cancel
      </button>
      <button
        on:click={() => {
          editingPack = JSON.parse(JSON.stringify(DEFAULT_CARDS));
        }}
      >
        Reset to default game pack
      </button>
      <button
        on:click={() => {
          if (state.state === "creatingGame") state.card_pack = editingPack;
          editingPack = undefined;
        }}
      >
        Save
      </button>
    </div>
  </Dialog>
{/if}

<style>
  :global(html),
  :global(body),
  :global(#app) {
    height: 100%;
    width: 100%;
    margin: 0;
    padding: 0;
    overflow: hidden;
  }
  :global(button:not(:disabled)) {
    cursor: pointer;
  }
  .menu-wrapper {
    padding: 1rem;
    box-sizing: border-box;
    height: 100%;
    width: 100%;
    overflow: auto;
  }
  p {
    margin: 0.5rem 0;
  }
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
    font-size: 0.8rem;
  }
  td[colspan="4"] {
    text-align: center;
  }

  form {
    display: grid;
    grid-gap: 0.5rem;
    grid-template-columns: auto 1fr;
    width: min(30rem, 95vw - 4rem);
  }

  form label {
    display: contents;
  }

  form button[type="submit"] {
    grid-column: 1/-1;
  }

  select {
    max-width: 50vw;
  }

  .map-preview {
    width: calc(95vw - 4rem);
    height: calc(95vh - 5rem);
  }

  .intro-info {
    margin-top: 5rem;
  }
  .intro-info p {
    max-width: 60rem;
  }

  .card-pack-editor {
    display: grid;
    grid-template-columns: 1fr min(25vw, 17rem) auto;
    width: min(70rem, 95vw - 4rem);
    gap: 0.3rem;
    margin-bottom: 0.3rem;
    overflow-y: scroll;
    max-height: calc(95vh - 6.5rem);
    padding-right: 1rem;
  }

  .card-pack-editor .header {
    font-weight: bold;
    text-align: center;
  }

  .card-pack-editor textarea {
    grid-column: 1;
    grid-row: span 4;
    resize: vertical;
    min-height: 4rem;
    height: 4rem;
    white-space: pre;
  }

  .card-pack-editor input {
    grid-column: 2;
    min-width: 0;
  }

  .card-pack-editor button.x {
    grid-column: 3;
    grid-row: span 3;
    height: 1.8rem;
    width: 1.8rem;
    align-self: center;
  }

  .card-pack-buttons {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 0.3rem;
  }

  details {
    margin-left: 2rem;
  }

  summary {
    cursor: pointer;
    margin: 1rem 0 0.3rem -2rem;
  }

  details > summary + ul:last-child {
    padding: 0;
  }

  li > code:first-child,
  li > p + div.code:last-child {
    font-family: "Ubuntu Mono", monospace;
    font-size: 1.1em;
    white-space: nowrap;
    width: 100%;
    overflow-x: auto;
    display: block;
    padding-bottom: 0.5rem;
  }
  li > code + p:last-child {
    margin: 0.3rem 0 1rem 1rem;
  }
  li > p + div.code:last-child {
    background-color: #eee;
    padding: 0.5rem;
    font-size: 1em;
  }
</style>
