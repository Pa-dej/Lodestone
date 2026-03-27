<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
  import LodestoneIcon from "../../icons/Lodestone.svg?raw";
  import ServerIcon from "../../icons/Server.svg?raw";
  import TerminalIcon from "../../icons/Terminal.svg?raw";
  import SettingsIcon from "../../icons/Settings.svg?raw";
  import { t } from "$lib/stores/i18n.svelte";

  interface NavItem {
    label: string;
    path: string;
    iconSvg: string;
  }

  const navItems = $derived<NavItem[]>([
    { label: t("nav_servers"), path: "/", iconSvg: ServerIcon },
    { label: t("nav_console"), path: "/console", iconSvg: TerminalIcon },
    { label: t("nav_settings"), path: "/settings", iconSvg: SettingsIcon },
  ]);

  const currentPath = $derived($page.url.pathname);

  function isActive(path: string): boolean {
    return currentPath === path;
  }
</script>

<aside class="sidebar panel">
  <header class="sidebar-header">
    <span class="brand-icon">{@html LodestoneIcon}</span>
    <div class="brand-text">
      <h1 class="brand">Lodestone</h1>
    </div>
  </header>

  <nav class="sidebar-nav">
    {#each navItems as item}
      <button
        type="button"
        class="btn btn-ghost sidebar-link"
        class:active={isActive(item.path)}
        title={item.label}
        onclick={() => goto(item.path)}
      >
        <span class="sidebar-icon">{@html item.iconSvg}</span>
        <span class="sidebar-label">{item.label}</span>
      </button>
    {/each}
  </nav>
</aside>

<style>
  .sidebar {
    width: 100%;
    height: 100vh;
    border-radius: 0;
    border-right: 0.5px solid var(--border);
    border-left: none;
    border-top: none;
    border-bottom: none;
    background: var(--surface);
    display: flex;
    flex-direction: column;
    padding: 14px 10px;
    gap: 16px;
    flex-shrink: 0;
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 6px;
    min-height: 56px;
  }

  .brand-icon {
    width: 42px;
    height: 42px;
    display: grid;
    place-items: center;
    color: var(--accent);
    flex-shrink: 0;
  }

  .brand-icon :global(svg) {
    width: 36px;
    height: 36px;
  }

  .brand {
    font-family: var(--font-display);
    font-style: italic;
    font-weight: 300;
    letter-spacing: -0.02em;
    font-size: 28px;
    line-height: 1;
    user-select: none;
  }

  .sidebar-nav {
    display: flex;
    flex-direction: column;
    gap: 8px;
    user-select: none;
  }

  .sidebar-link {
    justify-content: flex-start;
    width: 100%;
    color: var(--text-muted);
    min-height: 38px;
  }

  .sidebar-link.active {
    border-color: var(--accent);
    color: var(--accent);
    background: var(--accent-bg);
  }

  .sidebar-icon {
    width: 16px;
    height: 16px;
    display: grid;
    place-items: center;
  }

  .sidebar-icon :global(svg) {
    width: 15px;
    height: 15px;
  }

  @media (max-width: 920px) {
    .sidebar {
      padding-inline: 8px;
    }

    .brand-text {
      display: none;
    }

    .sidebar-header {
      justify-content: center;
      padding: 0;
    }

    .sidebar-link {
      justify-content: center;
      padding-inline: 0;
    }

    .sidebar-label {
      display: none;
    }
  }
</style>
