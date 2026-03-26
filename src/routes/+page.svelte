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
  let deleteModalOpen = $state(false);
  let deletingServer = $state<ServerConfig | null>(null);
  let deleteInProgress = $state(false);

  const deleteMessage = $derived.by(() =>
    deletingServer
      ? format(t("delete_confirm_text"), { name: deletingServer.name })
      : "",
  );

  // Drag & Drop состояние
  let gridEl: HTMLElement | null = $state(null);
  let cardEls: Record<string, HTMLElement> = $state({});
  let dragging: DragState | null = $state(null);
  let lastTargetSlot = $state(-1);
  
  const orderedServers = $derived(getOrderedServers());
  
  // Создаем массив порядка с null для пустых слотов (добавляем место для карточки "добавить")
  const order = $derived.by(() => {
    const result: (string | null)[] = [];
    orderedServers.forEach(s => result.push(s.id));
    result.push(null); // Место для карточки "добавить"
    return result;
  });

  function openCreateModal(): void {
    isModalOpen = true;
  }

  function closeCreateModal(): void {
    isModalOpen = false;
  }

  function requestEdit(server: ServerConfig): void {
    editingServer = server;
    isEditModalOpen = true;
  }

  function closeEditModal(): void {
    isEditModalOpen = false;
    editingServer = null;
  }

  async function saveServerProfile(payload: {
    id: string;
    name: string;
    port: number;
    ram_mb: number;
    jvm_args: string;
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

  // Drag & Drop функции
  function getCardPositionInGrid(index: number): { x: number; y: number; w: number; h: number } {
    if (!gridEl) return { x: 0, y: 0, w: 0, h: 0 };
    
    const gridRect = gridEl.getBoundingClientRect();
    const gridStyle = window.getComputedStyle(gridEl);
    const gap = parseFloat(gridStyle.gap) || 12;
    
    // Получаем количество колонок из grid-template-columns
    const columns = gridStyle.gridTemplateColumns.split(' ').length;
    
    // Вычисляем размеры карточки
    const totalGapWidth = gap * (columns - 1);
    const cardWidth = (gridRect.width - totalGapWidth) / columns;
    const cardHeight = 240; // увеличенная высота для размещения всех кнопок
    
    // Вычисляем позицию
    const row = Math.floor(index / columns);
    const col = index % columns;
    
    const x = col * (cardWidth + gap);
    const y = row * (cardHeight + gap);
    
    return { x, y, w: cardWidth, h: cardHeight };
  }

  function positionCard(cardId: string, slotIdx: number, animate: boolean): void {
    const el = cardEls[cardId];
    if (!el || !gridEl) return;
    
    const p = getCardPositionInGrid(slotIdx);
    el.style.transition = animate 
      ? 'left 0.2s cubic-bezier(.4,0,.2,1), top 0.2s cubic-bezier(.4,0,.2,1)' 
      : 'none';
    el.style.left = p.x + 'px';
    el.style.top = p.y + 'px';
    el.style.width = p.w + 'px';
    el.style.height = p.h + 'px';
  }

  function layoutAll(animate: boolean): void {
    order.forEach((cardId, slot) => {
      if (cardId !== null) {
        positionCard(cardId, slot, animate);
      }
    });
    
    // Позиционируем карточку добавления в последний слот
    const addCardSlot = orderedServers.length;
    if (cardEls['__add__']) {
      positionCard('__add__', addCardSlot, animate);
    }
  }

  function handleMouseDown(cardId: string, e: MouseEvent | TouchEvent): void {
    if (!gridEl || !cardEls[cardId]) return;
    
    if (e.type === 'touchstart') {
      e.preventDefault();
    }
    
    const cx = e.type === 'touchstart' ? (e as TouchEvent).touches[0].clientX : (e as MouseEvent).clientX;
    const cy = e.type === 'touchstart' ? (e as TouchEvent).touches[0].clientY : (e as MouseEvent).clientY;
    
    const el = cardEls[cardId];
    dragging = {
      cardId,
      startMouseX: cx,
      startMouseY: cy,
      startCardLeft: parseFloat(el.style.left),
      startCardTop: parseFloat(el.style.top),
    };
    
    lastTargetSlot = order.indexOf(cardId);
    
    el.style.transition = 'box-shadow 0.15s';
    el.style.zIndex = '100';
    el.style.boxShadow = '0 8px 32px rgba(0,0,0,0.22)';
    
    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
    document.addEventListener('touchmove', handleMouseMove, { passive: false });
    document.addEventListener('touchend', handleMouseUp);
  }

  function findNearestSlotByPosition(mouseX: number, mouseY: number): number {
    if (!dragging || !gridEl) return 0;
    
    const el = cardEls[dragging.cardId];
    if (!el) return 0;
    
    // Получаем текущую позицию и размер перетаскиваемой карточки
    const cardRect = el.getBoundingClientRect();
    const cardCenterX = cardRect.left + cardRect.width / 2;
    const cardCenterY = cardRect.top + cardRect.height / 2;
    
    const gridRect = gridEl.getBoundingClientRect();
    const relX = cardCenterX - gridRect.left;
    const relY = cardCenterY - gridRect.top;
    
    let bestSlot = 0;
    let bestDist = Infinity;
    
    for (let i = 0; i < order.length; i++) {
      const pos = getCardPositionInGrid(i);
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
    if (!dragging || !gridEl) return;
    
    const currentDrag = dragging;
    if (!cardEls[currentDrag.cardId]) return;
    
    if (e.type === 'touchmove') {
      e.preventDefault();
    }
    
    const cx = e.type === 'touchmove' ? (e as TouchEvent).touches[0].clientX : (e as MouseEvent).clientX;
    const cy = e.type === 'touchmove' ? (e as TouchEvent).touches[0].clientY : (e as MouseEvent).clientY;
    
    const dx = cx - currentDrag.startMouseX;
    const dy = cy - currentDrag.startMouseY;
    
    const el = cardEls[currentDrag.cardId];
    el.style.transition = 'box-shadow 0.15s';
    el.style.left = (currentDrag.startCardLeft + dx) + 'px';
    el.style.top = (currentDrag.startCardTop + dy) + 'px';
    
    const target = findNearestSlotByPosition(cx, cy);
    if (target !== lastTargetSlot) {
      lastTargetSlot = target;
      const preview = buildPreviewOrder(order, currentDrag.cardId, target);
      preview.forEach((cid, slot) => {
        if (cid !== null && cid !== currentDrag.cardId) {
          positionCard(cid, slot, true);
        }
      });
    }
  }

  function handleMouseUp(): void {
    if (!dragging) return;
    
    const draggedCardId = dragging.cardId;
    
    document.removeEventListener('mousemove', handleMouseMove);
    document.removeEventListener('mouseup', handleMouseUp);
    document.removeEventListener('touchmove', handleMouseMove);
    document.removeEventListener('touchend', handleMouseUp);
    
    const el = cardEls[draggedCardId];
    if (el) {
      el.style.zIndex = '';
      el.style.boxShadow = '';
    }
    
    const newOrder = buildPreviewOrder(order, draggedCardId, lastTargetSlot);
    const serverIds = newOrder.filter((id): id is string => id !== null);
    updateServerOrder(serverIds);
    
    dragging = null;
    
    // Перепозиционируем все карточки после обновления порядка
    setTimeout(() => layoutAll(true), 0);
  }

  onMount(() => {
    layoutAll(false);
    
    const handleResize = () => layoutAll(false);
    window.addEventListener('resize', handleResize);
    
    return () => {
      window.removeEventListener('resize', handleResize);
    };
  });

  // Перепозиционируем карточки при изменении порядка или размера
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
      <!-- Карточки серверов (абсолютное позиционирование) -->
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
            onEdit={() => requestEdit(server)}
            onOpenConsole={openConsole}
          />
        </div>
      {/each}
      
      <!-- Карточка добавления сервера -->
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

  /* Убираем стандартные стили карточки, так как теперь она в wrapper */
  .card-wrapper :global(.server-card),
  .card-wrapper :global(.add-card) {
    width: 100%;
    height: 100%;
  }
</style>
