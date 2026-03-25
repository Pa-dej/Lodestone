<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
  import LodestoneIcon from "../../icons/Lodestone.svg?raw";
  import ServerIcon from "../../icons/Server.svg?raw";
  import TerminalIcon from "../../icons/Terminal.svg?raw";
  import SettingsIcon from "../../icons/Settings.svg?raw";

  interface NavItem {
    label: string;
    path: string;
    iconSvg: string;
  }

  const navItems: NavItem[] = [
    { label: "Servers", path: "/", iconSvg: ServerIcon },
    { label: "Console", path: "/console", iconSvg: TerminalIcon },
    { label: "Settings", path: "/settings", iconSvg: SettingsIcon },
  ];

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
      <p class="brand-sub">Minecraft Server Launcher</p>
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
    width: 220px;
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
    gap: 20px;
    flex-shrink: 0;
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 6px;
    min-height: 42px;
  }

  .brand-icon {
    width: 30px;
    height: 30px;
    display: grid;
    place-items: center;
    color: var(--accent);
  }

  .brand-icon :global(svg) {
    width: 24px;
    height: 24px;
  }

  .brand {
    font-family: var(--font-display);
    font-style: italic;
    font-weight: 300;
    letter-spacing: -0.02em;
    font-size: 24px;
    line-height: 0.95;
  }

  .brand-sub {
    color: var(--text-hint);
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    margin-top: 2px;
    white-space: nowrap;
  }

  .sidebar-nav {
    display: flex;
    flex-direction: column;
    gap: 8px;
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
      width: 76px;
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
