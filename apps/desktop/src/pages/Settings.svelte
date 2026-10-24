<script lang="ts">
  import Icon from "../components/Icon.svelte";
  import { api } from "../lib/api";
  import { errorText } from "../lib/i18n";
  import { app, type Theme } from "../lib/store.svelte";

  let error = $state("");
  const themes: { id: Theme; label: string }[] = [
    { id: "system", label: "Как в системе" },
    { id: "light", label: "Светлая" },
    { id: "dark", label: "Тёмная" },
  ];

  async function elevate() {
    error = "";
    try {
      await api.relaunchElevated();
    } catch (e) {
      error = errorText(e);
    }
  }
</script>

<div class="page">
  <div class="page-head">
    <div>
      <h1>Настройки</h1>
      <p>Оформление, права и правила очистки</p>
    </div>
  </div>

  <section class="card">
    <div class="card-title">Тема</div>
    <div class="themes">
      {#each themes as t}
        <button class="theme" class:on={app.theme === t.id} onclick={() => app.setTheme(t.id)}>
          <span class="swatch {t.id}"></span>{t.label}
        </button>
      {/each}
    </div>
  </section>

  <section class="card row">
    <div>
      <div class="card-title">Права администратора</div>
      <p class="muted">
        {#if app.info?.is_elevated}
          Trufi Cleaner запущен от имени администратора: доступна очистка системных папок и общей автозагрузки.
        {:else}
          Без прав администратора системные папки Windows (Temp, обновления, журналы) пропускаются.
        {/if}
      </p>
      {#if error}<p class="err">{error}</p>{/if}
    </div>
    {#if !app.info?.is_elevated && app.info?.os === "windows"}
      <button class="btn btn-ghost" onclick={elevate}><Icon name="shield" size={16} /> Перезапустить как администратор</button>
    {/if}
  </section>

  <section class="card">
    <div class="card-title">Правила очистки</div>
    <p class="muted">
      Загружено правил: <b>{app.info?.rule_count ?? "–"}</b>
      {#if app.info?.rule_packs.length}({app.info.rule_packs.join(", ")}){/if}
    </p>
    {#if app.info?.user_rules_dir}
      <p class="muted">
        Свои правила можно добавить файлами <code>.toml</code> в папку
        <button class="link" onclick={() => app.info?.user_rules_dir && api.revealPath(app.info.user_rules_dir)}>{app.info.user_rules_dir}</button>
      </p>
    {/if}
    {#each app.info?.rule_errors ?? [] as e}<p class="err">{e}</p>{/each}
  </section>

  <section class="card">
    <div class="card-title">О программе</div>
    <p class="muted">Trufi Cleaner {app.info?.app_version ?? ""} · {app.info?.os_label ?? ""}</p>
    <p class="faint">Удаляется только то, что показано и отмечено. Системные папки, документы и данные профилей браузеров защищены всегда.</p>
  </section>
</div>

<style>
  .themes { display: flex; gap: 10px; }
  .theme { display: flex; align-items: center; gap: 10px; padding: 10px 14px; border-radius: 10px; border: 1px solid var(--border); background: var(--surface-2); font-weight: 550; }
  .theme.on { border-color: var(--accent); background: var(--accent-soft); color: var(--accent); }
  .swatch { width: 18px; height: 18px; border-radius: 6px; border: 1px solid var(--border); }
  .swatch.light { background: #f4f5f7; }
  .swatch.dark { background: #171a20; }
  .swatch.system { background: linear-gradient(135deg, #f4f5f7 50%, #171a20 50%); }
  .row { display: flex; justify-content: space-between; align-items: center; gap: 24px; }
  p { margin: 0 0 6px; }
  .err { color: var(--bad); }
  code { background: var(--track); padding: 1px 5px; border-radius: 5px; }
  .link { color: var(--accent); text-decoration: underline; text-underline-offset: 2px; word-break: break-all; text-align: left; }
</style>
