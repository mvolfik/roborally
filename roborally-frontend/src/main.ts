import * as frontend_wasm from "frontend-wasm";
import App from "./App.svelte";

frontend_wasm.default().then(
  (internals) => {
    window.frontend_wasm = frontend_wasm;
    window.internals = internals;
    const target = document.getElementById("app");
    target.innerHTML = "";
    new App({
      target,
    });
  },
  (e) => {
    document.getElementById(
      "app"
    ).innerText = `Loading failed with the following error: ${e}. Refresh the page to try again`;
  }
);
