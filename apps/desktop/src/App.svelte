<script lang="ts">
  import { onMount } from "svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import TitleBar from "./components/TitleBar.svelte";
  import { app } from "./lib/store.svelte";
  import Cleaner from "./pages/Cleaner.svelte";
  import Optimize from "./pages/Optimize.svelte";
  import Overview from "./pages/Overview.svelte";
  import Settings from "./pages/Settings.svelte";

  onMount(() => {
    app.start();
    const mq = window.matchMedia?.("(prefers-color-scheme: dark)");
    const onChange = () => app.applyTheme();
    mq?.addEventListener("change", onChange);
    return () => mq?.removeEventListener("change", onChange);
  });
</script>

<div class="shell">
  <TitleBar />
  <div class="body">
    <Sidebar />
    <main class="scroll">
      {#if app.page === "overview"}<Overview />
      {:else if app.page === "cleaner"}<Cleaner />
      {:else if app.page === "optimize"}<Optimize />
      {:else}<Settings />{/if}
    </main>
  </div>
</div>

<style>
  .shell { height: 100%; display: flex; flex-direction: column; }
  .body { flex: 1; display: flex; min-height: 0; }
  main { flex: 1; min-width: 0; }
</style>
