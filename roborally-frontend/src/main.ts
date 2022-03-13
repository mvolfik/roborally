import MapComponent from "./lib/Map.svelte";
import "./global.css";
import init, * as game_simulation from "../game_simulation";
import mapString from "../../maps/test.csv?raw";

init().then((internals) => {
  console.log({ game_simulation, internals });
  let { map, warnings } = game_simulation.GameMap.parse(mapString);
  if (warnings) {
    console.warn(warnings);
  }
  new MapComponent({
    target: document.getElementById("app"),
    props: {
      map,
    },
  });
});
