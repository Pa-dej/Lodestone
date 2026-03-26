// Утилита для drag & drop карточек серверов

export interface DragState {
  cardId: string;
  startMouseX: number;
  startMouseY: number;
  startCardLeft: number;
  startCardTop: number;
}

export interface CellPosition {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface CellCenter {
  x: number;
  y: number;
}

export function getCellCenter(gridEl: HTMLElement, cellEl: HTMLElement): CellCenter {
  const gr = gridEl.getBoundingClientRect();
  const sr = cellEl.getBoundingClientRect();
  return {
    x: sr.left - gr.left + sr.width / 2,
    y: sr.top - gr.top + sr.height / 2,
  };
}

export function getCellPosition(gridEl: HTMLElement, cellEl: HTMLElement): CellPosition {
  const gr = gridEl.getBoundingClientRect();
  const sr = cellEl.getBoundingClientRect();
  return {
    x: sr.left - gr.left + 8,
    y: sr.top - gr.top + 8,
    w: sr.width - 16,
    h: sr.height - 16,
  };
}

export function findNearestSlot(
  gridEl: HTMLElement,
  cells: HTMLElement[],
  mouseX: number,
  mouseY: number
): number {
  const gr = gridEl.getBoundingClientRect();
  const ex = mouseX - gr.left;
  const ey = mouseY - gr.top;
  
  let best = 0;
  let bestDist = Infinity;
  
  cells.forEach((cell, i) => {
    const c = getCellCenter(gridEl, cell);
    const d = Math.hypot(ex - c.x, ey - c.y);
    if (d < bestDist) {
      bestDist = d;
      best = i;
    }
  });
  
  return best;
}

export function buildPreviewOrder(
  currentOrder: (string | null)[],
  draggedId: string,
  targetSlot: number
): (string | null)[] {
  // Убираем перетаскиваемую карточку и null значения
  let tmp = currentOrder.filter(x => x !== null && x !== draggedId);
  
  // Находим позицию для вставки
  let insertPos = 0;
  for (let i = 0; i < targetSlot; i++) {
    if (currentOrder[i] !== null && currentOrder[i] !== draggedId) {
      insertPos++;
    }
  }
  
  // Вставляем карточку
  tmp.splice(insertPos, 0, draggedId);
  
  // Дополняем null значениями до нужной длины
  while (tmp.length < currentOrder.length) {
    tmp.push(null);
  }
  
  return tmp;
}
