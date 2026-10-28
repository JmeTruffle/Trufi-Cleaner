<script lang="ts">
  // Friendly wavy blob that shows how the computer feels. Drawn in code so
  // its colour and face can follow the health level.
  type Mood = "happy" | "calm" | "worried" | "sad" | "busy";
  let {
    mood = "happy",
    color = "#5ce8b0",
    size = 160,
    spin = false,
  }: { mood?: Mood; color?: string; size?: number; spin?: boolean } = $props();

  // Polar outline with gentle scallops, like a cookie or a little sun.
  function blob(waves: number, depth: number) {
    const pts: string[] = [];
    const n = 180;
    for (let i = 0; i <= n; i++) {
      const t = (i / n) * Math.PI * 2;
      const r = 44 * (1 + depth * Math.sin(waves * t));
      pts.push(`${(50 + r * Math.cos(t)).toFixed(2)},${(50 + r * Math.sin(t)).toFixed(2)}`);
    }
    return `M${pts.join("L")}Z`;
  }
  // Calm moods are softly scalloped, a stressed one gets spikier.
  let spiky = $derived(mood === "worried" || mood === "sad");
  let outline = $derived(blob(spiky ? 14 : 9, mood === "sad" ? 0.065 : spiky ? 0.05 : 0.035));
</script>

<div class="mascot" style="width:{size}px;height:{size}px">
  <svg viewBox="0 0 100 100" aria-hidden="true">
    <path class="body" class:spin d={outline} fill={color} />
    <g class="face">
      {#if mood === "busy"}
        <rect x="44" y="40" width="5" height="7" rx="1.6" fill="#1b1d2a" />
        <rect x="55" y="40" width="5" height="7" rx="1.6" fill="#1b1d2a" />
        <ellipse cx="52" cy="55" rx="3.2" ry="3.6" fill="#fff" />
      {:else if mood === "sad"}
        <rect x="38" y="41" width="5" height="5" rx="1.6" fill="#1b1d2a" />
        <rect x="50" y="41" width="5" height="5" rx="1.6" fill="#1b1d2a" />
        <path d="M41.5 56a5 4 0 0 1 10 0" fill="none" stroke="#fff" stroke-width="2.6" stroke-linecap="round" />
      {:else if mood === "worried"}
        <rect x="38" y="40" width="5" height="6" rx="1.6" fill="#1b1d2a" />
        <rect x="50" y="40" width="5" height="6" rx="1.6" fill="#1b1d2a" />
        <rect x="42" y="52" width="9" height="2.8" rx="1.4" fill="#fff" />
      {:else}
        <rect x="38" y="39" width="5" height="7" rx="1.6" fill="#1b1d2a" />
        <rect x="50" y="39" width="5" height="7" rx="1.6" fill="#1b1d2a" />
        {#if mood === "happy"}
          <path d="M41.5 51h10a5 5 0 0 1-10 0z" fill="#fff" />
        {:else}
          <path d="M42.5 51.5h8a4 3 0 0 1-8 0z" fill="#fff" />
        {/if}
      {/if}
    </g>
  </svg>
</div>

<style>
  .mascot { flex: none; animation: float 4s ease-in-out infinite; }
  svg { width: 100%; height: 100%; overflow: visible; filter: drop-shadow(0 12px 24px rgba(15, 25, 80, 0.25)); }
  .body { transition: fill 0.4s; transform-origin: 50px 50px; }
  .spin { animation: spin 6s linear infinite; }
  @keyframes float { 50% { transform: translateY(-6px); } }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) {
    .mascot, .spin { animation: none; }
  }
</style>
