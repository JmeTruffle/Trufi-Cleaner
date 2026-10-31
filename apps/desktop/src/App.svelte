<script lang="ts">
  import { onMount } from "svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import TitleBar from "./components/TitleBar.svelte";
  import UpdateBanner from "./components/UpdateBanner.svelte";
  import { app } from "./lib/store.svelte";
  import Cleaner from "./pages/Cleaner.svelte";
  import Drivers from "./pages/Drivers.svelte";
  import Optimize from "./pages/Optimize.svelte";
  import Overview from "./pages/Overview.svelte";
  import Settings from "./pages/Settings.svelte";

  onMount(() => app.start());
</script>

<div class="shell">
  <TitleBar />
  <UpdateBanner />
  <div class="body">
    <Sidebar />
    <main class="scroll">
      {#if app.page === "overview"}<Overview />
      {:else if app.page === "cleaner"}<Cleaner />
      {:else if app.page === "optimize"}<Optimize />
      {:else if app.page === "drivers"}<Drivers />
      {:else}<Settings />{/if}
    </main>
  </div>
</div>

<style>
  .shell {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--sky);
  }
  .body { flex: 1; display: flex; min-height: 0; }
  main { flex: 1; min-width: 0; }
</style>
