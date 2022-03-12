import App from "./App.svelte";
import "./global.css";
import init, * as game_simulation from "../game_simulation";

init().then(() => {
  console.log(game_simulation);
  new App({
    target: document.getElementById("app"),
  });
});
