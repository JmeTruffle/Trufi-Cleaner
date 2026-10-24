<script lang="ts">
  import AreaChart from "../components/AreaChart.svelte";
  import HealthRing from "../components/HealthRing.svelte";
  import Icon from "../components/Icon.svelte";
  import { bytes, count, duration, percent, rate } from "../lib/format";
  import { adviceActionLabel, adviceText, factorText, healthLevel } from "../lib/i18n";
  import { app } from "../lib/store.svelte";
  import type { Advice } from "../lib/types";

  let s = $derived(app.snapshot);
  let h = $derived(app.health);
  let memPct = $derived(s ? (s.memory_used / Math.max(1, s.memory_total)) * 100 : 0);
  let procTab = $state<"cpu" | "memory">("cpu");
  let procs = $derived(s ? (procTab === "cpu" ? s.top_by_cpu : s.top_by_memory) : []);

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
    }
  }

  async function quickClean() {
    app.page = "cleaner";
    await app.runScan(true);
  }

  const diskColor = (used: number) => (used >= 90 ? "var(--bad)" : used >= 80 ? "var(--warn)" : "var(--accent)");
</script>

<div class="page">
  <div class="page-head">
    <div>
      <h1>Обзор системы</h1>
      <p>Нагрузка в реальном времени и состояние компьютера</p>
    </div>
  </div>

  <div class="grid top">
    <section class="card health">
      {#if h}
        <HealthRing score={h.health.score} level={h.health.level} size={124} />
        <div class="health-text">
          <div class="level">{healthLevel[h.health.level]}</div>
          {#if h.health.factors.length}
            <ul>
              {#each h.health.factors.slice(0, 3) as f}
                <li>{factorText(f)}</li>
              {/each}
            </ul>
          {:else}
            <p class="muted">Нагрузка в норме, места достаточно.</p>
          {/if}
          <button class="btn btn-primary" onclick={quickClean} disabled={app.scanning || app.cleaning}>
            <Icon name="sparkle" size={16} /> Быстрая очистка
          </button>
        </div>
      {:else}
        <div class="placeholder">Оцениваем состояние…</div>
      {/if}
    </section>

    <section class="card metric">
      <div class="card-title">Процессор <span class="faint">{s?.cpu_cores ?? "–"} потоков</span></div>
      <div class="big num" style="color:var(--cpu)">{s ? percent(s.cpu_percent) : "–"}</div>
      <AreaChart values={app.cpuHistory} color="var(--cpu)" />
      <div class="sub faint" title={s?.cpu_brand}>{s?.cpu_brand ?? ""}</div>
    </section>

    <section class="card metric">
      <div class="card-title">Память <span class="faint">{s ? bytes(s.memory_total, 0) : ""}</span></div>
      <div class="big num" style="color:var(--mem)">{percent(memPct)}</div>
      <AreaChart values={app.memHistory} color="var(--mem)" />
      <div class="sub faint">
        {#if s}Занято {bytes(s.memory_used)}{#if s.swap_total} · подкачка {bytes(s.swap_used)}{/if}{/if}
      </div>
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
                <b>{d.mount.replace(/\\$/, "")}</b>
                {#if d.name && d.name !== d.mount}<span class="faint" title={d.name}>{d.name}</span>{/if}
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
        <div class="seg">
          <button class:on={procTab === "cpu"} onclick={() => (procTab = "cpu")}>Процессор</button>
          <button class:on={procTab === "memory"} onclick={() => (procTab = "memory")}>Память</button>
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

    <section class="card facts">
      <div class="card-title">Сейчас</div>
      <div class="fact"><span class="muted">Сеть</span><span class="num">↓ {rate(s?.net_rx_bps ?? 0)} · ↑ {rate(s?.net_tx_bps ?? 0)}</span></div>
      <AreaChart values={app.netHistory} max={1} color="var(--net)" height={40} />
      <div class="fact"><span class="muted">Без перезагрузки</span><span class="num">{s ? duration(s.uptime_secs) : "–"}</span></div>
      <div class="fact"><span class="muted">Процессов</span><span class="num">{s ? count(s.process_count) : "–"}</span></div>
    </section>
  </div>
</div>

<style>
  .grid { display: grid; gap: 16px; }
  .top { grid-template-columns: minmax(0, 1.55fr) minmax(0, 1fr) minmax(0, 1fr); }
  .mid { grid-template-columns: minmax(0, 1.25fr) minmax(0, 1fr); align-items: start; }
  .bottom { grid-template-columns: minmax(0, 1.25fr) minmax(0, 1fr); }

  .health { display: flex; align-items: center; gap: 20px; }
  .health-text { display: flex; flex-direction: column; gap: 8px; align-items: flex-start; min-width: 0; }
  .level { font-size: 16px; font-weight: 650; line-height: 1.3; }
  .health ul { margin: 0 0 6px; padding-left: 16px; color: var(--text-2); font-size: 13px; }
  .health p { margin: 0 0 6px; }
  .placeholder { color: var(--text-3); padding: 40px 0; }

  .metric { display: flex; flex-direction: column; }
  .big { font-size: 30px; font-weight: 700; letter-spacing: -0.02em; margin: -4px 0 8px; }
  .sub { font-size: 12px; margin-top: 8px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

  .disks { display: flex; flex-direction: column; gap: 14px; }
  .disk-head { display: flex; justify-content: space-between; align-items: center; gap: 12px; margin-bottom: 7px; }
  .disk-name { display: flex; gap: 8px; align-items: center; color: var(--text-2); min-width: 0; overflow: hidden; white-space: nowrap; }
  .disk-name :global(svg) { flex: none; }
  .disk-name .faint { overflow: hidden; text-overflow: ellipsis; min-width: 0; }
  .disk-head > .num { white-space: nowrap; flex: none; }
  .disk-name b { color: var(--text); }
  .free { color: var(--text); font-weight: 600; }
  .bar { height: 8px; border-radius: 8px; background: var(--track); overflow: hidden; }
  .bar i { display: block; height: 100%; border-radius: 8px; transition: width 0.6s; }

  .advice { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 12px; }
  .advice li { display: flex; gap: 12px; align-items: flex-start; }
  .advice .dot { width: 8px; height: 8px; border-radius: 50%; margin-top: 7px; flex: none; background: var(--text-3); }
  .advice .warning .dot { background: var(--warn); }
  .advice .critical .dot { background: var(--bad); }
  .advice-text { flex: 1; min-width: 0; font-size: 13px; }
  .advice-title { font-weight: 600; font-size: 14px; }
  .empty { display: flex; gap: 8px; align-items: center; color: var(--good); }

  .seg { display: flex; background: var(--surface-2); border: 1px solid var(--border); border-radius: 8px; padding: 2px; }
  .seg button { padding: 3px 10px; border-radius: 6px; font-size: 12px; color: var(--text-2); font-weight: 600; }
  .seg button.on { background: var(--surface); color: var(--text); box-shadow: var(--shadow); }
  table { width: 100%; border-collapse: collapse; }
  td { padding: 7px 0; border-top: 1px solid var(--border); }
  tr:first-child td { border-top: 0; }
  .pname { max-width: 0; width: 60%; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .right { text-align: right; padding-left: 12px; }

  .facts { display: flex; flex-direction: column; gap: 10px; }
  .fact { display: flex; justify-content: space-between; gap: 12px; }

  @media (max-width: 1100px) {
    .top { grid-template-columns: 1fr 1fr; }
    .top .health { grid-column: 1 / -1; }
  }
</style>
