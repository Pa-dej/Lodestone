<script lang="ts">
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";
  import NewServerModal from "$lib/components/NewServerModal.svelte";
  import EditServerModal from "$lib/components/EditServerModal.svelte";
  import DeleteConfirmModal from "$lib/components/DeleteConfirmModal.svelte";
  import ServerAddCard from "$lib/components/ServerAddCard.svelte";
  import ServerCard from "$lib/components/ServerCard.svelte";
  import { 
    openConsoleTab, 
    openServerFolder, 
    serverState, 
    startServer, 
    stopServer, 
    restartServer, 
    deleteServer,
    getServerMotd,
    updateServerProfile,
    getOrderedServers,
    updateServerOrder
  } from "$lib/stores/servers.svelte";
  import { format, t } from "$lib/stores/i18n.svelte";
  import type { ServerConfig } from "$lib/types";
  import { buildPreviewOrder, type DragState } from "$lib/utils/dragdrop";

  let isModalOpen = $state(false);
  let isEditModalOpen = $state(false);
  let editingServer = $state<ServerConfig | null>(null);
  let editingMotd = $state("A Lodestone Minecraft Server");
  let deleteModalOpen = $state(false);
  let deletingServer = $state<ServerConfig | null>(null);
  let deleteInProgress = $state(false);

  const deleteMessage = $derived.by(() =>
    deletingServer
      ? format(t("delete_confirm_text"), { name: deletingServer.name })
      : "",
  );

  let gridEl: HTMLElement | null = $state(null);
  let cardEls: Record<string, HTMLElement> = $state({});
  let dragging: DragState | null = $state(null);
  let lastTargetSlot = $state(-1);

  interface GridMetrics {
    columns: number;
    gap: number;
    cardWidth: number;
    cardHeight: number;
  }

  const GRID_GAP_FALLBACK = 12;
  const CARD_HEIGHT = 240;

  const orderedServers = $derived(getOrderedServers());

  const order = $derived.by(() => {
    const orderedIds = orderedServers.map((server) => server.id);
    return [...orderedIds, null];
  });

  function openCreateModal(): void {
    isModalOpen = true;
  }

  function closeCreateModal(): void {
    isModalOpen = false;
  }

  async function requestEdit(server: ServerConfig): Promise<void> {
    editingServer = server;
    isEditModalOpen = true;
    if (["velocity", "waterfall", "bungeecord"].includes(server.core)) {
      editingMotd = await getServerMotd(server.id);
    } else {
      editingMotd = "A Lodestone Minecraft Server";
    }
  }

  function closeEditModal(): void {
    isEditModalOpen = false;
    editingServer = null;
    editingMotd = "A Lodestone Minecraft Server";
  }

  async function saveServerProfile(payload: {
    id: string;
    name: string;
    port: number;
    ram_mb: number;
    jvm_args: string;
    motd?: string;
  }): Promise<void> {
    const updated = await updateServerProfile(payload);
    if (!updated) {
      throw new Error(serverState.error ?? "Failed to update server profile");
    }

    editingServer = updated;
    closeEditModal();
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

  function measureGridMetrics(): GridMetrics | null {
    if (!gridEl) {
      return null;
    }

    const gridRect = gridEl.getBoundingClientRect();
    const gridStyle = window.getComputedStyle(gridEl);
    const gap = Number.parseFloat(gridStyle.gap) || GRID_GAP_FALLBACK;
    const columns = Math.max(
      1,
      gridStyle.gridTemplateColumns
        .split(" ")
        .map((value) => value.trim())
        .filter((value) => value.length > 0).length,
    );

    const totalGapWidth = gap * (columns - 1);
    const cardWidth = (gridRect.width - totalGapWidth) / columns;

    return {
      columns,
      gap,
      cardWidth,
      cardHeight: CARD_HEIGHT,
    };
  }

  function getCardPositionInGrid(index: number, metrics: GridMetrics): { x: number; y: number; w: number; h: number } {
    const row = Math.floor(index / metrics.columns);
    const col = index % metrics.columns;
    return {
      x: col * (metrics.cardWidth + metrics.gap),
      y: row * (metrics.cardHeight + metrics.gap),
      w: metrics.cardWidth,
      h: metrics.cardHeight,
    };
  }

  function positionCard(cardId: string, slotIdx: number, animate: boolean, metrics: GridMetrics): void {
    const el = cardEls[cardId];
    if (!el) {
      return;
    }

    const position = getCardPositionInGrid(slotIdx, metrics);
    el.style.transition = animate
      ? "left 0.2s cubic-bezier(.4,0,.2,1), top 0.2s cubic-bezier(.4,0,.2,1)"
      : "none";
    el.style.left = `${position.x}px`;
    el.style.top = `${position.y}px`;
    el.style.width = `${position.w}px`;
    el.style.height = `${position.h}px`;
  }

  function layoutAll(animate: boolean): void {
    const metrics = measureGridMetrics();
    if (!metrics) {
      return;
    }

    order.forEach((cardId, slot) => {
      if (cardId !== null) {
        positionCard(cardId, slot, animate, metrics);
      }
    });

    const addCardSlot = orderedServers.length;
    if (cardEls["__add__"]) {
      positionCard("__add__", addCardSlot, animate, metrics);
    }
  }

  function handleMouseDown(cardId: string, e: MouseEvent | TouchEvent): void {
    if (!gridEl || !cardEls[cardId]) {
      return;
    }

    if (e.type === "touchstart") {
      e.preventDefault();
    }

    const cx =
      e.type === "touchstart" ? (e as TouchEvent).touches[0].clientX : (e as MouseEvent).clientX;
    const cy =
      e.type === "touchstart" ? (e as TouchEvent).touches[0].clientY : (e as MouseEvent).clientY;

    const el = cardEls[cardId];
    const startCardLeft = Number.parseFloat(el.style.left);
    const startCardTop = Number.parseFloat(el.style.top);
    dragging = {
      cardId,
      startMouseX: cx,
      startMouseY: cy,
      startCardLeft: Number.isFinite(startCardLeft) ? startCardLeft : 0,
      startCardTop: Number.isFinite(startCardTop) ? startCardTop : 0,
    };

    lastTargetSlot = order.indexOf(cardId);

    el.style.transition = "box-shadow 0.15s";
    el.style.zIndex = "100";
    el.style.boxShadow = "0 8px 32px rgba(0,0,0,0.22)";

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
    document.addEventListener("touchmove", handleMouseMove, { passive: false });
    document.addEventListener("touchend", handleMouseUp);
  }

  function findNearestSlotByPosition(): number {
    if (!dragging) {
      return 0;
    }

    const metrics = measureGridMetrics();
    if (!metrics) {
      return 0;
    }

    const el = cardEls[dragging.cardId];
    if (!el) {
      return 0;
    }

    const left = Number.parseFloat(el.style.left);
    const top = Number.parseFloat(el.style.top);
    if (!Number.isFinite(left) || !Number.isFinite(top)) {
      return Math.max(lastTargetSlot, 0);
    }

    const relX = left + metrics.cardWidth / 2;
    const relY = top + metrics.cardHeight / 2;

    let bestSlot = 0;
    let bestDist = Infinity;

    for (let i = 0; i < order.length; i++) {
      const pos = getCardPositionInGrid(i, metrics);
      const centerX = pos.x + pos.w / 2;
      const centerY = pos.y + pos.h / 2;
      const dist = Math.hypot(relX - centerX, relY - centerY);

      if (dist < bestDist) {
        bestDist = dist;
        bestSlot = i;
      }
    }

    return bestSlot;
  }

  function handleMouseMove(e: MouseEvent | TouchEvent): void {
    if (!dragging || !gridEl) {
      return;
    }

    const currentDrag = dragging;
    if (!cardEls[currentDrag.cardId]) {
      return;
    }

    if (e.type === "touchmove") {
      e.preventDefault();
    }

    const cx =
      e.type === "touchmove" ? (e as TouchEvent).touches[0].clientX : (e as MouseEvent).clientX;
    const cy =
      e.type === "touchmove" ? (e as TouchEvent).touches[0].clientY : (e as MouseEvent).clientY;

    const dx = cx - currentDrag.startMouseX;
    const dy = cy - currentDrag.startMouseY;

    const el = cardEls[currentDrag.cardId];
    el.style.transition = "box-shadow 0.15s";
    el.style.left = `${currentDrag.startCardLeft + dx}px`;
    el.style.top = `${currentDrag.startCardTop + dy}px`;

    const target = findNearestSlotByPosition();
    if (target !== lastTargetSlot) {
      lastTargetSlot = target;
      const preview = buildPreviewOrder(order, currentDrag.cardId, target);
      const metrics = measureGridMetrics();
      if (!metrics) {
        return;
      }
      preview.forEach((cid, slot) => {
        if (cid !== null && cid !== currentDrag.cardId) {
          positionCard(cid, slot, true, metrics);
        }
      });
    }
  }

  function handleMouseUp(): void {
    if (!dragging) {
      return;
    }

    const draggedCardId = dragging.cardId;

    document.removeEventListener("mousemove", handleMouseMove);
    document.removeEventListener("mouseup", handleMouseUp);
    document.removeEventListener("touchmove", handleMouseMove);
    document.removeEventListener("touchend", handleMouseUp);

    const el = cardEls[draggedCardId];
    if (el) {
      el.style.zIndex = "";
      el.style.boxShadow = "";
    }

    const newOrder = buildPreviewOrder(order, draggedCardId, lastTargetSlot);
    const serverIds = newOrder.filter((id): id is string => id !== null);
    updateServerOrder(serverIds);

    dragging = null;

    setTimeout(() => layoutAll(true), 0);
  }

  onMount(() => {
    layoutAll(false);

    const handleResize = () => layoutAll(false);
    window.addEventListener("resize", handleResize);

    return () => {
      window.removeEventListener("resize", handleResize);
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
      document.removeEventListener("touchmove", handleMouseMove);
      document.removeEventListener("touchend", handleMouseUp);
    };
  });

  $effect(() => {
    if (gridEl && Object.keys(cardEls).length > 0) {
      layoutAll(false);
    }
  });
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

  <div class="servers-grid-wrapper">
    <div class="servers-grid" bind:this={gridEl}>
      {#each orderedServers as server (server.id)}
        <div 
          class="card-wrapper"
          bind:this={cardEls[server.id]}
          onmousedown={(e) => handleMouseDown(server.id, e)}
          ontouchstart={(e) => handleMouseDown(server.id, e)}
          role="button"
          tabindex="0"
        >
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
            onEdit={() => {
              void requestEdit(server);
            }}
            onOpenConsole={openConsole}
          />
        </div>
      {/each}

      <div 
        class="card-wrapper add-card-wrapper"
        bind:this={cardEls['__add__']}
      >
        <ServerAddCard onClick={openCreateModal} />
      </div>
    </div>
  </div>

  <NewServerModal
    open={isModalOpen}
    onClose={closeCreateModal}
    onCreated={() => {
      closeCreateModal();
    }}
  />

  <EditServerModal
    open={isEditModalOpen}
    server={editingServer}
    motd={editingMotd}
    onClose={closeEditModal}
    onSave={saveServerProfile}
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

  .servers-grid-wrapper {
    position: relative;
    min-height: 240px;
  }

  .servers-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(240px, 100%), 1fr));
    gap: 12px;
    position: relative;
    user-select: none;
  }

  .card-wrapper {
    position: absolute;
    cursor: grab;
    will-change: left, top;
    touch-action: none;
    transition: box-shadow 0.15s ease;
    background: transparent;
    border-radius: var(--r-lg);
    overflow: hidden;
  }

  .card-wrapper:active {
    cursor: grabbing;
  }

  .add-card-wrapper {
    cursor: default;
    pointer-events: auto;
  }

  .add-card-wrapper:active {
    cursor: default;
  }

  .card-wrapper :global(.server-card),
  .card-wrapper :global(.add-card) {
    width: 100%;
    height: 100%;
  }
</style>
