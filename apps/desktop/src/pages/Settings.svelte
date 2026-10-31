<script lang="ts">
  import Icon from "../components/Icon.svelte";
  import { api } from "../lib/api";
  import { errorText } from "../lib/i18n";
  import { isMac } from "../lib/os";
  import { app } from "../lib/store.svelte";

  let error = $state("");

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
      <p>Обновления, {isMac ? "доступ к диску" : "права"} и правила очистки</p>
    </div>
  </div>

  <section class="card row">
    <div>
      <div class="card-title">Обновления</div>
      <p class="muted">
        Установлена версия <b>{app.info?.app_version ?? "–"}</b>.
        {#if app.update}
          Доступна версия {app.update.version}.
        {:else if app.updateState === "checking"}
          Проверяем…
        {:else if app.updateState === "none"}
          Это последняя версия.
        {:else if app.updateState === "error"}
          Не удалось проверить обновления. Проверьте подключение к интернету.
        {/if}
      </p>
      <p class="faint">Приложение само проверяет обновления при запуске и каждые 6 часов.</p>
    </div>
    {#if app.update}
      <button class="btn btn-primary" disabled={app.updateState === "downloading"} onclick={() => app.installUpdate()}>Обновить</button>
    {:else}
      <button class="btn btn-ghost" disabled={app.updateState === "checking"} onclick={() => app.checkUpdate()}>
        <Icon name="refresh" size={16} /> Проверить
      </button>
    {/if}
  </section>

  {#if isMac}
  <section class="card row">
    <div>
      <div class="card-title">Полный доступ к диску</div>
      <p class="muted">
        {#if app.info?.full_disk_access}
          Разрешён: Trufi Cleaner видит Корзину, кеш Safari, вложения Почты и данные программ из App Store.
        {:else}
          Без него macOS не даёт заглянуть в Корзину, кеш Safari, вложения Почты и данные программ из App Store, эти пункты пропускаются.
          Включите Trufi Cleaner в списке «Полный доступ к диску» и перезапустите приложение.
        {/if}
      </p>
      <p class="faint">Документы, Фото, Почта, iCloud Drive и облачные папки защищены всегда, независимо от доступа.</p>
    </div>
    {#if !app.info?.full_disk_access}
      <button class="btn btn-ghost" onclick={() => api.openSystemPage("full_disk_access").catch((e) => (error = errorText(e)))}>
        <Icon name="lock" size={16} /> Открыть настройки
      </button>
    {/if}
  </section>
  {:else}
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
  {/if}

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
  .row { display: flex; justify-content: space-between; align-items: center; gap: 24px; }
  p { margin: 0 0 6px; }
  .err { color: var(--bad); }
  code { background: var(--track); padding: 1px 6px; border-radius: 6px; }
  .link { color: var(--accent); text-decoration: underline; text-underline-offset: 2px; word-break: break-all; text-align: left; }
</style>
