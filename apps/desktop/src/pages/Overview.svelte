<script lang="ts">
  import AreaChart from "../components/AreaChart.svelte";
    import Icon from "../components/Icon.svelte";
  import Mascot from "../components/Mascot.svelte";
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

  const mascotColor = { good: "#5ee6a0", fair: "#ffd23f", poor: "#ff9a4d" };
  const mascotLine = { good: "Всё блестит!", fair: "Пора немного прибраться", poor: "Помогите, тут завал!" };

  const diskColor = (used: number) => (used >= 90 ? "var(--bad)" : used >= 80 ? "var(--warn)" : "var(--accent)");
</script>

<div class="page">
  <section class="hero">
    {#if h}
      {@const lvl = h.health.level}
      <div class="hero-text">
        <div class="eyebrow">Состояние компьютера</div>
        <div class="score num display">{h.health.score}<span class="of">/100</span></div>
        <h1>{healthLevel[lvl]}</h1>
        <div class="chips">
          {#each h.health.factors.slice(0, 3) as f}
            <span class="chip chip-small">{factorText(f)}</span>
          {:else}
            <span class="chip chip-small">Нагрузка в норме, места достаточно</span>
          {/each}
        </div>
        <button class="btn btn-primary hero-btn" onclick={quickClean} disabled={app.scanning || app.cleaning}>
          <Icon name="sparkle" size={16} /> Быстрая очистка
        </button>
      </div>
      <div class="hero-art">
        <div class="bubble">{mascotLine[lvl]}</div>
        <Mascot mood={lvl === "good" ? "happy" : lvl === "fair" ? "calm" : "worried"} color={mascotColor[lvl]} size={190} />
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

  <div class="grid">
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

  </div>
</div>

<style>
  .grid { display: grid; gap: 18px; }
  .top { grid-template-columns: repeat(3, minmax(0, 1fr)); }
  .mid { grid-template-columns: minmax(0, 1.25fr) minmax(0, 1fr); align-items: start; }

  .hero { display: flex; align-items: center; justify-content: space-between; gap: 24px; padding: 4px 4px 0; min-height: 250px; }
  .hero-text { display: flex; flex-direction: column; align-items: flex-start; gap: 8px; min-width: 0; }
  .eyebrow { font-weight: 650; color: var(--text-2); font-size: 15px; }
  .score { font-size: 96px; font-weight: 800; line-height: 0.95; letter-spacing: -0.04em; }
  .of { font-size: 28px; font-weight: 700; color: var(--text-3); letter-spacing: 0; margin-left: 6px; }
  .hero h1 { margin: 2px 0 6px; font-size: 30px; font-weight: 800; letter-spacing: -0.02em; line-height: 1.15; }
  .hero-btn { margin-top: 10px; height: 46px; padding: 0 24px; font-size: 15px; }
  .hero-art { position: relative; flex: none; padding: 40px 24px 0 0; }
  .bubble {
    position: absolute;
    top: 0;
    right: 150px;
    max-width: 210px;
    padding: 10px 16px;
    border-radius: 22px 22px 6px 22px;
    background: rgba(255, 255, 255, 0.2);
    border: 1px solid rgba(255, 255, 255, 0.3);
    backdrop-filter: blur(12px);
    font-weight: 650;
    white-space: nowrap;
    z-index: 1;
  }
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
    .hero-art { display: none; }
  }
</style>
