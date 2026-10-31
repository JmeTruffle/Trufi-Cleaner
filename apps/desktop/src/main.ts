import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";
import { isMac } from "./lib/os";

// macOS gets Liquid Glass surfaces (see the end of app.css).
if (isMac) document.documentElement.classList.add("os-mac");

const app = mount(App, { target: document.getElementById("app")! });

export default app;
