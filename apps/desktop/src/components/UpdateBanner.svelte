<script lang="ts">
  import { app } from "../lib/store.svelte";
  import Icon from "./Icon.svelte";

  let pct = $derived(app.updateProgress === null ? null : Math.round(app.updateProgress * 100));
</script>

{#if app.update && !app.updateDismissed}
  <div class="banner">
    <span class="dot"><Icon name="sparkle" size={15} /></span>
    {#if app.updateState === "downloading"}
      <span class="text">Устанавливаем версию {app.update.version}… {pct !== null ? `${pct}%` : ""}</span>
      <span class="bar"><i style="width:{pct ?? 30}%"></i></span>
    {:else}
      <span class="text">
        <b>Доступна версия {app.update.version}</b>
        {#if app.update.notes}<span class="muted"> · {app.update.notes.split("\n")[0]}</span>{/if}
        {#if app.updateState === "error"}<span class="err"> · не удалось установить, попробуйте ещё раз</span>{/if}
      </span>
      <button class="btn btn-small btn-ghost" onclick={() => (app.updateDismissed = true)}>Позже</button>
      <button class="btn btn-small btn-primary" onclick={() => app.installUpdate()}>Обновить</button>
    {/if}
  </div>
{/if}

<style>
  .banner {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px 8px 16px;
    background: var(--accent-soft);
    border-bottom: 1px solid var(--border);
    flex: none;
  }
  .dot { color: var(--accent); display: grid; }
  .text { flex: 1; min-width: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; font-size: 13px; }
  .err { color: var(--bad); }
  .bar { width: 180px; height: 6px; border-radius: 6px; background: var(--track); overflow: hidden; }
  .bar i { display: block; height: 100%; background: var(--accent); transition: width 0.2s; }
</style>
