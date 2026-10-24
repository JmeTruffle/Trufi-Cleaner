<script lang="ts">
  let { score, level, size = 148 }: { score: number; level: "good" | "fair" | "poor"; size?: number } = $props();
  const R = 54;
  const C = 2 * Math.PI * R;
  let color = $derived(level === "good" ? "var(--good)" : level === "fair" ? "var(--warn)" : "var(--bad)");
</script>

<div class="ring" style="width:{size}px;height:{size}px">
  <svg viewBox="0 0 128 128">
    <circle cx="64" cy="64" r={R} fill="none" stroke="var(--track)" stroke-width="10" />
    <circle
      cx="64"
      cy="64"
      r={R}
      fill="none"
      stroke={color}
      stroke-width="10"
      stroke-linecap="round"
      stroke-dasharray={C}
      stroke-dashoffset={C * (1 - score / 100)}
      transform="rotate(-90 64 64)"
    />
  </svg>
  <div class="label">
    <div class="score num">{score}</div>
    <div class="of">из 100</div>
  </div>
</div>

<style>
  .ring { position: relative; flex: none; }
  svg { width: 100%; height: 100%; }
  circle { transition: stroke-dashoffset 0.8s ease, stroke 0.3s; }
  .label { position: absolute; inset: 0; display: grid; place-content: center; text-align: center; }
  .score { font-size: 38px; font-weight: 700; letter-spacing: -0.02em; line-height: 1; }
  .of { font-size: 12px; color: var(--text-3); margin-top: 4px; }
</style>
