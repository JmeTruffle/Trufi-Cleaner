<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "../components/Icon.svelte";
  import Modal from "../components/Modal.svelte";
  import { api } from "../lib/api";
  import { bytes, percent } from "../lib/format";
  import { errorText } from "../lib/i18n";
  import { isMac } from "../lib/os";
  import { app } from "../lib/store.svelte";
  import type { InstalledApp, StartupItem } from "../lib/types";

  let startup = $state<StartupItem[] | null>(null);
  let apps = $state<InstalledApp[] | null>(null);
  let query = $state("");
  let message = $state("");
  let uninstalling = $state<InstalledApp | null>(null);

  let enabledCount = $derived(startup?.filter((s) => s.enabled).length ?? 0);
  let filteredApps = $derived(
    (apps ?? [])
      .filter((a) => !query || `${a.name} ${a.publisher ?? ""}`.toLowerCase().includes(query.toLowerCase()))
      .sort((a, b) => (b.size_bytes ?? 0) - (a.size_bytes ?? 0)),
  );
  let s = $derived(app.snapshot);

  onMount(loadStartup);

  $effect(() => {
    if (app.optimizeTab === "apps" && apps === null) loadApps();
  });

  async function loadStartup() {
    try {
      startup = await api.startupItems();
    } catch (e) {
      message = errorText(e);
      startup = [];
    }
  }

  async function loadApps() {
    try {
      apps = await api.installedApps();
    } catch (e) {
      message = errorText(e);
      apps = [];
    }
  }

  async function toggle(item: StartupItem) {
    message = "";
    const next = !item.enabled;
    try {
      await api.setStartupEnabled(item.id, next);
      item.enabled = next;
      app.refreshHealth();
    } catch (e) {
      message = errorText(e);
    }
  }

  async function uninstall(a: InstalledApp) {
    uninstalling = null;
    try {
      const outcome = await api.uninstallApp(a.id);
      if (outcome === "moved_to_trash") {
        message = `«${a.name}» перемещена в Корзину. Её данные можно найти и удалить на странице «Очистка» в разделе «Остатки удалённых программ».`;
        apps = (apps ?? []).filter((x) => x.id !== a.id);
      } else {
        message = `Запущен деинсталлятор «${a.name}». Следуйте его шагам, затем обновите список.`;
      }
    } catch (e) {
      message = errorText(e);
    }
  }

  // On macOS the system asks for the administrator password itself, so
  // items that need it stay switchable.
  const locked = (item: StartupItem) => item.requires_admin && !app.info?.is_elevated && !isMac;
  const adminHint = isMac ? "macOS спросит пароль администратора" : "Нужны права администратора";
</script>

