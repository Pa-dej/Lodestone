<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
  import type { CoreType } from "$lib/types";
  import ConsolePanel from "$lib/components/ConsolePanel.svelte";
  import PaperIcon from "../../icons/servers/Paper.svg?raw";
  import PurpurIcon from "../../icons/servers/Purpur.svg?raw";
  import FabricIcon from "../../icons/servers/Fabric.svg?raw";
  import QuiltIcon from "../../icons/servers/Quilt.svg?raw";
  import ForgeIcon from "../../icons/servers/Forge.svg?raw";
  import FoliaIcon from "../../icons/servers/Folia.svg?raw";
  import VelocityIcon from "../../icons/servers/Velocity.svg?raw";
  import WaterfallIcon from "../../icons/servers/Waterfall.svg?raw";
  import BungeeCordIcon from "../../icons/servers/BungeeCord.svg?raw";
  import VanillaIcon from "../../icons/Server.svg?raw";
  import {
    attachConsole,
    closeConsoleTab,
    openConsoleTab,
    serverState,
  } from "$lib/stores/servers.svelte";
  import { t } from "$lib/stores/i18n.svelte";

  let handledQueryId = $state<string | null>(null);
  let pickerQuery = $state("");
  let tabPickerEl: HTMLDetailsElement | null = null;

  interface CoreVisual {
    iconSvg: string;
    color: string;
  }

  const coreVisuals: Record<CoreType, CoreVisual> = {
    paper: { iconSvg: PaperIcon, color: "var(--core-paper)" },
    purpur: { iconSvg: PurpurIcon, color: "var(--core-purpur)" },
    fabric: { iconSvg: FabricIcon, color: "var(--core-fabric)" },
    quilt: { iconSvg: QuiltIcon, color: "var(--core-quilt)" },
    forge: { iconSvg: ForgeIcon, color: "var(--core-forge)" },
    folia: { iconSvg: FoliaIcon, color: "var(--core-folia)" },
    velocity: { iconSvg: VelocityIcon, color: "var(--core-velocity)" },
    waterfall: { iconSvg: WaterfallIcon, color: "var(--core-waterfall)" },
    bungeecord: { iconSvg: BungeeCordIcon, color: "var(--core-bungeecord)" },
    vanilla: { iconSvg: VanillaIcon, color: "var(--core-vanilla)" },
  };

  const tabServers = $derived.by(() =>
    serverState.consoleTabs
      .map((id) => serverState.servers.find((server) => server.id === id))
      .filter((server) => server !== undefined),
  );

  const pickerServers = $derived.by(() => {
    const query = pickerQuery.trim().toLowerCase();
    return [...serverState.servers]
      .filter((server) => {
        if (!query) {
          return true;
        }
        return (
          server.name.toLowerCase().includes(query) ||
          server.core.toLowerCase().includes(query) ||
          server.version.toLowerCase().includes(query)
        );
      })
      .sort((a, b) => {
        const openedDiff = Number(serverState.consoleTabs.includes(b.id)) - Number(serverState.consoleTabs.includes(a.id));
        if (openedDiff !== 0) {
          return openedDiff;
        }

        const runningDiff = Number(b.running) - Number(a.running);
        if (runningDiff !== 0) {
          return runningDiff;
        }

        return a.name.localeCompare(b.name);
      });
  });

  function activateTab(id: string): void {
    openConsoleTab(id);
  }

  function removeTab(id: string): void {
    closeConsoleTab(id);
  }

  function openFromPicker(id: string, closeContainer: HTMLElement | null): void {
    openConsoleTab(id);
    void attachConsole(id);
    void goto(`/console?server=${id}`);
    pickerQuery = "";
    closeContainer?.removeAttribute("open");
  }

  function handlePickerToggle(event: Event): void {
    const details = event.currentTarget as HTMLDetailsElement;
    if (!details.open) {
      pickerQuery = "";
    }
  }

  function handleWindowPointerDown(event: PointerEvent): void {
    if (!tabPickerEl?.open) {
      return;
    }

    const target = event.target;
    if (target instanceof Node && !tabPickerEl.contains(target)) {
      tabPickerEl.open = false;
      pickerQuery = "";
    }
  }

  $effect(() => {
    const fromQuery = $page.url.searchParams.get("server");
    if (fromQuery && fromQuery !== handledQueryId) {
      openConsoleTab(fromQuery);
      void attachConsole(fromQuery);
      handledQueryId = fromQuery;
    }

    if (!serverState.activeConsoleServer && serverState.consoleTabs.length > 0) {
      serverState.activeConsoleServer = serverState.consoleTabs[0];
    }
  });
</script>

<svelte:window onpointerdown={handleWindowPointerDown} />

