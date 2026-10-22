export interface Bounds { left: number; top: number; right: number; bottom: number }
export interface PopupSize { width: number; height: number }

/** Pick a nearby side first, then clamp to the visible panel (not the whole page). */
export function placePopover(anchor: Bounds, size: PopupSize, bounds: Bounds) {
  const gap = 12, margin = 8;
  const minX = bounds.left + margin, minY = bounds.top + margin;
  const maxX = Math.max(minX, bounds.right - size.width - margin);
  const maxY = Math.max(minY, bounds.bottom - size.height - margin);
  const centerX = (anchor.left + anchor.right - size.width) / 2;
  const centerY = (anchor.top + anchor.bottom - size.height) / 2;
  const candidates = [
    { left: anchor.right + gap, top: centerY },
    { left: anchor.left - size.width - gap, top: centerY },
    { left: centerX, top: anchor.bottom + gap },
    { left: centerX, top: anchor.top - size.height - gap },
  ];
  const fits = (p: typeof candidates[number]) => p.left >= minX && p.left <= maxX && p.top >= minY && p.top <= maxY;
  // If neither full side fits, prefer above/below when that can avoid covering the part.
  const p = candidates.find(fits) ?? candidates.slice(2).find(p => p.top >= minY && p.top <= maxY) ?? candidates[0];
  return { left: Math.max(minX, Math.min(maxX, p.left)), top: Math.max(minY, Math.min(maxY, p.top)) };
}
