<script lang="ts">
  import type { Snippet } from "svelte";
  let { title, onclose, children, actions }: { title: string; onclose: () => void; children: Snippet; actions: Snippet } = $props();
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onclose}>
  <div class="modal card" role="dialog" tabindex="-1" aria-modal="true" aria-label={title} onclick={(e) => e.stopPropagation()}>
    <h2>{title}</h2>
    <div class="body">{@render children()}</div>
    <div class="actions">{@render actions()}</div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(20, 30, 80, 0.4);
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
    display: grid;
    place-items: center;
    z-index: 50;
    animation: fade 0.12s ease;
  }
  .modal { width: min(480px, calc(100vw - 32px)); padding: 26px 28px; background: rgba(70, 100, 190, 0.92); }
  h2 { margin: 0 0 10px; font-size: 22px; font-weight: 800; letter-spacing: -0.01em; }
  .body { color: var(--text-2); }
  .actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 20px; }
  @keyframes fade { from { opacity: 0; } }
</style>
