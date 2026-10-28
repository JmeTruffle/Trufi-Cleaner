<script lang="ts">
  import { onMount } from "svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import TitleBar from "./components/TitleBar.svelte";
  import UpdateBanner from "./components/UpdateBanner.svelte";
  import { app } from "./lib/store.svelte";
  import Cleaner from "./pages/Cleaner.svelte";
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
      {:else}<Settings />{/if}
    </main>
  </div>
</div>

<style>
  .shell {
    height: 100%;
    display: flex;
    flex-direction: column;
    background:
      radial-gradient(60% 45% at 85% 0%, rgba(110, 240, 210, 0.28), transparent 70%),
      radial-gradient(50% 40% at 0% 10%, rgba(140, 200, 255, 0.35), transparent 70%),
      linear-gradient(180deg, var(--sky-top) 0%, var(--sky-mid) 45%, var(--sky-bottom) 100%);
  }
  .body { flex: 1; display: flex; min-height: 0; }
  main { flex: 1; min-width: 0; }
</style>
