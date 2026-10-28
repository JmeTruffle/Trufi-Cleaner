<script lang="ts">
  import Icon from "../components/Icon.svelte";
  import Mascot from "../components/Mascot.svelte";
  import Modal from "../components/Modal.svelte";
  import { api } from "../lib/api";
  import { bytes, count, plural } from "../lib/format";
  import { categoryName, categoryOrder, errorText, riskHint, riskName } from "../lib/i18n";
  import { app } from "../lib/store.svelte";
  import type { Category, RuleFinding } from "../lib/types";

  let expanded = $state<Record<string, boolean>>({});
  let confirming = $state(false);
  let error = $state("");

  let groups = $derived.by(() => {
    const by = new Map<Category, RuleFinding[]>();
    for (const f of app.scan?.findings ?? []) {
      if (!by.has(f.category)) by.set(f.category, []);
      by.get(f.category)!.push(f);
    }
    return categoryOrder
      .filter((c) => by.has(c))
      .map((c) => ({ category: c, items: by.get(c)!, bytes: by.get(c)!.reduce((s, f) => s + f.bytes, 0) }));
  });

  let selectedBytes = $derived(app.selectedBytes());
  let elevated = $derived(app.info?.is_elevated ?? false);
  let selectedFindings = $derived(
    (app.scan?.findings ?? []).filter((f) =>
      f.itemized ? (app.selectedEntries[f.rule_id] ?? []).length > 0 : app.selected[f.rule_id],
    ),
  );
  let needsReview = $derived(selectedFindings.some((f) => f.risk === "expert"));
  let needsAdmin = $derived(!elevated && selectedFindings.some((f) => f.requires_admin));
  let progress = $derived(app.scanProgress.total ? (app.scanProgress.done / app.scanProgress.total) * 100 : 0);

  function entryChecked(f: RuleFinding, path: string) {
    return (app.selectedEntries[f.rule_id] ?? []).includes(path);
  }

  function toggleEntry(f: RuleFinding, path: string) {
    const cur = app.selectedEntries[f.rule_id] ?? [];
    app.selectedEntries[f.rule_id] = cur.includes(path) ? cur.filter((p) => p !== path) : [...cur, path];
  }

  function ruleState(f: RuleFinding): "on" | "off" | "some" {
    if (!f.itemized) return app.selected[f.rule_id] ? "on" : "off";
    const n = (app.selectedEntries[f.rule_id] ?? []).length;
    return n === 0 ? "off" : n === f.top_entries.length ? "on" : "some";
  }

  function toggleRule(f: RuleFinding) {
    if (f.itemized) {
      const on = ruleState(f) !== "off";
      app.selectedEntries[f.rule_id] = on ? [] : f.top_entries.map((e) => e.path);
      if (!on) expanded[f.rule_id] = true;
    } else {
      app.selected[f.rule_id] = !app.selected[f.rule_id];
    }
  }

  function groupState(items: RuleFinding[]): "on" | "off" | "some" {
    const states = items.map(ruleState);
    if (states.every((s) => s === "on")) return "on";
    if (states.every((s) => s === "off")) return "off";
    return "some";
  }

  function toggleGroup(items: RuleFinding[]) {
    const turnOn = groupState(items) !== "on";
    for (const f of items) {
      if (f.itemized) continue; // reviewed items are never bulk-selected
      app.selected[f.rule_id] = turnOn;
    }
  }

  async function scan() {
    error = "";
    try {
      await app.runScan();
    } catch (e) {
      error = errorText(e);
    }
  }

  async function clean() {
    confirming = false;
    error = "";
    try {
      await app.runClean();
    } catch (e) {
      error = errorText(e);
    }
  }

  function startClean() {
    if (needsReview || needsAdmin) confirming = true;
    else clean();
  }

  const shortPath = (p: string) => p.replace(/^C:\\Users\\[^\\]+/i, "~");
</script>

