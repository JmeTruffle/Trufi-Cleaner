<script lang="ts">
  import { onMount } from "svelte";
  import { windowControls } from "../lib/api";
  import Icon from "./Icon.svelte";
  import Logo from "./Logo.svelte";

  let controls = $state<Awaited<ReturnType<typeof windowControls>>>(null);
  onMount(async () => (controls = await windowControls()));
</script>

<header class="titlebar" data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <Logo size={20} />
    <span data-tauri-drag-region>Trufi Cleaner</span>
  </div>
  {#if controls}
    <div class="controls">
      <button aria-label="Свернуть" onclick={controls.minimize}><Icon name="min" size={14} /></button>
      <button aria-label="Развернуть" onclick={controls.toggleMaximize}><Icon name="max" size={12} /></button>
      <button aria-label="Закрыть" class="close" onclick={controls.close}><Icon name="close" size={14} /></button>
    </div>
  {/if}
</header>

<style>
  .titlebar {
    height: 38px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-left: 16px;
    flex: none;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 9px;
    font-size: 13px;
    font-weight: 700;
    color: var(--text);
  }
  .controls { display: flex; height: 100%; }
  .controls button {
    width: 46px;
    height: 100%;
    display: grid;
    place-items: center;
    color: var(--text-2);
  }
  .controls button:hover { background: var(--surface-hover); color: var(--text); }
  .controls .close:hover { background: #e81123; color: #fff; }
</style>
