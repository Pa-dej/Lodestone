<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import Topbar from "$lib/components/Topbar.svelte";
  import { initServers, serverState, startPollingServers, startServer, stopPollingServers } from "$lib/stores/servers.svelte";
  import { loadSettings, settingsState } from "$lib/stores/settings.svelte";
  import { t } from "$lib/stores/i18n.svelte";
  let { children } = $props();

  let bootError = $state<string | null>(null);
  let initialized = $state(false);

  async function initializeApp(): Promise<void> {
    try {
      await loadSettings();
      await initServers();

      if (settingsState.settings.autostart_servers) {
        for (const server of serverState.servers) {
          if (!server.running) {
            await startServer(server.id);
          }
        }
      }
      startPollingServers();
      initialized = true;
    } catch (error) {
      bootError = error instanceof Error ? error.message : String(error);
    }
  }

  onMount(() => {
    void initializeApp();
    return () => {
      stopPollingServers();
    };
  });
</script>

<div class="app-shell">
  <Sidebar />
  <div class="shell-main">
    <Topbar />
    <main class="shell-content">
      {#if bootError}
        <div class="alert alert-danger">
          <span class="alert-icon">✕</span>
          <div class="alert-text">
            <div class="alert-title">{t("error_title")}</div>
            <div class="alert-sub">{bootError}</div>
          </div>
        </div>
      {:else if !initialized}
        <div class="panel loading-panel">{t("loading")}</div>
      {/if}
      {@render children()}
    </main>
  </div>
</div>

<style>
  .app-shell {
    height: 100vh;
    overflow: hidden;
    display: grid;
    grid-template-columns: 220px 1fr;
    background: var(--bg);
  }

  .shell-main {
    min-width: 0;
    height: 100vh;
    overflow: hidden;
    display: grid;
    grid-template-rows: 44px 1fr;
  }

  .shell-content {
    min-width: 0;
    overflow: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .loading-panel {
    padding: 12px;
  }

  @media (max-width: 920px) {
    .app-shell {
      grid-template-columns: 76px 1fr;
    }
  }

  @media (max-width: 640px) {
    .app-shell {
      grid-template-columns: 64px 1fr;
    }

    .shell-content {
      padding: 10px;
    }
  }
</style>
