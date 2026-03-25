<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
  import ConsolePanel from "$lib/components/ConsolePanel.svelte";
  import {
    attachConsole,
    closeConsoleTab,
    openConsoleTab,
    serverState,
  } from "$lib/stores/servers.svelte";

  let handledQueryId = $state<string | null>(null);

  const tabServers = $derived.by(() =>
    serverState.consoleTabs
      .map((id) => serverState.servers.find((server) => server.id === id))
      .filter((server) => server !== undefined),
  );

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
    closeContainer?.removeAttribute("open");
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

<section class="console-page">
  <div class="tabs-bar panel">
    <div class="tab-list">
      {#if tabServers.length === 0}
        <div class="empty-tabs">Нет открытых вкладок.</div>
      {:else}
        {#each tabServers as tab (tab.id)}
          <button
            type="button"
            class="console-tab"
            class:active={serverState.activeConsoleServer === tab.id}
            onclick={() => activateTab(tab.id)}
          >
            <span class="tab-dot" class:running={tab.running}></span>
            <span>{tab.name}</span>
            <span
              class="tab-close"
              role="button"
              tabindex="0"
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
      {/if}
    </div>

    <details class="tab-picker">
      <summary class="btn btn-ghost btn-sm picker-toggle">+ Вкладки</summary>
      <div class="tab-picker-menu panel">
        {#each serverState.servers as server (server.id)}
          <button
            type="button"
            class="picker-item"
            class:opened={serverState.consoleTabs.includes(server.id)}
            onclick={(event) => {
              const details = (event.currentTarget as HTMLElement).closest("details");
              openFromPicker(server.id, details);
            }}
          >
            <span class="tab-dot" class:running={server.running}></span>
            <span class="picker-name">{server.name}</span>
            {#if serverState.consoleTabs.includes(server.id)}
              <span class="tag">opened</span>
            {/if}
          </button>
        {/each}
      </div>
    </details>
  </div>

  <div class="console-body">
    {#if serverState.consoleTabs.length === 0}
      <div class="panel quick-open">
        <h2 class="panel-title">Откройте сервер</h2>
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
  }

  .tabs-bar {
    display: flex;
    gap: 10px;
    padding: 8px;
    justify-content: space-between;
    align-items: flex-start;
  }

  .tab-list {
    display: flex;
    gap: 8px;
    overflow-x: auto;
    min-width: 0;
    flex: 1;
  }

  .empty-tabs {
    color: var(--text-muted);
    font-size: 12px;
  }

  .console-tab {
    border: 0.5px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface);
    color: var(--text-muted);
    padding: 7px 10px;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    transition: border-color var(--tr), color var(--tr), background var(--tr);
    white-space: nowrap;
  }

  .console-tab.active {
    border-color: var(--accent);
    color: var(--text);
    background: var(--surface-2);
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

  .tab-close {
    opacity: 0;
    color: var(--text-hint);
    transition: opacity var(--tr), color var(--tr);
  }

  .console-tab:hover .tab-close {
    opacity: 1;
  }

  .tab-close:hover {
    color: var(--text);
  }

  .console-body {
    min-height: 0;
  }

  .tab-picker {
    position: relative;
    flex-shrink: 0;
  }

  .picker-toggle {
    list-style: none;
    user-select: none;
  }

  .picker-toggle::-webkit-details-marker {
    display: none;
  }

  .tab-picker-menu {
    position: absolute;
    right: 0;
    top: calc(100% + 6px);
    width: min(280px, 70vw);
    display: flex;
    flex-direction: column;
    gap: 6px;
    z-index: 40;
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

  .picker-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
</style>
