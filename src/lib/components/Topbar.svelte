<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/stores";

  type ThemeMode = "dark" | "light" | "system";

  interface ThemeOption {
    id: ThemeMode;
    label: string;
  }

  const themeOptions: ThemeOption[] = [
    { id: "dark", label: "Темная" },
    { id: "light", label: "Светлая" },
    { id: "system", label: "Система" },
  ];

  const pageLabel = $derived.by(() => {
    const path = $page.url.pathname;
    if (path === "/console") {
      return "Console";
    }
    if (path === "/settings") {
      return "Settings";
    }
    return "Servers";
  });

  let themeMode = $state<ThemeMode>("system");
  let mediaQuery: MediaQueryList | null = null;
  let mediaListener: ((event: MediaQueryListEvent) => void) | null = null;

  function resolvedTheme(mode: ThemeMode): "dark" | "light" {
    if (mode === "system") {
      return mediaQuery?.matches ? "dark" : "light";
    }
    return mode;
  }

  function applyTheme(mode: ThemeMode, persist = true): void {
    themeMode = mode;
    document.documentElement.dataset.accent = "pine";
    document.documentElement.dataset.theme = resolvedTheme(mode);
    if (persist) {
      localStorage.setItem("lodestone-theme", mode);
    }
  }

  onMount(() => {
    mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    mediaListener = () => {
      if (themeMode === "system") {
        applyTheme("system", false);
      }
    };
    mediaQuery.addEventListener("change", mediaListener);

    const saved = (localStorage.getItem("lodestone-theme") as ThemeMode | null) ?? "system";
    if (saved === "dark" || saved === "light" || saved === "system") {
      applyTheme(saved, false);
    } else {
      applyTheme("system", false);
    }

    return () => {
      if (mediaQuery && mediaListener) {
        mediaQuery.removeEventListener("change", mediaListener);
      }
    };
  });
</script>

<header class="app-topbar">
  <div>
    <div class="topbar-eyebrow">Lodestone</div>
    <div class="topbar-title">{pageLabel}</div>
  </div>

  <div class="theme-switcher">
    {#each themeOptions as option}
      <button
        type="button"
        class="btn btn-ghost btn-sm theme-btn"
        class:active={option.id === themeMode}
        onclick={() => applyTheme(option.id)}
      >
        {option.label}
      </button>
    {/each}
  </div>
</header>

<style>
  .app-topbar {
    height: 44px;
    border-bottom: 0.5px solid var(--border-soft);
    background: var(--bg);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 16px;
    position: sticky;
    top: 0;
    z-index: 20;
    gap: 12px;
  }

  .topbar-eyebrow {
    color: var(--text-hint);
    font-size: 10px;
    letter-spacing: 0.11em;
    text-transform: uppercase;
    line-height: 1;
  }

  .topbar-title {
    font-family: var(--font-display);
    font-size: 20px;
    font-style: italic;
    font-weight: 300;
    line-height: 1;
    margin-top: 3px;
  }

  .theme-switcher {
    display: inline-flex;
    gap: 6px;
  }

  .theme-btn {
    min-width: 74px;
  }

  .theme-btn.active {
    background: var(--accent-bg);
    border-color: var(--accent);
    color: var(--accent);
  }

  @media (max-width: 760px) {
    .topbar-eyebrow {
      display: none;
    }

    .topbar-title {
      font-size: 16px;
      margin-top: 0;
    }

    .theme-btn {
      min-width: 0;
      padding: 6px 9px;
      font-size: 11px;
    }
  }
</style>