<div class="page">
  <div class="page-head">
    <div>
      <h1>Оптимизация</h1>
      <p>Автозагрузка, установленные программы и тяжёлые процессы</p>
    </div>
    <div class="chips">
      <button class="chip" class:on={app.optimizeTab === "startup"} onclick={() => (app.optimizeTab = "startup")}>Автозагрузка</button>
      <button class="chip" class:on={app.optimizeTab === "apps"} onclick={() => (app.optimizeTab = "apps")}>Программы</button>
      <button class="chip" class:on={app.optimizeTab === "processes"} onclick={() => (app.optimizeTab = "processes")}>Процессы</button>
    </div>
  </div>

  {#if message}<div class="note card"><Icon name="info" size={16} /> {message}</div>{/if}

  {#if app.optimizeTab === "startup"}
    <section class="card list">
      <div class="list-head">
        <div>
          <b>{enabledCount}</b> <span class="muted">из {startup?.length ?? 0} запускаются вместе с системой</span>
        </div>
        {#if isMac}
          <span class="faint small">Отключение обратимо и сработает при следующем входе в систему</span>
        {:else}
          <span class="faint small">Отключение обратимо и работает так же, как в Диспетчере задач</span>
        {/if}
      </div>
      {#if isMac}
        <div class="mac-note faint small">
          Объекты входа и службы, которые программы ставят для автозапуска. Выключенный объект входа Trufi Cleaner запоминает и может вернуть.
          Программы, добавленные через новые механизмы macOS, видны только в Системных настройках:
          <button class="link" onclick={() => api.openSystemPage("login_items").catch((e) => (message = errorText(e)))}>Объекты входа</button>
        </div>
      {/if}
      {#each startup ?? [] as item (item.id)}
        <div class="row">
          <div class="row-main">
            <div class="row-title">
              {item.name}
              {#if !item.target_exists}<span class="badge badge-expert" title="Программа, которую запускает запись, не найдена">Файл не найден</span>{/if}
              {#if item.scope === "all_users"}<span class="badge badge-muted">Все пользователи</span>{/if}
              {#if item.source === "folder"}<span class="badge badge-muted">Папка автозагрузки</span>{/if}
              {#if item.source === "login_item"}<span class="badge badge-muted" title="Открывается при входе в систему">Объект входа</span>{/if}
              {#if item.source === "launch_daemon"}<span class="badge badge-muted" title="Работает от имени системы, ещё до входа пользователя">Системная служба</span>{/if}
            </div>
            <div class="cmd faint" title={item.command}>{item.command}</div>
          </div>
          {#if item.requires_admin && !app.info?.is_elevated}
            <span class="faint" title={adminHint}><Icon name="shield" size={15} /></span>
          {/if}
          <input
            type="checkbox"
            class="switch"
            checked={item.enabled}
            disabled={locked(item)}
            onchange={() => toggle(item)}
            aria-label={item.name}
          />
        </div>
      {:else}
        <div class="empty faint">{startup ? "Автозагрузка пуста" : "Загружаем…"}</div>
      {/each}
    </section>
  {:else if app.optimizeTab === "apps"}
    <section class="card list">
      <div class="list-head">
        <div class="search">
          <Icon name="search" size={16} />
          <input placeholder="Поиск программ" bind:value={query} />
        </div>
        <span class="faint small">{apps?.length ?? 0} программ</span>
      </div>
      {#each filteredApps as a (a.id)}
        <div class="row">
          <div class="row-main">
            <div class="row-title">
              {a.name}
              {#if a.broken}<span class="badge badge-expert" title="Запись осталась, а файлов программы нет">Файлы не найдены</span>{/if}
            </div>
            <div class="faint small">{[a.publisher, a.version, isMac ? a.bundle_id : null].filter(Boolean).join(" · ")}</div>
          </div>
          <span class="num muted">{a.size_bytes ? bytes(a.size_bytes) : ""}</span>
          <button class="btn btn-ghost btn-small" disabled={!a.can_uninstall} onclick={() => (uninstalling = a)}>Удалить</button>
        </div>
      {:else}
        <div class="empty faint">{apps ? "Ничего не найдено" : "Загружаем…"}</div>
      {/each}
    </section>
  {:else}
    <section class="card list">
      <div class="list-head">
        <div class="muted">Программы, которые сильнее всего нагружают компьютер сейчас</div>
      </div>
      <div class="procs">
        <div>
          <h3>Процессор</h3>
          {#each s?.top_by_cpu ?? [] as p}
            <div class="proc">
              <span class="pname">{p.name}{#if p.instances > 1}<span class="faint"> ×{p.instances}</span>{/if}</span>
              <span class="pbar"><i style="width:{Math.min(100, p.cpu_percent)}%;background:var(--cpu)"></i></span>
              <span class="num">{percent(p.cpu_percent, 1)}</span>
            </div>
          {/each}
        </div>
        <div>
          <h3>Память</h3>
          {#each s?.top_by_memory ?? [] as p}
            <div class="proc">
              <span class="pname">{p.name}{#if p.instances > 1}<span class="faint"> ×{p.instances}</span>{/if}</span>
              <span class="pbar"><i style="width:{(p.memory_bytes / Math.max(1, s?.memory_total ?? 1)) * 100}%;background:var(--mem)"></i></span>
              <span class="num">{bytes(p.memory_bytes)}</span>
            </div>
          {/each}
        </div>
      </div>
    </section>
  {/if}
</div>

{#if uninstalling}
  {@const a = uninstalling}
  <Modal title="Удалить {a.name}?" onclose={() => (uninstalling = null)}>
    {#if isMac}
      <p>Программа будет перемещена в Корзину через Finder, её можно будет вернуть. Если у программы есть свой деинсталлятор, запустится он. Для программ из App Store и установленных администратором macOS спросит пароль.</p>
    {:else}
      <p>Запустится собственный деинсталлятор программы. Trufi Cleaner сам ничего не удаляет на этом шаге.</p>
    {/if}
    {#snippet actions()}
      <button class="btn btn-ghost" onclick={() => (uninstalling = null)}>Отмена</button>
      <button class="btn btn-danger" onclick={() => uninstall(a)}>{isMac ? "Переместить в Корзину" : "Запустить удаление"}</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .note { display: flex; gap: 8px; align-items: center; padding: 12px 18px; color: var(--text-2); }
  .list { padding: 0 0 6px; }
  .list-head { display: flex; justify-content: space-between; align-items: center; gap: 16px; padding: 16px 22px; border-bottom: 1px solid var(--divider); }
  .small { font-size: 12px; }
  .row { display: flex; align-items: center; gap: 14px; padding: 13px 0; margin: 0 22px; border-bottom: 1px solid var(--divider); }
  .row:last-child { border-bottom: 0; }
  .row-main { flex: 1; min-width: 0; }
  .row-title { display: flex; gap: 8px; align-items: center; font-weight: 650; font-size: 15px; }
  .cmd { font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; font-family: ui-monospace, "Cascadia Mono", Consolas, monospace; }
  .empty { padding: 32px; text-align: center; }
  .mac-note { padding: 12px 22px; border-bottom: 1px solid var(--divider); line-height: 1.5; }
  .link { color: var(--accent); text-decoration: underline; text-underline-offset: 2px; font-weight: 600; }
  .search { display: flex; align-items: center; gap: 8px; color: var(--text-2); flex: 1; max-width: 380px; height: 40px; padding: 0 16px; border-radius: 999px; background: var(--surface-2); border: 1px solid var(--border); }
  .search input { border: 0; outline: 0; background: none; flex: 1; color: var(--text); }
  .search input::placeholder { color: var(--text-3); }
  .procs { display: grid; grid-template-columns: 1fr 1fr; gap: 32px; padding: 18px 22px 18px; }
  .procs h3 { margin: 0 0 10px; font-size: 17px; font-weight: 750; }
  .proc { display: grid; grid-template-columns: 1fr 90px 70px; gap: 12px; align-items: center; padding: 6px 0; }
  .pname { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .pbar { height: 8px; border-radius: 8px; background: var(--track); overflow: hidden; }
  .pbar i { display: block; height: 100%; border-radius: 6px; }
  .proc .num { text-align: right; }
</style>
