<script lang="ts">
  import Map from "./lib/Map.svelte";
  import mapString from "../../maps/test.csv?raw";
  import { GameMap } from "../game_simulation";

  let gameMap = GameMap.parse(mapString);
</script>

<Map map={gameMap} />
