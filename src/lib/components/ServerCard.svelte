<script lang="ts">
  import type { CoreType, ServerConfig } from "$lib/types";
  import PlayIcon from "../../icons/Play.svg?raw";
  import TerminalIcon from "../../icons/Terminal.svg?raw";
  import RestartIcon from "../../icons/Restart.svg?raw";
  import UsersIcon from "../../icons/Users.svg?raw";
  import TrashIcon from "../../icons/Trash.svg?raw";
  import FolderOpenIcon from "../../icons/FolderOpen.svg?raw";
  import EditIcon from "../../icons/Edit.svg?raw";
  import PaperIcon from "../../icons/servers/Paper.svg?raw";
  import PurpurIcon from "../../icons/servers/Purpur.svg?raw";
  import FabricIcon from "../../icons/servers/Fabric.svg?raw";
  import QuiltIcon from "../../icons/servers/Quilt.svg?raw";
  import ForgeIcon from "../../icons/servers/Forge.svg?raw";
  import FoliaIcon from "../../icons/servers/Folia.svg?raw";
  import WaterfallIcon from "../../icons/servers/Waterfall.svg?raw";
  import VanillaIcon from "../../icons/Server.svg?raw";
  import { t } from "$lib/stores/i18n.svelte";

  import { isServerRestarting } from "$lib/stores/servers.svelte";

  interface Props {
    server: ServerConfig;
    onStart: (id: string) => void;
    onStop: (id: string) => void;
    onRestart: (id: string) => void;
    onDelete: (id: string) => void;
    onOpenFolder: (id: string) => void;
    onEdit: (id: string) => void;
    onOpenConsole: (id: string) => void;
  }

  interface CoreVisual {
    iconSvg: string;
    color: string;
  }

  let { server, onStart, onStop, onRestart, onDelete, onOpenFolder, onEdit, onOpenConsole }: Props = $props();

  const isRestarting = $derived(isServerRestarting(server.id));

  const coreVisuals: Record<CoreType, CoreVisual> = {
    paper: { iconSvg: PaperIcon, color: "var(--core-paper)" },
    purpur: { iconSvg: PurpurIcon, color: "var(--core-purpur)" },
    fabric: { iconSvg: FabricIcon, color: "var(--core-fabric)" },
    quilt: { iconSvg: QuiltIcon, color: "var(--core-quilt)" },
    forge: { iconSvg: ForgeIcon, color: "var(--core-forge)" },
    folia: { iconSvg: FoliaIcon, color: "var(--core-folia)" },
    waterfall: { iconSvg: WaterfallIcon, color: "var(--core-waterfall)" },
    vanilla: { iconSvg: VanillaIcon, color: "var(--core-vanilla)" },
  };

  function stopCardDrag(event: MouseEvent | TouchEvent): void {
    event.stopPropagation();
  }
</script>

