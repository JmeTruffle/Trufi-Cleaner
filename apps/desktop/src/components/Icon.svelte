<script lang="ts">
  // Small stroke icon set (24px grid), drawn for this app.
  const paths: Record<string, string> = {
    overview: "M4 13h4v7H4zM10 4h4v16h-4zM16 9h4v11h-4z",
    cleaner: "M14 4l6 6M9 9l6 6M4 20l5-1 9-9-4-4-9 9z",
    optimize: "M12 3v4M12 17v4M4.2 7l3.5 2M16.3 15l3.5 2M4.2 17l3.5-2M16.3 9l3.5-2M12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6z",
    drivers: "M8 8h8v8H8zM10 4v4M14 4v4M10 16v4M14 16v4M4 10h4M4 14h4M16 10h4M16 14h4",
    windows: "M4 5.5l7-1v7H4zM13 4.2l7-1.2v8.5h-7zM4 13h7v7l-7-1zM13 13h7v8l-7-1.2z",
    laptop: "M5 5h14v10H5zM2.5 19h19",
    lock: "M6 11h12v9H6zM8.5 11V8a3.5 3.5 0 0 1 7 0v3",
    settings: "M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8zM3 12h2M19 12h2M12 3v2M12 19v2M5.6 5.6l1.4 1.4M17 17l1.4 1.4M5.6 18.4L7 17M17 7l1.4-1.4",
    shield: "M12 3l8 3v6c0 4.5-3.4 8.2-8 9-4.6-.8-8-4.5-8-9V6z",
    chevron: "M9 6l6 6-6 6",
    folder: "M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z",
    alert: "M12 9v4M12 17h.01M10.3 3.9L2.4 18a2 2 0 0 0 1.7 3h15.8a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z",
    check: "M5 12l5 5 9-10",
    info: "M12 16v-4M12 8h.01M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z",
    disk: "M4 6h16v12H4zM8 14h.01M12 14h4",
    sparkle: "M12 3l1.9 5.1L19 10l-5.1 1.9L12 17l-1.9-5.1L5 10l5.1-1.9zM19 17l.8 2.2L22 20l-2.2.8L19 23l-.8-2.2L16 20l2.2-.8z",
    trash: "M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3",
    search: "M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14zM20 20l-4-4",
    min: "M5 12h14",
    max: "M5 5h14v14H5z",
    close: "M6 6l12 12M18 6L6 18",
    refresh: "M20 11a8 8 0 1 0-2.3 5.7M20 4v7h-7",
  };
  let { name, size = 18, stroke = 1.8 }: { name: string; size?: number; stroke?: number } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
><path d={paths[name] ?? ""} /></svg>