<div class="page">
  <div class="page-head">
    <div>
      <h1>Очистка</h1>
      <p>Кеши, временные файлы, журналы и остатки программ на всех дисках</p>
    </div>
    {#if app.scan && !app.scanning}
      <button class="btn btn-ghost" onclick={scan}><Icon name="refresh" size={16} /> Сканировать заново</button>
    {/if}
  </div>

  {#if error}<div class="error card"><Icon name="alert" size={16} /> {error}</div>{/if}

  {#if app.scanning}
    <section class="card state">
      <Mascot mood="busy" color="#5ce8d0" size={120} spin />
      <h2>Сканирование…</h2>
      <p class="muted current">{app.scanProgress.current || "Подготовка"}</p>
      <div class="progress"><i style="width:{progress}%"></i></div>
      <p class="faint num">{app.scanProgress.done} из {app.scanProgress.total || "…"} проверок</p>
    </section>
  {:else if app.cleaning}
    <section class="card state">
      <Mascot mood="busy" color="#5ce8d0" size={120} spin />
      <h2>Очищаем…</h2>
      <p class="muted">Удаляем только отмеченное. Это займёт несколько секунд.</p>
    </section>
  {:else if app.lastClean}
    {@const r = app.lastClean}
    <section class="card state done">
      <Mascot mood="happy" color="#5ee6a0" size={130} />
      <h2>Освобождено {bytes(r.freed_bytes)}</h2>
      <p class="muted">
        Удалено {count(r.removed_files)} {plural(r.removed_files, "объект", "объекта", "объектов")}.
        {#if r.failed_files}
          {count(r.failed_files)} {plural(r.failed_files, "файл занят", "файла заняты", "файлов заняты")} программами или
          требуют прав администратора, их пропустили.
        {/if}
      </p>
      <button class="btn btn-primary" onclick={scan}>Сканировать снова</button>
    </section>
  {:else if !app.scan}
    <section class="card state">
      <Mascot mood="calm" color="#5ce8d0" size={130} />
      <h2>Найдём, что можно удалить</h2>
      <p class="muted">Сканирование ничего не удаляет. Вы увидите список и сами решите, что очистить.</p>
      <div class="options">
        <label>
          <input
            type="checkbox"
            class="check"
            checked={app.scanOptions.leftovers}
            onchange={(e) => app.setScanOptions({ ...app.scanOptions, leftovers: e.currentTarget.checked })}
          />
          Искать остатки удалённых программ
        </label>
        <label>
          <input
            type="checkbox"
            class="check"
            checked={app.scanOptions.largeFiles}
            onchange={(e) => app.setScanOptions({ ...app.scanOptions, largeFiles: e.currentTarget.checked })}
          />
          Показать крупные файлы на всех дисках
        </label>
      </div>
      <button class="btn btn-primary" onclick={scan}>Сканировать</button>
    </section>
  {:else}
    <div class="summary">
      <div>
        <div class="eyebrow">Можно освободить</div>
        <div class="total num display">{bytes(app.scan.total_bytes)}</div>
        <div class="muted">
          {app.scan.findings.length} {plural(app.scan.findings.length, "пункт", "пункта", "пунктов")}
          · проверено правил: {app.scan.rules_checked}
        </div>
      </div>
      <div class="legend">
        {#each ["safe", "moderate", "expert"] as const as r}
          <span class="badge badge-{r}" title={riskHint[r]}>{riskName[r]}</span>
        {/each}
      </div>
    </div>

    {#each groups as g}
      {@const gs = groupState(g.items)}
      <section class="card group">
        <header class="group-head">
          <input
            type="checkbox"
            class="check"
            checked={gs === "on"}
            indeterminate={gs === "some"}
            onchange={() => toggleGroup(g.items)}
            aria-label="Выбрать категорию"
          />
          <h3>{categoryName[g.category]}</h3>
          <span class="num muted">{bytes(g.bytes)}</span>
        </header>

        {#each g.items as f (f.rule_id)}
          {@const st = ruleState(f)}
          <div class="rule" class:open={expanded[f.rule_id]}>
            <div class="rule-row">
              <input
                type="checkbox"
                class="check"
                checked={st === "on"}
                indeterminate={st === "some"}
                onchange={() => toggleRule(f)}
                aria-label={f.name}
              />
              <button class="rule-main" onclick={() => (expanded[f.rule_id] = !expanded[f.rule_id])}>
                <span class="chev"><Icon name="chevron" size={14} /></span>
                <span class="rule-name">{f.name}</span>
                <span class="badges">
                  {#if f.risk !== "safe"}<span class="badge badge-{f.risk}">{riskName[f.risk]}</span>{/if}
                  {#if f.requires_admin && !elevated}<span class="badge badge-muted" title="Нужны права администратора"><Icon name="shield" size={12} /></span>{/if}
                  {#if f.running_processes.length}
                    <span class="badge badge-moderate" title="Закройте программу, чтобы очистить полностью">Запущено: {f.running_processes[0]}</span>
                  {/if}
                </span>
                <span class="size num">{bytes(f.bytes)}</span>
              </button>
            </div>

            {#if expanded[f.rule_id]}
              <div class="details">
                {#if f.description}<p class="muted">{f.description}</p>{/if}
                {#if f.top_entries.length}
                  <ul class="entries">
                    {#each f.top_entries as e (e.path)}
                      <li>
                        {#if f.itemized}
                          <input type="checkbox" class="check" checked={entryChecked(f, e.path)} onchange={() => toggleEntry(f, e.path)} />
                        {/if}
                        <span class="path" title={e.path}>{shortPath(e.path)}</span>
                        {#if e.note === "possible_saves"}<span class="badge badge-expert">Возможно, сохранения игр</span>{/if}
                        <span class="num muted">{bytes(e.bytes)}</span>
                        <button class="icon-btn" title="Показать в проводнике" onclick={() => api.revealPath(e.path)}>
                          <Icon name="folder" size={15} />
                        </button>
                      </li>
                    {/each}
                  </ul>
                {:else}
                  <p class="faint">{count(f.files)} {plural(f.files, "файл", "файла", "файлов")}</p>
                {/if}
              </div>
            {/if}
          </div>
        {/each}
      </section>
    {/each}

    <div class="spacer"></div>
    <div class="actionbar card">
      <div>
        <div class="faint">Выбрано</div>
        <div class="sel num">{bytes(selectedBytes)}</div>
      </div>
      <button class="btn btn-primary" disabled={selectedBytes === 0} onclick={startClean}>
        <Icon name="trash" size={16} /> Очистить
      </button>
    </div>
  {/if}
</div>

{#if confirming}
  <Modal title="Проверьте перед очисткой" onclose={() => (confirming = false)}>
    {#if needsReview}
      <p>Среди отмеченного есть пункты, которые нужно проверить вручную: остатки программ, крупные файлы или установщики. Они будут удалены безвозвратно, мимо Корзины.</p>
    {/if}
    {#if needsAdmin}
      <p>Часть файлов можно удалить только с правами администратора. Без них эти файлы пропустим. Перезапустить Trufi Cleaner от имени администратора можно в настройках.</p>
    {/if}
    {#snippet actions()}
      <button class="btn btn-ghost" onclick={() => (confirming = false)}>Отмена</button>
      <button class="btn btn-primary" onclick={clean}>Очистить {bytes(selectedBytes)}</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .error { display: flex; gap: 8px; align-items: center; color: var(--bad); background: var(--bad-soft); border-color: transparent; padding: 12px 18px; }

  .state { display: flex; flex-direction: column; align-items: center; text-align: center; padding: 40px 24px 44px; gap: 6px; }
  .state h2 { margin: 14px 0 2px; font-size: 30px; font-weight: 800; letter-spacing: -0.02em; }
  .state p { margin: 0 0 8px; max-width: 520px; font-size: 15px; }
  .options { display: flex; flex-direction: column; gap: 12px; margin: 10px 0 18px; text-align: left; }
  .options label { display: flex; gap: 10px; align-items: center; cursor: pointer; font-weight: 550; }
  .state .btn-primary { height: 46px; padding: 0 28px; font-size: 15px; }
  .current { min-height: 21px; }
  .progress { width: min(440px, 100%); height: 10px; border-radius: 10px; background: var(--track); overflow: hidden; margin: 8px 0; }
  .progress i { display: block; height: 100%; border-radius: 10px; background: linear-gradient(90deg, var(--accent), var(--accent-2)); transition: width 0.2s; }

  .summary { display: flex; justify-content: space-between; align-items: flex-end; gap: 16px; padding: 0 4px; }
  .eyebrow { font-weight: 650; color: var(--text-2); font-size: 15px; }
  .total { font-size: 64px; font-weight: 800; letter-spacing: -0.04em; line-height: 1.05; }
  .legend { display: flex; gap: 6px; }

  .group { padding: 8px 0; }
  .group-head { display: flex; align-items: center; gap: 12px; padding: 12px 22px; }
  .group-head h3 { margin: 0; font-size: 17px; font-weight: 750; flex: 1; }
  .group-head .num { font-weight: 700; color: var(--text); }

  .rule { border-top: 1px solid var(--divider); margin: 0 22px; }
  .rule-row { display: flex; align-items: center; gap: 12px; }
  .rule-main { flex: 1; display: flex; align-items: center; gap: 8px; min-height: 50px; text-align: left; min-width: 0; }
  .chev { color: var(--text-3); transition: transform 0.15s; display: grid; }
  .open .chev { transform: rotate(90deg); }
  .rule-name { flex: 0 1 auto; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; font-weight: 550; }
  .badges { display: flex; gap: 6px; flex: 1; min-width: 0; overflow: hidden; }
  .size { font-weight: 700; white-space: nowrap; }

  .details { padding: 0 0 14px 54px; font-size: 13px; }
  .details p { margin: 0 0 8px; }
  .entries { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 2px; }
  .entries li { display: flex; align-items: center; gap: 10px; padding: 4px 0; }
  .path { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-family: ui-monospace, "Cascadia Mono", Consolas, monospace; font-size: 12px; color: var(--text-2); }
  .icon-btn { color: var(--text-3); display: grid; padding: 5px; border-radius: 999px; }
  .icon-btn:hover { color: var(--text); background: var(--surface-hover); }

  .spacer { flex: 1; }
  .actionbar {
    position: sticky;
    bottom: 0;
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 14px 12px 26px;
    border-radius: 999px;
    background: rgba(62, 88, 175, 0.92);
    box-shadow: 0 12px 32px rgba(15, 25, 80, 0.3);
  }
  .sel { font-size: 22px; font-weight: 800; letter-spacing: -0.01em; }
  .actionbar .btn-primary { height: 46px; padding: 0 26px; font-size: 15px; }
</style>
