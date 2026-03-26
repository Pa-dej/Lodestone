export interface DragState {
  cardId: string;
  startMouseX: number;
  startMouseY: number;
  startCardLeft: number;
  startCardTop: number;
}

export function buildPreviewOrder(
  currentOrder: (string | null)[],
  draggedId: string,
  targetSlot: number
): (string | null)[] {
  const nextOrder = currentOrder.filter((item) => item !== null && item !== draggedId);

  let insertPos = 0;
  for (let i = 0; i < targetSlot; i++) {
    if (currentOrder[i] !== null && currentOrder[i] !== draggedId) {
      insertPos++;
    }
  }

  nextOrder.splice(insertPos, 0, draggedId);

  while (nextOrder.length < currentOrder.length) {
    nextOrder.push(null);
  }

  return nextOrder;
}
