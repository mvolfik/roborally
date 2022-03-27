import App from "./App.svelte";
import "./global.css";
import * as frontend_wasm from "../frontend-wasm";

frontend_wasm.default().then((internals) => {
  window.frontend_wasm = frontend_wasm;
  window.internals = internals;
  new App({
    target: document.getElementById("app"),
  });
});
