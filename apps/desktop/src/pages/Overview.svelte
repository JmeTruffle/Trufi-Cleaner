<script lang="ts">
  import AreaChart from "../components/AreaChart.svelte";
    import Icon from "../components/Icon.svelte";
  import Mascot from "../components/Mascot.svelte";
  import { bytes, count, duration, percent, plural, rate } from "../lib/format";
  import { adviceActionLabel, adviceText, factorText, healthLevel } from "../lib/i18n";
  import { api } from "../lib/api";
  import { isMac, isWindows, osName } from "../lib/os";
  import { app } from "../lib/store.svelte";
  import type { Advice } from "../lib/types";

  let s = $derived(app.snapshot);
  let h = $derived(app.health);
  let memPct = $derived(s ? (s.memory_used / Math.max(1, s.memory_total)) * 100 : 0);
  let procTab = $state<"cpu" | "memory">("cpu");
  let procs = $derived(s ? (procTab === "cpu" ? s.top_by_cpu : s.top_by_memory) : []);
  let u = $derived(app.updates);
  let pendingUpdates = $derived(u?.pending.filter((x) => !x.driver && !x.optional && !x.definition).length ?? 0);
  let driverProblems = $derived(
    app.drivers?.filter((d) => d.problem_code != null && d.problem_code !== 0 && d.problem_code !== 22 && d.problem_code !== 45).length ?? 0,
  );
  let driverOffers = $derived(u?.pending.filter((x) => x.driver).length ?? 0);

  function act(a: Advice) {
    switch (a.action) {
      case "scan":
        app.page = "cleaner";
        if (!app.scan && !app.scanning) app.runScan();
        break;
      case "clean":
        app.page = "cleaner";
        break;
      case "startup":
        app.optimizeTab = "startup";
        app.page = "optimize";
        break;
      case "processes":
        app.optimizeTab = "processes";
        app.page = "optimize";
        break;
      case "updates":
        api.openSystemPage("system_update").catch(() => {});
        break;
      case "drivers":
        app.page = "drivers";
        break;
    }
  }

  async function quickClean() {
    app.page = "cleaner";
    await app.runScan(true);
  }

  // The mascot's colour slides smoothly with the score: green when all is
  // well, through yellow and orange, to red when the system needs help.
  function mascotColor(score: number) {
    const stops: [number, number][] = [[0, 2], [35, 8], [55, 30], [80, 48], [100, 150]];
    let i = stops.findIndex(([s]) => score <= s);
    if (i <= 0) i = 1;
    const [s0, h0] = stops[i - 1];
    const [s1, h1] = stops[Math.min(i, stops.length - 1)];
    const t = s1 === s0 ? 1 : Math.min(1, Math.max(0, (score - s0) / (s1 - s0)));
    return `hsl(${(h0 + (h1 - h0) * t).toFixed(0)} 82% 63%)`;
  }
  const mascotMood = (score: number) => (score >= 80 ? "happy" : score >= 55 ? "calm" : score >= 35 ? "worried" : "sad");

  const dmy = (iso: string) => iso.split("-").reverse().join(".");

  const diskColor = (used: number) => (used >= 90 ? "var(--bad)" : used >= 80 ? "var(--warn)" : "var(--accent)");
</script>

