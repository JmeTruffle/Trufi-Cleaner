<script lang="ts">
  import { app, type Page } from "../lib/store.svelte";
  import Icon from "./Icon.svelte";

  const items: { id: Page; label: string }[] = [
    { id: "overview", label: "Обзор" },
    { id: "cleaner", label: "Очистка" },
    { id: "optimize", label: "Оптимизация" },
    { id: "drivers", label: "Драйверы" },
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
    width: 220px;
    flex: none;
    margin: 4px 0 14px 14px;
    padding: 14px 10px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    backdrop-filter: blur(18px);
  }
  button {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 44px;
    padding: 0 16px;
    border-radius: 999px;
    color: var(--text-2);
    font-weight: 600;
    font-size: 14.5px;
    transition: background 0.15s, color 0.15s;
    text-align: left;
    position: relative;
  }
  button:hover { background: var(--surface); color: var(--text); }
  button.active { background: #fff; color: var(--ink); box-shadow: var(--shadow); }
  .pulse {
    margin-left: auto;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent-2);
    animation: pulse 1.2s ease-in-out infinite;
  }
  @keyframes pulse { 50% { opacity: 0.3; } }
  .spacer { flex: 1; }
  .sys { padding: 12px 16px; border-radius: 18px; background: var(--surface-2); font-size: 12px; line-height: 1.5; }
  .host { font-weight: 700; color: var(--text); }
  .admin { display: flex; gap: 5px; align-items: center; color: var(--accent); margin-top: 4px; font-weight: 600; }
</style>
