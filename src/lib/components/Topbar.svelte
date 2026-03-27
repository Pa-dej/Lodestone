<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/stores";
  import { i18nState, initializeLanguage, setLanguage, t, type UiLanguage } from "$lib/stores/i18n.svelte";

  type ThemeMode = "dark" | "light" | "system";

  interface ThemeOption {
    id: ThemeMode;
    icon: string;
    labelKey: "theme_light" | "theme_dark" | "theme_system";
  }

  const SunIcon = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><g fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><circle cx="12" cy="12" r="4"/><path d="M12 2v2m0 16v2M4.93 4.93l1.41 1.41m11.32 11.32l1.41 1.41M2 12h2m16 0h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41"/></g></svg>`;
  const MoonIcon = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3a6 6 0 0 0 9 9a9 9 0 1 1-9-9m7 0v4m2-2h-4"/></svg>`;
  const MonitorIcon = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><g fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><rect width="20" height="14" x="2" y="3" rx="2"/><path d="M8 21h8m-4-4v4"/></g></svg>`;

  const themeOptions: ThemeOption[] = [
    { id: "light", icon: SunIcon, labelKey: "theme_light" },
    { id: "dark", icon: MoonIcon, labelKey: "theme_dark" },
    { id: "system", icon: MonitorIcon, labelKey: "theme_system" },
  ];

  const pageLabel = $derived.by(() => {
    const path = $page.url.pathname;
    if (path === "/console") {
      return t("page_console");
    }
    if (path === "/settings") {
      return t("page_settings");
    }
    return t("page_servers");
  });

  let themeMode = $state<ThemeMode>("system");
  let mediaQuery: MediaQueryList | null = null;
  let mediaListener: ((event: MediaQueryListEvent) => void) | null = null;

  const activeTheme = $derived(themeOptions.find((option) => option.id === themeMode) ?? themeOptions[2]);
  const activeThemeLabel = $derived(t(activeTheme.labelKey));

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

  function cycleTheme(): void {
    const index = themeOptions.findIndex((option) => option.id === themeMode);
    const next = themeOptions[(index + 1) % themeOptions.length];
    applyTheme(next.id);
  }

  function setUiLanguage(language: UiLanguage): void {
    setLanguage(language);
  }

  onMount(() => {
    initializeLanguage();
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
  <div class="topbar-page">
    <div class="topbar-title">{pageLabel}</div>
  </div>

  <div class="topbar-controls">
    <button
      type="button"
      class="btn btn-ghost btn-sm carousel-theme-btn"
      title={activeThemeLabel}
      aria-label={activeThemeLabel}
      onclick={cycleTheme}
    >
      <span class="theme-icon">{@html activeTheme.icon}</span>
    </button>

    <div class="lang-switcher" role="group" aria-label={t("language_toggle")}>
      <button
        type="button"
        class="lang-btn"
        class:active={i18nState.language === "en"}
        onclick={() => setUiLanguage("en")}
      >
        ENG
      </button>
      <button
        type="button"
        class="lang-btn"
        class:active={i18nState.language === "ru"}
        onclick={() => setUiLanguage("ru")}
      >
        RUS
      </button>
    </div>
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

  .topbar-title {
    font-family: var(--font-display);
    font-size: 20px;
    font-style: italic;
    font-weight: 300;
    line-height: 1;
    margin-top: 3px;
    user-select: none;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .topbar-page {
    min-width: 0;
    flex: 1 1 auto;
  }

  .topbar-controls {
    display: inline-flex;
    gap: 6px;
    align-items: center;
    user-select: none;
    flex: 0 0 auto;
  }

  .carousel-theme-btn {
    width: 36px;
    height: 36px;
    padding: 0;
  }

  .theme-icon {
    width: 16px;
    height: 16px;
    display: grid;
    place-items: center;
  }

  .theme-icon :global(svg) {
    width: 16px;
    height: 16px;
    display: block;
  }

  .lang-switcher {
    display: inline-flex;
    border: 0.5px solid var(--border);
    border-radius: var(--r-md);
    overflow: hidden;
    background: var(--surface);
    user-select: none;
  }

  .lang-btn {
    min-width: 52px;
    padding: 7px 10px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    font-size: 11px;
    cursor: pointer;
    transition: background var(--tr), color var(--tr);
    letter-spacing: 0.05em;
  }

  .lang-btn + .lang-btn {
    border-left: 0.5px solid var(--border);
  }

  .lang-btn.active {
    background: var(--accent-bg);
    color: var(--accent);
  }

  .lang-btn:hover:not(.active) {
    color: var(--text);
  }

  @media (max-width: 760px) {
    .app-topbar {
      padding-inline: 10px;
      gap: 8px;
    }

    .topbar-title {
      font-size: 16px;
      margin-top: 0;
    }

    .lang-btn {
      min-width: 44px;
      padding: 6px 8px;
      font-size: 10px;
    }
  }

  @media (max-width: 560px) {
    .topbar-title {
      font-size: 14px;
    }

    .carousel-theme-btn {
      width: 32px;
      height: 32px;
    }

    .lang-btn {
      min-width: 38px;
      padding: 5px 6px;
      font-size: 10px;
      letter-spacing: 0.03em;
    }
  }
</style>
