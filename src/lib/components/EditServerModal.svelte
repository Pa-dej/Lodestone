<script lang="ts">
  import { serverState } from "$lib/stores/servers.svelte";
  import type { ServerConfig } from "$lib/types";
  import { t } from "$lib/stores/i18n.svelte";

  interface EditServerPayload {
    id: string;
    name: string;
    port: number;
    ram_mb: number;
    jvm_args: string;
    motd?: string;
  }

  interface Props {
    open: boolean;
    server: ServerConfig | null;
    motd: string;
    onClose: () => void;
    onSave: (payload: EditServerPayload) => Promise<void>;
  }

  let { open, server, motd, onClose, onSave }: Props = $props();

  let name = $state("");
  let port = $state(25565);
  let ramMb = $state(2048);
  let jvmArgs = $state("");
  let proxyMotd = $state("A Lodestone Minecraft Server");
  let saving = $state(false);
  let modalError = $state<string | null>(null);
  let lastServerId = $state<string | null>(null);
  const minRamMb = $derived(Math.max(512, serverState.ramLimits.min_mb || 512));
  const maxRamMb = $derived(Math.max(minRamMb, serverState.ramLimits.max_mb || 16384));
  const isProxyCore = $derived(
    server !== null && ["velocity", "waterfall", "bungeecord"].includes(server.core),
  );

  function resetFromServer(target: ServerConfig): void {
    name = target.name;
    port = target.port;
    ramMb = Math.min(maxRamMb, Math.max(minRamMb, target.ram_mb));
    jvmArgs = target.jvm_args ?? "";
    proxyMotd = motd || "A Lodestone Minecraft Server";
    modalError = null;
  }

  function closeIfAllowed(): void {
    if (saving) {
      return;
    }
    modalError = null;
    onClose();
  }

  async function submitEdit(): Promise<void> {
    if (!server) {
      return;
    }

    const normalizedName = name.trim();
    if (!normalizedName) {
      modalError = t("field_server_name");
      return;
    }

    if (!Number.isInteger(port) || port < 1 || port > 65535) {
      modalError = `${t("field_port")}: 1-65535`;
      return;
    }

    if (!Number.isFinite(ramMb) || ramMb < minRamMb || ramMb > maxRamMb) {
      modalError = `${t("field_ram_mb")}: ${minRamMb}-${maxRamMb}`;
      return;
    }

    saving = true;
    modalError = null;

    try {
      await onSave({
        id: server.id,
        name: normalizedName,
        port,
        ram_mb: Math.round(ramMb),
        jvm_args: jvmArgs.trim(),
        motd: isProxyCore ? proxyMotd.trim() || "A Lodestone Minecraft Server" : undefined,
      });
    } catch (error) {
      modalError = error instanceof Error ? error.message : String(error);
    } finally {
      saving = false;
    }
  }

  $effect(() => {
    if (!open) {
      lastServerId = null;
      return;
    }

    if (server && server.id !== lastServerId) {
      resetFromServer(server);
      lastServerId = server.id;
    }
  });

  $effect(() => {
    if (open && isProxyCore) {
      proxyMotd = motd || "A Lodestone Minecraft Server";
    }
  });
</script>

{#if open && server}
  <div class="modal-overlay" role="presentation">
    <div class="modal panel" role="dialog" aria-modal="true" aria-label={t("server_edit_profile")}>
      <header class="modal-header">
        <h2 class="panel-title">{t("server_edit_profile")}</h2>
        <button type="button" class="btn-icon" onclick={closeIfAllowed}>✕</button>
      </header>

      <div class="server-info">
        <span class="tag">{server.core}</span>
        <span class="tag">v{server.version}</span>
      </div>

      <div class="form-grid">
        <label class="field">
          <span class="field-label">{t("field_server_name")}</span>
          <input class="input" bind:value={name} placeholder="my-server" />
        </label>

        <label class="field">
          <span class="field-label">{t("field_port")}</span>
          <input class="input" type="number" min={1} max={65535} bind:value={port} />
        </label>

        <div class="field">
          <span class="field-label">{t("field_ram_mb")}</span>
          <div class="slider-row">
            <input class="range-input" type="range" min={minRamMb} max={maxRamMb} step={256} bind:value={ramMb} />
            <span class="tag">{ramMb} MB</span>
          </div>
        </div>

        <label class="field">
          <span class="field-label">{t("field_jvm_args")}</span>
          <input class="input" bind:value={jvmArgs} placeholder="-XX:+UseG1GC -XX:+ParallelRefProcEnabled" />
        </label>

        {#if isProxyCore}
          <label class="field">
            <span class="field-label">{t("field_motd")}</span>
            <input class="input" bind:value={proxyMotd} placeholder="A Lodestone Minecraft Server" />
          </label>
        {/if}
      </div>

      {#if modalError}
        <div class="alert alert-danger">
          <span class="alert-icon">✕</span>
          <div class="alert-text">
            <div class="alert-title">{t("error_title")}</div>
            <div class="alert-sub">{modalError}</div>
          </div>
        </div>
      {/if}

      <footer class="modal-footer">
        <button type="button" class="btn btn-ghost" onclick={closeIfAllowed} disabled={saving}>
          {t("action_cancel")}
        </button>
        <button type="button" class="btn btn-primary" onclick={() => void submitEdit()} disabled={saving}>
          {saving ? t("action_saving") : t("action_save")}
        </button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: var(--overlay-bg);
    display: grid;
    place-items: center;
    z-index: 210;
    padding: 16px;
  }

  .modal {
    width: min(560px, 100%);
    max-height: min(86vh, 640px);
    overflow: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .server-info {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .form-grid {
    display: grid;
    gap: 12px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .slider-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .range-input {
    width: 100%;
    accent-color: var(--accent);
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
