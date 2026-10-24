<script lang="ts">
  import { app, type Page } from "../lib/store.svelte";
  import Icon from "./Icon.svelte";

  const items: { id: Page; label: string }[] = [
    { id: "overview", label: "Обзор" },
    { id: "cleaner", label: "Очистка" },
    { id: "optimize", label: "Оптимизация" },
    { id: "settings", label: "Настройки" },
  ];
</script>

<nav class="sidebar">
  {#each items as item}
    <button class:active={app.page === item.id} onclick={() => (app.page = item.id)}>
      <Icon name={item.id} size={19} />
      <span>{item.label}</span>
      {#if item.id === "cleaner" && app.scanning}<i class="pulse"></i>{/if}
    </button>
  {/each}

  <div class="spacer"></div>

  {#if app.info}
    <div class="sys">
      <div class="host">{app.info.host_name}</div>
      <div class="faint">{app.info.os_label}</div>
      {#if app.info.is_elevated}
        <div class="admin"><Icon name="shield" size={13} /> Администратор</div>
      {/if}
    </div>
  {/if}
</nav>

<style>
  .sidebar {
    width: 208px;
    flex: none;
    padding: 16px 12px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    background: var(--surface);
    border-right: 1px solid var(--border);
  }
  button {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 40px;
    padding: 0 12px;
    border-radius: 10px;
    color: var(--text-2);
    font-weight: 550;
    text-align: left;
    position: relative;
  }
  button:hover { background: var(--surface-2); color: var(--text); }
  button.active { background: var(--accent-soft); color: var(--accent); }
  .pulse {
    margin-left: auto;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
    animation: pulse 1.2s ease-in-out infinite;
  }
  @keyframes pulse { 50% { opacity: 0.3; } }
  .spacer { flex: 1; }
  .sys { padding: 10px 12px; font-size: 12px; line-height: 1.5; }
  .host { font-weight: 600; color: var(--text-2); }
  .admin { display: flex; gap: 5px; align-items: center; color: var(--accent); margin-top: 4px; font-weight: 600; }
</style>
