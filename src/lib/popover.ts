/** Place a floating layer next to an anchor, flipping above when there is no room below. */
export function placeBelow(
  anchor: DOMRect,
  size: { width: number; height: number },
  align: 'start' | 'end' = 'start',
  gap = 6,
): { left: number; top: number; up: boolean } {
  const vw = window.innerWidth;
  const vh = window.innerHeight;
  const roomBelow = vh - anchor.bottom - gap - 8;
  const up = roomBelow < size.height && anchor.top > roomBelow;
  let left = align === 'start' ? anchor.left : anchor.right - size.width;
  left = Math.max(8, Math.min(vw - size.width - 8, left));
  let top = up ? anchor.top - size.height - gap : anchor.bottom + gap;
  top = Math.max(8, Math.min(vh - size.height - 8, top));
  return { left: Math.round(left), top: Math.round(top), up };
}
