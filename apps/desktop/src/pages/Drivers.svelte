<script lang="ts">
  import Icon from "../components/Icon.svelte";
  import { api } from "../lib/api";
  import { plural } from "../lib/format";
  import { driverClassName, errorText, problemHint, problemText } from "../lib/i18n";
  import { app } from "../lib/store.svelte";
  import type { DriverInfo, SystemPage } from "../lib/types";

  type Filter = "attention" | "third_party" | "all";

  let query = $state("");
  let message = $state("");
  let picked = $state<Filter | null>(null);

  let list = $derived(app.drivers ?? []);
  let problems = $derived(list.filter(hasProblem));
  let unsigned = $derived(list.filter((d) => d.signed === false && d.inf));
  let old = $derived(list.filter((d) => d.old));
  let attention = $derived(list.filter(needsAttention));
  let offers = $derived((app.updates?.pending ?? []).filter((u) => u.driver));
  let filter = $derived<Filter>(picked ?? (attention.length ? "attention" : "third_party"));
  let loading = $derived(app.driversState === "loading" && !app.drivers);
  let checking = $derived(app.updatesState === "loading" || app.driversState === "loading");

  let shown = $derived(
    list
      .filter((d) => (filter === "attention" ? needsAttention(d) : filter === "third_party" ? !d.inbox : true))
      .filter((d) => !query || `${d.device} ${d.provider ?? ""} ${d.class ?? ""}`.toLowerCase().includes(query.toLowerCase())),
  );

  function hasProblem(d: DriverInfo) {
    return d.problem_code != null && d.problem_code !== 0 && d.problem_code !== 22 && d.problem_code !== 45;
  }

  function needsAttention(d: DriverInfo) {
    return hasProblem(d) || !!d.update || (d.signed === false && !!d.inf) || d.old;
  }

  function age(days: number) {
    const y = Math.floor(days / 365);
    return y >= 1 ? `${y} ${plural(y, "год", "года", "лет")}` : `${days} ${plural(days, "день", "дня", "дней")}`;
  }

  function date(iso: string | null) {
    if (!iso) return "";
    const [y, m, d] = iso.split("-");
    return `${d}.${m}.${y}`;
  }

  async function open(page: SystemPage) {
    message = "";
    try {
      await api.openSystemPage(page);
    } catch (e) {
      message = errorText(e);
    }
  }

  async function recheck() {
    message = "";
    await app.loadDrivers(true);
    await app.loadUpdates(true);
  }
</script>

