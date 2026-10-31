<script lang="ts">
  import { onMount } from "svelte";
  import { api, inTauri } from "../lib/api";
  import { isMac, isWindows } from "../lib/os";
  import { app, type Page } from "../lib/store.svelte";
  import Icon from "./Icon.svelte";

  let nav: HTMLElement;

  /** A full-window mask with a rounded hole where the sidebar is. */
  function holeMask(r: DOMRect, radius: number) {
    const W = window.innerWidth, H = window.innerHeight;
    const { x, y, width: w, height: h } = r;
    const k = Math.min(radius, w / 2, h / 2);
    const hole =
      `M${x + k} ${y}H${x + w - k}A${k} ${k} 0 0 1 ${x + w} ${y + k}V${y + h - k}` +
      `A${k} ${k} 0 0 1 ${x + w - k} ${y + h}H${x + k}A${k} ${k} 0 0 1 ${x} ${y + h - k}V${y + k}A${k} ${k} 0 0 1 ${x + k} ${y}Z`;
    const svg = `<svg xmlns='http://www.w3.org/2000/svg' width='${W}' height='${H}' viewBox='0 0 ${W} ${H}'>` +
      `<path fill='black' fill-rule='evenodd' d='M0 0H${W}V${H}H0Z${hole}'/></svg>`;
    return `url("data:image/svg+xml,${encodeURIComponent(svg)}")`;
  }

  // macOS 26+: ask for the system's Liquid Glass under the sidebar and keep
  // it in place. Older macOS answers false and the CSS glass stays.
  onMount(() => {
    if (!isMac || !inTauri) return;
    const radius = parseFloat(getComputedStyle(nav).borderTopLeftRadius) || 26;
    let native = true;
    const place = async () => {
      if (!native) return;
      const r = nav.getBoundingClientRect();
      native = await api
        .setGlassFrame({ x: r.x, y: r.y, width: r.width, height: r.height, radius })
        .catch(() => false);
      const root = document.documentElement;
      if (native) {
        root.style.setProperty("--glass-hole", holeMask(r, radius));
        root.classList.add("native-glass");
      } else {
        root.classList.remove("native-glass");
      }
    };
    place();
    const ro = new ResizeObserver(place);
    ro.observe(nav);
    window.addEventListener("resize", place);
    return () => {
      ro.disconnect();
      window.removeEventListener("resize", place);
    };
  });

  // Drivers are a Windows page: macOS updates drivers with the system.
  const items: { id: Page; label: string }[] = [
    { id: "overview", label: "Обзор" },
    { id: "cleaner", label: "Очистка" },
    { id: "optimize", label: "Оптимизация" },
    ...(isWindows ? [{ id: "drivers" as Page, label: "Драйверы" }] : []),
    { id: "settings", label: "Настройки" },
  ];
</script>

<nav class="sidebar" bind:this={nav}>
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
    -webkit-backdrop-filter: blur(18px);
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