<div class="page">
  <section class="hero">
    {#if h}
      <div class="hero-text">
        <div class="eyebrow">Состояние компьютера</div>
        <div class="score num display">{h.health.score}<span class="of">/100</span></div>
        <h1>{healthLevel[h.health.level]}</h1>
        <button class="btn btn-primary hero-btn" onclick={quickClean} disabled={app.scanning || app.cleaning}>
          <Icon name="sparkle" size={16} /> Быстрая очистка
        </button>
      </div>
      <div class="hero-art">
        <div class="bubble">
          {#each h.health.factors.slice(0, 3) as f}
            <p>{factorText(f)}</p>
          {:else}
            <p>Нагрузка в норме, места достаточно</p>
          {/each}
        </div>
        <Mascot mood={mascotMood(h.health.score)} color={mascotColor(h.health.score)} size={150} />
      </div>
    {:else}
      <div class="placeholder">Оцениваем состояние…</div>
    {/if}
  </section>

  <div class="grid top">
    <section class="card metric">
      <div class="card-title">Процессор <span class="faint">{s?.cpu_cores ?? "–"} потоков</span></div>
      <div class="big num display">{s ? percent(s.cpu_percent) : "–"}</div>
      <AreaChart values={app.cpuHistory} color="var(--cpu)" />
      <div class="sub faint" title={s?.cpu_brand}>{s?.cpu_brand ?? ""}</div>
    </section>

    <section class="card metric">
      <div class="card-title">Память <span class="faint">{s ? bytes(s.memory_total, 0) : ""}</span></div>
      <div class="big num display">{percent(memPct)}</div>
      <AreaChart values={app.memHistory} color="var(--mem)" />
      <div class="sub faint">
        {#if s}Занято {bytes(s.memory_used)}{#if s.swap_total} · подкачка {bytes(s.swap_used)}{/if}{/if}
      </div>
    </section>

    <section class="card metric">
      <div class="card-title">Сеть <span class="faint">{s ? `${count(s.process_count)} процессов` : ""}</span></div>
      <div class="big num display net">↓ {rate(s?.net_rx_bps ?? 0)}</div>
      <AreaChart values={app.netHistory} max={1} color="var(--net)" />
      <div class="sub faint">↑ {rate(s?.net_tx_bps ?? 0)} · без перезагрузки {s ? duration(s.uptime_secs) : "–"}</div>
    </section>
  </div>

  <div class="grid mid">
    <section class="card">
      <div class="card-title">Диски <span class="faint">{s?.disks.length ?? 0}</span></div>
      <div class="disks">
        {#each s?.disks ?? [] as d}
          {@const used = 100 - (d.available_bytes / Math.max(1, d.total_bytes)) * 100}
          <div class="disk">
            <div class="disk-head">
              <span class="disk-name">
                <Icon name="disk" size={16} />
                {#if isMac}
                  <!-- macOS: volumes are known by name ("Macintosh HD"), not mount point. -->
                  <b>{d.name || d.mount}</b>
                  {#if d.mount !== "/"}<span class="faint" title={d.mount}>{d.mount}</span>{/if}
                {:else}
                  <b>{d.mount.replace(/\\$/, "")}</b>
                  {#if d.name && d.name !== d.mount}<span class="faint" title={d.name}>{d.name}</span>{/if}
                {/if}
                {#if d.is_system}<span class="badge badge-muted">Системный</span>{/if}
                {#if d.kind !== "unknown"}<span class="badge badge-muted">{d.kind.toUpperCase()}</span>{/if}
              </span>
              <span class="num muted">свободно <b class="free">{bytes(d.available_bytes)}</b> из {bytes(d.total_bytes, 0)}</span>
            </div>
            <div class="bar"><i style="width:{used}%;background:{diskColor(used)}"></i></div>
          </div>
        {/each}
      </div>
    </section>

    <section class="card">
      <div class="card-title">Советы</div>
      {#if h?.advice.length}
        <ul class="advice">
          {#each h.advice as a}
            {@const t = adviceText(a)}
            <li class={a.severity}>
              <span class="dot"></span>
              <div class="advice-text">
                <div class="advice-title">{t.title}</div>
                <div class="muted">{t.detail}</div>
              </div>
              {#if a.action && a.action !== "restart"}
                <button class="btn btn-ghost btn-small" onclick={() => act(a)}>{adviceActionLabel[a.action]}</button>
              {/if}
            </li>
          {/each}
        </ul>
      {:else}
        <div class="empty"><Icon name="check" /> Всё в порядке</div>
      {/if}
    </section>
  </div>

  <div class="grid bottom">
    <section class="card">
      <div class="card-title">
        Активные программы
        <div class="chips">
          <button class="chip chip-small" class:on={procTab === "cpu"} onclick={() => (procTab = "cpu")}>Процессор</button>
          <button class="chip chip-small" class:on={procTab === "memory"} onclick={() => (procTab = "memory")}>Память</button>
        </div>
      </div>
      <table>
        <tbody>
          {#each procs as p}
            <tr>
              <td class="pname">{p.name}{#if p.instances > 1}<span class="faint"> ×{p.instances}</span>{/if}</td>
              <td class="num right">{percent(p.cpu_percent, 1)}</td>
              <td class="num right muted">{bytes(p.memory_bytes)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </section>

    <section class="card">
      <div class="card-title">Система</div>
      <div class="sys-row">
        <span class="sys-icon"><Icon name={isMac ? "laptop" : "windows"} size={18} /></span>
        <div class="sys-main">
          <div class="sys-title">
            {#if u?.version}{u.version.name} {u.version.display_version ?? ""}{:else}{osName}{/if}
          </div>
          {#if app.updatesState === "loading" && !u}
            <div class="muted">Проверяем обновления…</div>
          {:else if app.updatesState === "error" && !u}
            <div class="muted">Не удалось проверить обновления: {app.updatesError}</div>
          {:else if u}
            {#if u.out_of_support}
              <div class="st bad">Версия не поддерживается{#if u.servicing_ends} с {dmy(u.servicing_ends)}{/if}</div>
            {:else if pendingUpdates}
              <div class="st warn">Ждут установки: {pendingUpdates}</div>
            {:else if u.reboot_required}
              <div class="st warn">Обновления ждут перезагрузки</div>
            {:else if u.error}
              <div class="muted">{isMac ? "Обновление ПО" : "Центр обновления"} недоступен: {u.error}</div>
            {:else}
              <div class="st good">Обновления установлены</div>
            {/if}
            <div class="faint small">
              {[
                u.version && (u.version.build_label ? `Сборка ${u.version.build_label}` : u.version.build ? `Сборка ${u.version.build}.${u.version.revision}` : ""),
                u.last_install && `обновлялась ${dmy(u.last_install)}`,
              ].filter(Boolean).join(" · ")}
            </div>
          {/if}
        </div>
        <button class="btn btn-ghost btn-small" onclick={() => api.openSystemPage("system_update").catch(() => {})}>
          {isMac ? "Обновление ПО" : "Центр обновления"}
        </button>
      </div>
      {#if isMac}
        <div class="sys-row">
          <span class="sys-icon"><Icon name="lock" size={18} /></span>
          <div class="sys-main">
            <div class="sys-title">Полный доступ к диску</div>
            {#if app.info?.full_disk_access}
              <div class="st good">Разрешён</div>
            {:else}
              <div class="st warn">Не разрешён</div>
              <div class="faint small">Без него не видны Корзина, кеш Safari, вложения Почты и остатки программ из App Store</div>
            {/if}
          </div>
          {#if app.info && !app.info.full_disk_access}
            <button class="btn btn-ghost btn-small" onclick={() => api.openSystemPage("full_disk_access").catch(() => {})}>Разрешить</button>
          {/if}
        </div>
      {/if}
      {#if isWindows}
      <div class="sys-row">
        <span class="sys-icon"><Icon name="drivers" size={18} /></span>
        <div class="sys-main">
          <div class="sys-title">Драйверы</div>
          {#if app.driversState === "loading" && !app.drivers}
            <div class="muted">Проверяем устройства…</div>
          {:else if app.driversState === "error" && !app.drivers}
            <div class="muted">Не удалось проверить: {app.driversError}</div>
          {:else if app.drivers}
            {#if driverProblems}
              <div class="st bad">{driverProblems} {plural(driverProblems, "устройство работает", "устройства работают", "устройств работают")} с ошибкой</div>
            {:else}
              <div class="st good">Все устройства работают</div>
            {/if}
            {#if driverOffers}<div class="faint small">Доступно обновлений: {driverOffers}</div>{/if}
          {/if}
        </div>
        <button class="btn btn-ghost btn-small" onclick={() => (app.page = "drivers")}>Открыть</button>
      </div>
      {/if}
    </section>
  </div>
</div>

<style>
  .grid { display: grid; gap: 18px; }
  .top { grid-template-columns: repeat(3, minmax(0, 1fr)); }
  .mid { grid-template-columns: minmax(0, 1.25fr) minmax(0, 1fr); align-items: start; }
  .bottom { grid-template-columns: minmax(0, 1.25fr) minmax(0, 1fr); align-items: start; }

  .sys-row { display: flex; gap: 12px; align-items: flex-start; padding: 12px 0; border-top: 1px solid var(--divider); }
  .card-title + .sys-row { border-top: 0; padding-top: 2px; }
  .sys-icon { flex: none; width: 36px; height: 36px; border-radius: 12px; display: grid; place-items: center; background: var(--surface-2); color: var(--text-2); }
  .sys-main { flex: 1; min-width: 0; font-size: 13px; }
  .sys-title { font-weight: 700; font-size: 15px; }
  .st { font-weight: 600; }
  .st.good { color: var(--good); }
  .st.warn { color: var(--warn); }
  .st.bad { color: var(--bad); }
  .small { font-size: 12px; }

  .hero { display: flex; align-items: center; justify-content: space-between; gap: 24px; padding: 4px 4px 0; min-height: 250px; }
  .hero-text { flex: 1; display: flex; flex-direction: column; align-items: flex-start; gap: 8px; min-width: 0; }
  .eyebrow { font-weight: 650; color: var(--text-2); font-size: 15px; }
  .score { font-size: 96px; font-weight: 800; line-height: 0.95; letter-spacing: -0.04em; }
  .of { font-size: 28px; font-weight: 700; color: var(--text-3); letter-spacing: 0; margin-left: 6px; }
  .hero h1 { margin: 2px 0 6px; font-size: 30px; font-weight: 800; letter-spacing: -0.02em; line-height: 1.15; }
  .hero-btn { margin-top: 10px; height: 46px; padding: 0 24px; font-size: 15px; }
  .hero-art { display: flex; align-items: flex-start; flex: none; padding-right: 16px; }
  .bubble {
    width: max-content;
    max-width: 230px;
    margin: 0 -4px 0 0;
    padding: 12px 18px;
    border-radius: 22px 22px 6px 22px;
    background: rgba(255, 255, 255, 0.22);
    border: 1px solid rgba(255, 255, 255, 0.32);
    backdrop-filter: blur(12px);
    font-weight: 650;
    font-size: 13.5px;
    line-height: 1.35;
    position: relative;
    z-index: 1;
  }
  .bubble p { margin: 0; }
  .bubble p + p { margin-top: 6px; }
  .hero-art :global(.mascot) { margin-top: 56px; }
  .placeholder { color: var(--text-3); padding: 40px 0; }

  .metric { display: flex; flex-direction: column; }
  .big { font-size: 40px; font-weight: 800; letter-spacing: -0.03em; line-height: 1.05; margin: -2px 0 10px; }
  .net { font-size: 30px; line-height: 1.4; }
  .sub { font-size: 12.5px; margin-top: 10px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

  .disks { display: flex; flex-direction: column; gap: 16px; }
  .disk-head { display: flex; justify-content: space-between; align-items: center; gap: 12px; margin-bottom: 8px; }
  .disk-name { display: flex; gap: 8px; align-items: center; color: var(--text-2); min-width: 0; overflow: hidden; white-space: nowrap; }
  .disk-name :global(svg) { flex: none; }
  .disk-name .faint { overflow: hidden; text-overflow: ellipsis; min-width: 0; }
  .disk-head > .num { white-space: nowrap; flex: none; }
  .disk-name b { color: var(--text); font-size: 15px; }
  .free { color: var(--text); font-weight: 700; }
  .bar { height: 10px; border-radius: 10px; background: var(--track); overflow: hidden; }
  .bar i { display: block; height: 100%; border-radius: 10px; transition: width 0.6s; }

  .advice { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; }
  .advice li { display: flex; gap: 12px; align-items: flex-start; padding: 12px 0; border-top: 1px solid var(--divider); }
  .advice li:first-child { border-top: 0; padding-top: 2px; }
  .advice .dot { width: 10px; height: 10px; border-radius: 50%; margin-top: 6px; flex: none; background: var(--text-3); }
  .advice .warning .dot { background: var(--warn); }
  .advice .critical .dot { background: var(--bad); }
  .advice-text { flex: 1; min-width: 0; font-size: 13px; }
  .advice-title { font-weight: 700; font-size: 15px; }
  .empty { display: flex; gap: 8px; align-items: center; color: var(--good); font-weight: 600; }

  table { width: 100%; border-collapse: collapse; }
  td { padding: 9px 0; border-top: 1px solid var(--divider); }
  tr:first-child td { border-top: 0; }
  .pname { max-width: 0; width: 60%; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; font-weight: 550; }
  .right { text-align: right; padding-left: 12px; }

  @media (max-width: 1000px) {
    .top { grid-template-columns: 1fr 1fr; }
    .bottom { grid-template-columns: 1fr; }
    .hero-art { display: none; }
  }
</style>