<div class="page">
  <div class="page-head">
    <div>
      <h1>Драйверы</h1>
      <p>Стабильные и свежие драйверы: меньше сбоев, вылетов и конфликтов устройств</p>
    </div>
    <div class="head-actions">
      <button class="btn btn-ghost" onclick={() => open("device_manager")}>Диспетчер устройств</button>
      <button class="btn btn-primary" onclick={recheck} disabled={checking}>
        <span class:spin={checking}><Icon name="refresh" size={16} /></span>
        {checking ? "Проверяем…" : "Проверить снова"}
      </button>
    </div>
  </div>

  {#if message}<div class="note card"><Icon name="info" size={16} /> {message}</div>{/if}

  {#if app.driversState === "error" && !app.drivers}
    <div class="note card"><Icon name="alert" size={16} /> Не удалось получить список драйверов: {app.driversError}</div>
  {:else if loading}
    <div class="card empty faint">Собираем список устройств и драйверов…</div>
  {:else if app.drivers}
    <div class="stats">
      <button class="card stat" class:bad={problems.length > 0} onclick={() => (picked = "attention")}>
        <span class="stat-num num display">{problems.length}</span>
        <span class="stat-label">{problems.length ? plural(problems.length, "устройство с ошибкой", "устройства с ошибкой", "устройств с ошибкой") : "Ошибок и конфликтов нет"}</span>
      </button>
      <button class="card stat" class:warn={offers.length > 0} onclick={() => (picked = "attention")}>
        <span class="stat-num num display">{offers.length}</span>
        <span class="stat-label">
          {#if app.updatesState === "loading" && !app.updates}Ищем обновления…
          {:else}{plural(offers.length, "обновление", "обновления", "обновлений")} от Windows Update{/if}
        </span>
      </button>
      <button class="card stat" class:warn={unsigned.length > 0} onclick={() => (picked = "attention")}>
        <span class="stat-num num display">{unsigned.length}</span>
        <span class="stat-label">без цифровой подписи</span>
      </button>
      <button class="card stat" onclick={() => (picked = "attention")}>
        <span class="stat-num num display">{old.length}</span>
        <span class="stat-label">старше трёх лет</span>
      </button>
    </div>

    {#if offers.length}
      <section class="card offer">
        <span class="offer-icon"><Icon name="sparkle" size={20} /></span>
        <div class="offer-text">
          <b>Windows Update предлагает {offers.length} {plural(offers.length, "драйвер", "драйвера", "драйверов")}</b>
          <div class="muted">Это версии, проверенные Microsoft на совместимость. Ставить их безопаснее всего.</div>
        </div>
        <button class="btn btn-ghost btn-small" onclick={() => open(offers.some((o) => o.optional) ? "optional_updates" : "windows_update")}>Установить</button>
      </section>
    {/if}

    <section class="card list">
      <div class="list-head">
        <div class="chips">
          <button class="chip" class:on={filter === "attention"} onclick={() => (picked = "attention")}>Требуют внимания · {attention.length}</button>
          <button class="chip" class:on={filter === "third_party"} onclick={() => (picked = "third_party")}>Сторонние</button>
          <button class="chip" class:on={filter === "all"} onclick={() => (picked = "all")}>Все · {list.length}</button>
        </div>
        <div class="search">
          <Icon name="search" size={16} />
          <input placeholder="Поиск устройств" bind:value={query} />
        </div>
      </div>
      {#each shown as d (d.id)}
        <div class="row">
          <span class="icon" class:bad={hasProblem(d)}><Icon name={hasProblem(d) ? "alert" : "drivers"} size={18} /></span>
          <div class="row-main">
            <div class="row-title">
              <span class="name" title={d.device}>{d.device}</span>
              {#if hasProblem(d)}<span class="badge badge-expert">{problemText(d.problem_code!)}</span>{/if}
              {#if d.update}<span class="badge badge-safe" title={d.update.title}>Есть обновление</span>{/if}
              {#if d.signed === false && d.inf}<span class="badge badge-moderate" title="Windows не может проверить, кто выпустил драйвер">Без подписи</span>{/if}
              {#if d.old && d.age_days}<span class="badge badge-muted" title="Сторонний драйвер давно не обновлялся">{age(d.age_days)}</span>{/if}
            </div>
            <div class="faint small">
              {[driverClassName(d.class), d.provider, d.version, date(d.date)].filter(Boolean).join(" · ")}
            </div>
            {#if hasProblem(d)}<div class="hint">{problemHint(d.problem_code!)}</div>
            {:else if d.update}<div class="hint">{d.update.title}</div>{/if}
          </div>
          {#if hasProblem(d)}
            <button class="btn btn-ghost btn-small" onclick={() => open("device_manager")}>Открыть</button>
          {:else if d.update}
            <button class="btn btn-ghost btn-small" onclick={() => open(d.update?.optional ? "optional_updates" : "windows_update")}>Обновить</button>
          {/if}
        </div>
      {:else}
        <div class="empty">
          {#if filter === "attention"}<span class="ok"><Icon name="check" /> Все устройства работают, драйверы в порядке</span>
          {:else}<span class="faint">Ничего не найдено</span>{/if}
        </div>
      {/each}
    </section>

    <section class="card tips">
      <div class="card-title">Как держать драйверы стабильными</div>
      <ul>
        <li>Ставьте драйверы из Центра обновления или с сайта производителя устройства. «Сборники драйверов» часто ставят не те версии.</li>
        <li>Видеодрайвер и драйвер чипсета стоит обновлять регулярно. Остальные трогайте, только если устройство сбоит.</li>
        <li>Если после обновления начались сбои, откатите драйвер в Диспетчере устройств: «Свойства» → «Драйвер» → «Откатить».</li>
      </ul>
    </section>
  {/if}
</div>

<style>
  .head-actions { display: flex; gap: 8px; flex: none; }
  .spin { display: inline-flex; animation: spin 1s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  .note { display: flex; gap: 8px; align-items: center; padding: 12px 18px; color: var(--text-2); }
  .empty { padding: 32px; text-align: center; }
  .ok { display: inline-flex; gap: 8px; align-items: center; color: var(--good); font-weight: 600; }

  .stats { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 18px; }
  .stat { display: flex; flex-direction: column; align-items: flex-start; gap: 2px; text-align: left; color: var(--text); transition: background 0.15s; }
  .stat:hover { background: var(--surface-hover); }
  .stat-num { font-size: 40px; font-weight: 800; line-height: 1.05; letter-spacing: -0.03em; }
  .stat-label { color: var(--text-2); font-size: 13px; }
  .stat.bad .stat-num { color: var(--bad); }
  .stat.warn .stat-num { color: var(--warn); }

  .offer { display: flex; align-items: center; gap: 14px; }
  .offer-icon { display: inline-flex; color: var(--accent); }
  .offer-text { flex: 1; font-size: 13px; }
  .offer-text b { font-size: 15px; }

  .list { padding: 0 0 6px; }
  .list-head { display: flex; justify-content: space-between; align-items: center; gap: 16px; padding: 16px 22px; border-bottom: 1px solid var(--divider); flex-wrap: wrap; }
  .search { display: flex; align-items: center; gap: 8px; color: var(--text-2); flex: 1; max-width: 300px; min-width: 180px; height: 38px; padding: 0 16px; border-radius: 999px; background: var(--surface-2); border: 1px solid var(--border); }
  .search input { border: 0; outline: 0; background: none; flex: 1; min-width: 0; color: var(--text); }
  .search input::placeholder { color: var(--text-3); }
  .row { display: flex; align-items: center; gap: 14px; padding: 13px 0; margin: 0 22px; border-bottom: 1px solid var(--divider); }
  .row:last-child { border-bottom: 0; }
  .icon { flex: none; width: 36px; height: 36px; border-radius: 12px; display: grid; place-items: center; background: var(--surface-2); color: var(--text-2); }
  .icon.bad { background: var(--bad-soft); color: var(--bad); }
  .row-main { flex: 1; min-width: 0; }
  .row-title { display: flex; gap: 8px; align-items: center; font-weight: 650; font-size: 15px; min-width: 0; }
  .name { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; min-width: 0; }
  .small { font-size: 12px; }
  .hint { font-size: 12.5px; color: var(--text-2); margin-top: 3px; }

  .tips ul { margin: 0; padding-left: 18px; display: flex; flex-direction: column; gap: 6px; color: var(--text-2); }

  @media (max-width: 1000px) {
    .stats { grid-template-columns: 1fr 1fr; }
  }
</style>
