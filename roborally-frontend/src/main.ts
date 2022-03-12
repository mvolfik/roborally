import App from "./App.svelte";
import "./global.css";
import init, * as game_simulation from "../game_simulation";

init().then((internals) => {
  console.log({ game_simulation, internals });
  new App({
    target: document.getElementById("app"),
  });
});
