<script lang="ts">
  let { size = 22 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 32 32" aria-hidden="true">
  <defs>
    <linearGradient id="trufi-g" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#2cc5b1" />
      <stop offset="1" stop-color="#0b8577" />
    </linearGradient>
  </defs>
  <rect width="32" height="32" rx="9" fill="url(#trufi-g)" />
  <path d="M9 11.5h14M16 11.5V24" stroke="#fff" stroke-width="3" stroke-linecap="round" />
  <circle cx="23" cy="20.5" r="2.2" fill="#fff" opacity="0.85" />
</svg>
