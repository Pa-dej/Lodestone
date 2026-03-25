<script lang="ts">
  import { goto } from "$app/navigation";
  import NewServerModal from "$lib/components/NewServerModal.svelte";
  import DeleteConfirmModal from "$lib/components/DeleteConfirmModal.svelte";
  import ServerAddCard from "$lib/components/ServerAddCard.svelte";
  import ServerCard from "$lib/components/ServerCard.svelte";
  import { openConsoleTab, openServerFolder, serverState, startServer, stopServer, restartServer, deleteServer } from "$lib/stores/servers.svelte";
  import { format, t } from "$lib/stores/i18n.svelte";
  import type { ServerConfig } from "$lib/types";

  let isModalOpen = $state(false);
  let deleteModalOpen = $state(false);
  let deletingServer = $state<ServerConfig | null>(null);
  let deleteInProgress = $state(false);

  const deleteMessage = $derived.by(() =>
    deletingServer
      ? format(t("delete_confirm_text"), { name: deletingServer.name })
      : "",
  );

  function openCreateModal(): void {
    isModalOpen = true;
  }

  function closeCreateModal(): void {
    isModalOpen = false;
  }

  function openConsole(serverId: string): void {
    openConsoleTab(serverId);
    void goto(`/console?server=${serverId}`);
  }

  function requestDelete(server: ServerConfig): void {
    deletingServer = server;
    deleteModalOpen = true;
  }

  function closeDeleteModal(): void {
    if (deleteInProgress) {
      return;
    }
    deleteModalOpen = false;
    deletingServer = null;
  }

  async function confirmDelete(): Promise<void> {
    if (!deletingServer) {
      return;
    }
    deleteInProgress = true;
    await deleteServer(deletingServer.id);
    deleteInProgress = false;
    closeDeleteModal();
  }
</script>

<section class="servers-page">
  {#if serverState.error}
    <div class="alert alert-danger">
      <span class="alert-icon">✕</span>
      <div class="alert-text">
        <div class="alert-title">{t("error_title")}</div>
        <div class="alert-sub">{serverState.error}</div>
      </div>
    </div>
  {/if}

  <div class="servers-grid">
    {#each serverState.servers as server (server.id)}
      <ServerCard
        {server}
        onStart={(id) => {
          void startServer(id);
        }}
        onStop={(id) => {
          void stopServer(id);
        }}
        onRestart={(id) => {
          void restartServer(id);
        }}
        onDelete={() => requestDelete(server)}
        onOpenFolder={(id) => {
          void openServerFolder(id);
        }}
        onOpenConsole={openConsole}
      />
    {/each}
    <ServerAddCard onClick={openCreateModal} />
  </div>

  <NewServerModal
    open={isModalOpen}
    onClose={closeCreateModal}
    onCreated={() => {
      closeCreateModal();
    }}
  />

  <DeleteConfirmModal
    open={deleteModalOpen}
    title={t("delete_confirm_title")}
    message={deleteMessage}
    disabled={deleteInProgress}
    onCancel={closeDeleteModal}
    onConfirm={() => {
      void confirmDelete();
    }}
  />
</section>

<style>
  .servers-page {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .servers-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(240px, 100%), 1fr));
    gap: 12px;
  }
</style>
