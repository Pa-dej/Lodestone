<script lang="ts">
  import type { CoreType, ServerConfig } from "$lib/types";
  import PlayIcon from "../../icons/Play.svg?raw";
  import TerminalIcon from "../../icons/Terminal.svg?raw";
  import PaperIcon from "../../icons/servers/Paper.svg?raw";
  import PurpurIcon from "../../icons/servers/Purpur.svg?raw";
  import FabricIcon from "../../icons/servers/Fabric.svg?raw";
  import ForgeIcon from "../../icons/servers/Forge.svg?raw";
  import FoliaIcon from "../../icons/servers/Folia.svg?raw";
  import VanillaIcon from "../../icons/Server.svg?raw";

  interface Props {
    server: ServerConfig;
    onStart: (id: string) => void;
    onStop: (id: string) => void;
    onOpenConsole: (id: string) => void;
  }

  interface CoreVisual {
    iconSvg: string;
    color: string;
  }

  let { server, onStart, onStop, onOpenConsole }: Props = $props();

  const coreVisuals: Record<CoreType, CoreVisual> = {
    paper: { iconSvg: PaperIcon, color: "var(--core-paper)" },
    purpur: { iconSvg: PurpurIcon, color: "var(--core-purpur)" },
    fabric: { iconSvg: FabricIcon, color: "var(--core-fabric)" },
    forge: { iconSvg: ForgeIcon, color: "var(--core-forge)" },
    folia: { iconSvg: FoliaIcon, color: "var(--core-folia)" },
    vanilla: { iconSvg: VanillaIcon, color: "var(--core-vanilla)" },
  };
</script>

<article class="card server-card" class:running={server.running}>
  <div class="server-head">
    <div class="server-icon" class:running={server.running} style={`color:${coreVisuals[server.core].color}`}>
      {@html coreVisuals[server.core].iconSvg}
    </div>
    <span class="badge badge-muted badge-dot" class:badge-running={server.running}>
      {server.running ? "Running" : "Stopped"}
    </span>
  </div>

  <h3 class="server-name">{server.name}</h3>
  <p class="server-meta">{server.core} · {server.version} · :{server.port}</p>

  <div class="server-actions">
    {#if server.running}
      <button type="button" class="btn btn-danger btn-sm" onclick={() => onStop(server.id)}>
        ■ Stop
      </button>
    {:else}
      <button type="button" class="btn btn-primary btn-sm" onclick={() => onStart(server.id)}>
        <span class="btn-icon-inline">{@html PlayIcon}</span>
        <span>Start</span>
      </button>
    {/if}
    <button type="button" class="btn btn-secondary btn-sm" onclick={() => onOpenConsole(server.id)}>
      <span class="btn-icon-inline">{@html TerminalIcon}</span>
      <span>Console</span>
    </button>
  </div>
</article>

<style>
  .server-card {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-height: 180px;
  }

  .server-card.running {
    border-color: var(--accent);
  }

  .server-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .server-icon {
    width: 42px;
    height: 42px;
    border-radius: var(--r-md);
    background: var(--surface-2);
    display: grid;
    place-items: center;
    font-size: 20px;
    border: 0.5px solid var(--border);
  }

  .server-icon :global(svg) {
    width: 20px;
    height: 20px;
  }

  .server-icon.running {
    background: var(--accent-bg);
    border-color: var(--accent);
  }

  .badge-running {
    color: var(--success-color);
    border-color: var(--success-border);
    background: var(--success-bg);
  }

  .server-name {
    font-size: 18px;
    color: var(--text);
    line-height: 1.2;
  }

  .server-meta {
    font-size: 11px;
    color: var(--text-muted);
    text-transform: lowercase;
  }

  .server-actions {
    margin-top: auto;
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .btn-icon-inline {
    width: 12px;
    height: 12px;
    display: grid;
    place-items: center;
  }

  .btn-icon-inline :global(svg) {
    width: 12px;
    height: 12px;
    stroke-width: 2.5;
  }
</style>
