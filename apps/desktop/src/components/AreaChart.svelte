<script lang="ts">
  // Live area chart for a rolling window. Values are plotted right-aligned so
  // the newest sample is always at the right edge.
  let {
    values,
    max = 100,
    color = "var(--accent)",
    height = 72,
    capacity = 60,
  }: { values: number[]; max?: number; color?: string; height?: number; capacity?: number } = $props();

  const W = 300;
  const id = `g${Math.random().toString(36).slice(2, 8)}`;
  let top = $derived(Math.max(max, ...values, 1));
  // Until the window fills up, spread the samples wider so a fresh chart
  // is not a sliver at the right edge.
  let slots = $derived(Math.min(capacity, Math.max(values.length, 12)));
  let points = $derived(
    values.map((v, i) => {
      const x = W - (values.length - 1 - i) * (W / (slots - 1));
      const y = height - (Math.min(v, top) / top) * (height - 4) - 2;
      return [x, y] as const;
    }),
  );
  let line = $derived(points.map(([x, y], i) => `${i ? "L" : "M"}${x.toFixed(1)},${y.toFixed(1)}`).join(""));
  let area = $derived(points.length ? `${line}L${W},${height}L${points[0][0].toFixed(1)},${height}Z` : "");
</script>

<svg viewBox="0 0 {W} {height}" preserveAspectRatio="none" style="height:{height}px" aria-hidden="true">
  <defs>
    <linearGradient {id} x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color={color} stop-opacity="0.28" />
      <stop offset="1" stop-color={color} stop-opacity="0" />
    </linearGradient>
  </defs>
  {#each [0.25, 0.5, 0.75] as g}
    <line x1="0" x2={W} y1={height * g} y2={height * g} stroke="var(--border)" stroke-width="1" vector-effect="non-scaling-stroke" />
  {/each}
  {#if points.length > 1}
    <path d={area} fill="url(#{id})" />
    <path d={line} fill="none" stroke={color} stroke-width="2" vector-effect="non-scaling-stroke" stroke-linejoin="round" />
  {/if}
</svg>

<style>
  svg { width: 100%; display: block; }
</style>