<section class="console-page">
  <div class="tabs-bar panel">
    <div class="tabs-track">
      <div class="tab-list">
        {#if tabServers.length === 0}
          <div class="empty-tabs">{t("console_no_tabs")}</div>
        {/if}

        {#each tabServers as tab (tab.id)}
          <button
            type="button"
            class="console-tab"
            class:active={serverState.activeConsoleServer === tab.id}
            style={`--core-color:${coreVisuals[tab.core].color}`}
            onclick={() => activateTab(tab.id)}
            onmousedown={(event) => {
              if (event.button === 1) {
                event.preventDefault();
              }
            }}
            onauxclick={(event) => {
              if (event.button === 1) {
                event.preventDefault();
                removeTab(tab.id);
              }
            }}
            title={tab.name}
          >
            <span class="tab-core" aria-hidden="true">
              {@html coreVisuals[tab.core].iconSvg}
            </span>
            <span class="tab-dot" class:running={tab.running}></span>
            <span class="tab-name">{tab.name}</span>
            <span
              class="tab-close"
              role="button"
              tabindex="0"
              aria-label={`Close ${tab.name}`}
              onclick={(event) => {
                event.stopPropagation();
                removeTab(tab.id);
              }}
              onkeydown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  removeTab(tab.id);
                }
              }}
            >
              ✕
            </span>
          </button>
        {/each}
      </div>

      <details class="tab-picker" bind:this={tabPickerEl} ontoggle={handlePickerToggle}>
        <summary class="picker-toggle" title={t("console_tabs_button")} aria-label={t("console_tabs_button")}>
          +
        </summary>
        <div class="tab-picker-menu panel">
          <div class="picker-search-row">
            <input
              type="text"
              class="input picker-search"
              bind:value={pickerQuery}
              placeholder={t("console_tab_search_placeholder")}
            />
          </div>

          <div class="picker-list">
            {#if pickerServers.length === 0}
              <div class="picker-empty">{t("console_tab_search_empty")}</div>
            {:else}
              {#each pickerServers as server (server.id)}
                <button
                  type="button"
                  class="picker-item"
                  class:opened={serverState.consoleTabs.includes(server.id)}
                  style={`--core-color:${coreVisuals[server.core].color}`}
                  onclick={(event) => {
                    const details = (event.currentTarget as HTMLElement).closest("details");
                    openFromPicker(server.id, details);
                  }}
                >
                  <span class="picker-core" aria-hidden="true">
                    {@html coreVisuals[server.core].iconSvg}
                  </span>

                  <span class="tab-dot" class:running={server.running}></span>

                  <span class="picker-main">
                    <span class="picker-name">{server.name}</span>
                    <span class="picker-meta">{server.core} · v{server.version}</span>
                  </span>

                  {#if serverState.consoleTabs.includes(server.id)}
                    <span class="tag">{t("console_opened")}</span>
                  {/if}
                </button>
              {/each}
            {/if}
          </div>
        </div>
      </details>
    </div>
  </div>

  <div class="console-body">
    {#if serverState.consoleTabs.length === 0}
      <div class="panel quick-open">
        <h2 class="panel-title">{t("console_open_server")}</h2>
        <div class="quick-list">
          {#each serverState.servers as server (server.id)}
            <button
              type="button"
              class="btn btn-ghost quick-item"
              onclick={() => {
                openConsoleTab(server.id);
                void attachConsole(server.id);
                void goto(`/console?server=${server.id}`);
              }}
            >
              <span class="tab-dot" class:running={server.running}></span>
              <span>{server.name}</span>
            </button>
          {/each}
        </div>
      </div>
    {:else}
      <ConsolePanel serverId={serverState.activeConsoleServer} />
    {/if}
  </div>
</section>

<style>
  .console-page {
    display: grid;
    grid-template-rows: auto 1fr;
    gap: 12px;
    min-height: calc(100vh - 92px);
    min-width: 0;
    width: 100%;
  }

  .tabs-bar {
    padding: 8px;
    overflow: visible;
    min-width: 0;
    width: 100%;
  }

  .tabs-track {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    min-width: 0;
  }

  .tab-list {
    display: flex;
    align-items: center;
    flex-wrap: nowrap;
    gap: 8px;
    flex: 1 1 auto;
    overflow-x: auto;
    overflow-y: hidden;
    padding-bottom: 2px;
    width: 100%;
    min-width: 0;
    max-width: 100%;
    overscroll-behavior-x: contain;
  }

  .empty-tabs {
    color: var(--text-muted);
    font-size: 13px;
    padding: 0 12px;
    min-height: 36px;
    border: 0.5px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface);
    display: inline-flex;
    align-items: center;
    white-space: nowrap;
  }

  .console-tab {
    border: 0.5px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface);
    color: var(--text-muted);
    padding: 0 12px;
    height: 36px;
    display: inline-flex;
    align-items: center;
    flex: 0 0 auto;
    gap: 8px;
    cursor: pointer;
    transition: border-color var(--tr), color var(--tr), background var(--tr);
    min-width: clamp(118px, 20vw, 220px);
    max-width: min(42vw, 260px);
    box-shadow: none;
  }

  .console-tab:hover {
    color: var(--text);
    border-color: var(--text-hint);
    background: var(--surface-2);
  }

  .console-tab.active {
    border-color: var(--accent);
    color: var(--accent);
    background: color-mix(in srgb, var(--accent-bg) 70%, var(--surface));
  }

  .tab-dot {
    width: 7px;
    height: 7px;
    border-radius: 999px;
    background: var(--text-hint);
  }

  .tab-dot.running {
    background: var(--success-color);
  }

  .tab-core {
    width: 15px;
    height: 15px;
    display: grid;
    place-items: center;
    color: var(--core-color);
    flex-shrink: 0;
  }

  .tab-core :global(svg) {
    width: 15px;
    height: 15px;
    display: block;
  }

  .tab-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    text-align: left;
  }

  .tab-close {
    opacity: 0;
    color: var(--text-hint);
    transition: opacity var(--tr), color var(--tr);
    flex-shrink: 0;
  }

  .console-tab:hover .tab-close,
  .console-tab.active .tab-close {
    opacity: 1;
  }

  .tab-close:hover {
    color: var(--text);
  }

  .console-body {
    min-height: 0;
    min-width: 0;
  }

  .tab-picker {
    position: relative;
    flex: 0 0 auto;
  }

  .picker-toggle {
    list-style: none;
    user-select: none;
    width: 36px;
    height: 36px;
    display: grid;
    place-items: center;
    border: 0.5px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface);
    color: var(--text-muted);
    cursor: pointer;
    font-size: 20px;
    line-height: 1;
    transition: border-color var(--tr), color var(--tr), background var(--tr);
  }

  .picker-toggle:hover {
    color: var(--text);
    border-color: var(--accent);
    background: var(--surface-2);
  }

  .picker-toggle::-webkit-details-marker {
    display: none;
  }

  .tab-picker[open] .picker-toggle {
    border-color: var(--accent);
    color: var(--accent);
    background: color-mix(in srgb, var(--accent-bg) 70%, var(--surface));
  }

  .tab-picker-menu {
    position: absolute;
    right: 0;
    top: calc(100% + 6px);
    width: clamp(290px, 48vw, 420px);
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 40;
    padding: 8px;
  }

  .picker-search-row {
    position: sticky;
    top: 0;
    z-index: 2;
    background: var(--surface);
    border-radius: var(--r-md);
  }

  .picker-search {
    min-height: 34px;
    padding: 7px 10px;
    font-size: 12px;
  }

  .picker-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-height: min(56vh, 420px);
    overflow-y: auto;
    padding-right: 2px;
  }

  .picker-empty {
    min-height: 36px;
    border: 0.5px solid var(--border);
    border-radius: var(--r-md);
    color: var(--text-muted);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
  }

  .picker-item {
    border: 0.5px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface);
    color: var(--text);
    padding: 8px 10px;
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    text-align: left;
  }

  .picker-item:hover {
    border-color: var(--accent);
    background: var(--surface-2);
  }

  .picker-item.opened {
    color: var(--text-muted);
  }

  .picker-core {
    width: 15px;
    height: 15px;
    display: grid;
    place-items: center;
    color: var(--core-color);
    flex-shrink: 0;
  }

  .picker-core :global(svg) {
    width: 15px;
    height: 15px;
    display: block;
  }

  .picker-main {
    min-width: 0;
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
  }

  .picker-name {
    width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .picker-meta {
    width: 100%;
    font-size: 11px;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-transform: lowercase;
  }

  .quick-open {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .quick-list {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .quick-item {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  @media (max-width: 860px) {
    .console-tab {
      min-width: clamp(104px, 38vw, 184px);
      max-width: min(58vw, 198px);
      height: 34px;
      padding-inline: 10px;
      gap: 6px;
    }

    .tab-core {
      width: 13px;
      height: 13px;
    }

    .tab-core :global(svg) {
      width: 13px;
      height: 13px;
    }

    .tab-close {
      opacity: 1;
    }

    .picker-toggle {
      width: 34px;
      height: 34px;
      font-size: 17px;
    }

    .tab-picker-menu {
      width: min(88vw, 360px);
      right: -4px;
    }
  }

  @media (max-width: 640px) {
    .tabs-bar {
      padding: 6px;
    }

    .tab-list {
      gap: 6px;
    }

    .console-tab {
      min-width: 96px;
      max-width: 62vw;
      height: 32px;
      padding-inline: 8px;
      gap: 5px;
    }

    .tab-close {
      opacity: 1;
    }
  }
</style>
