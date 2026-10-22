import type { Motherboard, MemoryDevice } from './telemetry';

export interface BoardProfile {
  id: string; family: 'desktop' | 'portable' | 'server'; matched: boolean;
  label: string; socket: string | null; memoryLabels: string[]; pcieLabels: string[]; m2Labels: string[];
  sataPorts: number; source: string | null;
}
const gigabyteSource = 'https://www.gigabyte.com/us/Motherboard/B860M-DS3H-WIFI6E-rev-1x/sp';
// Shared characteristics of B860M DS3H WIFI6E revisions 1.x / 2.x. Placement is an
// illustration, not a CAD model or a detected occupancy map. Keep matching exact;
// similarly named EAGLE / DS3H / WIFI boards can have different connectors.
// Artwork reference: https://www.gigabyte.com/FileUpload/Global/KeyFeature/4218/innergigabyte/images/product/diagram.png
export function boardProfile(board?: Motherboard | null): BoardProfile {
  const model = board?.model?.trim().replace(/\s+/g, ' ').toUpperCase();
  if (/gigabyte/i.test(board?.manufacturer ?? '') && model === 'B860M DS3H WIFI6E') return {
    id: 'gigabyte-b860m-ds3h-wifi6e', family: 'desktop', matched: true,
    label: 'Micro-ATX · 244 × 244 mm', socket: 'LGA1851',
    memoryLabels: ['DDR5_A1', 'DDR5_A2', 'DDR5_B1', 'DDR5_B2'],
    pcieLabels: ['PCIEX16 · Gen 5 ×16', 'PCIEX1_1 · Gen 4 ×1', 'PCIEX1_2 · Gen 4 ×1', 'PCIEX1_3 · Gen 4 ×1'],
    m2Labels: ['M2A_CPU · Gen 5 ×4', 'M2N_SB · Gen 4 ×4'], sataPorts: 4, source: gigabyteSource,
  };
  const chassis = board?.chassisType;
  const family = chassis != null && [8,9,10,11,14,30,31,32].includes(chassis) ? 'portable'
    : chassis != null && [17,23,28,29].includes(chassis) ? 'server' : 'desktop';
  return { id: `adaptive-${family}`, family, matched: false,
    label: `${family === 'portable' ? 'Portable' : family === 'server' ? 'Server' : chassis != null && [3,4,5,6,7,13,15,16,24,35,36].includes(chassis) ? 'Desktop' : 'General'} schematic`,
    socket: null, memoryLabels: [], pcieLabels: [], m2Labels: [], sataPorts: 0, source: null };
}
export function boardName(board?: Motherboard | null): string {
  if (!board?.model) return 'Motherboard model unavailable';
  const vendor = board.manufacturer?.replace(/ Technology Co\.,? Ltd\.?/i, '').trim();
  return vendor && !board.model.toLowerCase().startsWith(vendor.toLowerCase()) ? `${vendor} ${board.model}` : board.model;
}
export interface BoardMemorySlot { label: string; state: 'installed' | 'empty' | 'unknown'; device: MemoryDevice | null }
const locatorKey = (text: string | null | undefined) => text?.toUpperCase().replace(/(?:DDR\d|DIMM|CHANNEL|BANK|[\s_\-])/g, '') ?? '';
export function memorySlots(board: Motherboard | null | undefined, profile = boardProfile(board)): BoardMemorySlot[] {
  const devices = board?.memoryDevices ?? [];
  const count = profile.memoryLabels.length || Math.max(board?.memorySlotCount ?? 0, devices.length);
  // Unknown inventory is a neutral memory bank, never four invented installed sticks.
  if (!count) return [];
  return Array.from({ length: Math.min(count, 16) }, (_, i) => {
    const expected = profile.memoryLabels[i];
    // Exact locator match only; firmware order does not specify physical socket order.
    const matches = expected ? devices.filter(d => locatorKey(d.locator) === locatorKey(expected)) : [];
    const device = expected ? (matches.length === 1 ? matches[0] : null) : devices[i] ?? null;
    const label = expected ?? device?.locator ?? `Position ${i + 1}`;
    return { label, device, state: device?.sizeBytes == null ? 'unknown' : device.sizeBytes > 0 ? 'installed' : 'empty' };
  });
}
export function memoryInventoryText(board?: Motherboard | null): string {
  const devices = board?.memoryDevices ?? [];
  if (!devices.length) return 'RAM population not reported';
  const installed = devices.filter(d => d.sizeBytes != null && d.sizeBytes > 0).length;
  const unknown = devices.filter(d => d.sizeBytes == null).length;
  const missing = Math.max(0, (board?.memorySlotCount ?? devices.length) - devices.length);
  return `${installed} installed memory device${installed === 1 ? '' : 's'}${unknown + missing ? ` · ${unknown + missing} unknown` : ''}`;
}
