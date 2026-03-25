<script lang="ts">
  import { goto } from "$app/navigation";
  import NewServerModal from "$lib/components/NewServerModal.svelte";
  import ServerAddCard from "$lib/components/ServerAddCard.svelte";
  import ServerCard from "$lib/components/ServerCard.svelte";
  import { openConsoleTab, serverState, startServer, stopServer, restartServer, deleteServer } from "$lib/stores/servers.svelte";

  let isModalOpen = $state(false);

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
</script>

<section class="servers-page">
  {#if serverState.error}
    <div class="alert alert-danger">
      <span class="alert-icon">✕</span>
      <div class="alert-text">
        <div class="alert-title">Ошибка</div>
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
        onDelete={(id) => {
          if (confirm(`Вы уверены, что хотите удалить сервер "${server.name}"? Это действие нельзя отменить.`)) {
            void deleteServer(id);
          }
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