<article class="card server-card" class:running={server.running} style={`--core-color:${coreVisuals[server.core].color}`}>
  <div class="server-bg-icon" aria-hidden="true">
    {@html coreVisuals[server.core].iconSvg}
  </div>

  <div class="server-head">
    <div class="server-icon" class:running={server.running}>
      {@html coreVisuals[server.core].iconSvg}
    </div>
    <div class="server-head-right">
      <span class="badge badge-muted badge-dot" class:badge-running={server.running}>
        {server.running ? t("status_running") : t("status_stopped")}
      </span>
      <button
        type="button"
        class="btn-icon btn-folder"
        title={t("open_in_explorer")}
        onmousedown={stopCardDrag}
        ontouchstart={stopCardDrag}
        onclick={() => onOpenFolder(server.id)}
      >
        {@html FolderOpenIcon}
      </button>
      <button
        type="button"
        class="btn-icon btn-edit"
        title={t("server_edit_profile")}
        onmousedown={stopCardDrag}
        ontouchstart={stopCardDrag}
        onclick={() => onEdit(server.id)}
      >
        {@html EditIcon}
      </button>
      <button 
        type="button" 
        class="btn-icon btn-delete" 
        title={t("delete_server_title")}
        disabled={server.running}
        onmousedown={stopCardDrag}
        ontouchstart={stopCardDrag}
        onclick={() => onDelete(server.id)}
      >
        {@html TrashIcon}
      </button>
    </div>
  </div>

  <h3 class="server-name">{server.name}</h3>
  <p class="server-meta">{server.core} · {server.version} · :{server.port}</p>
  
  {#if server.running && server.online_players !== null && server.online_players !== undefined && server.online_players > 0}
    <div class="server-online">
      <span class="online-icon">{@html UsersIcon}</span>
      <span class="online-text">
        {server.online_players}/{server.max_players ?? 20} {t("players_count")}
      </span>
    </div>
  {/if}

  <div class="server-actions">
    {#if server.running}
      <button 
        type="button" 
        class="btn btn-danger btn-sm" 
        disabled={isRestarting}
        onmousedown={stopCardDrag}
        ontouchstart={stopCardDrag}
        onclick={() => onStop(server.id)}
      >
        ■ {t("action_stop")}
      </button>
      <button 
        type="button" 
        class="btn btn-sm"
        class:btn-warning={isRestarting}
        class:btn-secondary={!isRestarting}
        disabled={isRestarting}
        onmousedown={stopCardDrag}
        ontouchstart={stopCardDrag}
        onclick={() => onRestart(server.id)}
      >
        <span class="btn-icon-inline" class:spinning={isRestarting}>{@html RestartIcon}</span>
        <span>{isRestarting ? t("action_restarting") : t("action_restart")}</span>
      </button>
    {:else}
      <button
        type="button"
        class="btn btn-primary btn-sm"
        onmousedown={stopCardDrag}
        ontouchstart={stopCardDrag}
        onclick={() => onStart(server.id)}
      >
        <span class="btn-icon-inline">{@html PlayIcon}</span>
        <span>{t("action_start")}</span>
      </button>
    {/if}
    <button
      type="button"
      class="btn btn-secondary btn-sm"
      onmousedown={stopCardDrag}
      ontouchstart={stopCardDrag}
      onclick={() => onOpenConsole(server.id)}
    >
      <span class="btn-icon-inline">{@html TerminalIcon}</span>
      <span>{t("action_console")}</span>
    </button>
  </div>
</article>

<style>
  .server-card {
    position: relative;
    overflow: hidden;
    isolation: isolate;
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-height: 240px;
    user-select: none;
  }

  .server-card > :not(.server-bg-icon) {
    position: relative;
    z-index: 1;
  }

  .server-card.running {
    border-color: var(--accent);
  }

  .server-bg-icon {
    position: absolute;
    right: -58px;
    bottom: -54px;
    width: 230px;
    height: 230px;
    color: var(--core-color);
    opacity: 0.08;
    transform: rotate(-14deg);
    pointer-events: none;
    z-index: 0;
  }

  .server-bg-icon :global(svg) {
    width: 100%;
    height: 100%;
    display: block;
  }

  .server-bg-icon :global(path),
  .server-bg-icon :global(circle),
  .server-bg-icon :global(rect),
  .server-bg-icon :global(polygon),
  .server-bg-icon :global(polyline),
  .server-bg-icon :global(ellipse),
  .server-bg-icon :global(line) {
    stroke: currentColor !important;
    fill: none !important;
  }

  .server-bg-icon :global(path[fill]:not([fill="none"])),
  .server-bg-icon :global(circle[fill]:not([fill="none"])),
  .server-bg-icon :global(rect[fill]:not([fill="none"])),
  .server-bg-icon :global(polygon[fill]:not([fill="none"])),
  .server-bg-icon :global(polyline[fill]:not([fill="none"])),
  .server-bg-icon :global(ellipse[fill]:not([fill="none"])) {
    fill: currentColor !important;
    stroke: none !important;
  }

  .server-bg-icon :global(svg > path[d="M0 0h24v24H0z"]) {
    stroke: none !important;
    fill: none !important;
  }

  .server-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .server-head-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .btn-folder {
    width: 28px;
    height: 28px;
    color: var(--text-hint);
    background: transparent;
    border: 0.5px solid transparent;
    transition: all var(--tr);
  }

  .btn-folder:hover:not(:disabled) {
    color: var(--accent);
    background: var(--accent-bg);
    border-color: var(--accent);
  }

  .btn-folder :global(svg) {
    width: 14px;
    height: 14px;
  }

  .btn-delete {
    width: 28px;
    height: 28px;
    color: var(--text-hint);
    background: transparent;
    border: 0.5px solid transparent;
    transition: all var(--tr);
  }

  .btn-delete:hover:not(:disabled) {
    color: var(--error-color);
    background: var(--error-bg);
    border-color: var(--error-color);
  }

  .btn-delete:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .btn-delete :global(svg) {
    width: 14px;
    height: 14px;
  }

  .btn-edit {
    width: 28px;
    height: 28px;
    color: var(--text-hint);
    background: transparent;
    border: 0.5px solid transparent;
    transition: all var(--tr);
  }

  .btn-edit:hover:not(:disabled) {
    color: var(--accent);
    background: var(--accent-bg);
    border-color: var(--accent);
  }

  .btn-edit :global(svg) {
    width: 14px;
    height: 14px;
  }

  .server-icon {
    width: 32px;
    height: 32px;
    border-radius: var(--r-md);
    color: var(--core-color);
    background: var(--surface-2);
    display: grid;
    place-items: center;
    font-size: 20px;
    border: 0.5px solid var(--border);
    flex-shrink: 0;
  }

  .server-icon :global(svg) {
    width: 18px;
    height: 18px;
  }

  .server-icon.running {
    background: var(--accent-bg);
    border-color: var(--core-color);
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

  .server-online {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 8px;
    background: var(--accent-bg);
    border: 0.5px solid var(--accent);
    border-radius: var(--r-sm);
    font-size: 11px;
    color: var(--accent);
  }

  .online-icon {
    width: 12px;
    height: 12px;
    display: grid;
    place-items: center;
  }

  .online-icon :global(svg) {
    width: 12px;
    height: 12px;
    stroke-width: 2;
  }

  .online-text {
    font-weight: 500;
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

  .spinning {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .btn:disabled {
    opacity: 0.7;
    cursor: not-allowed;
    pointer-events: none;
  }

  .btn:disabled:hover {
    transform: none;
    opacity: 0.7;
  }
</style>
