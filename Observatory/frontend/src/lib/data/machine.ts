import type { Component, TelemetryFrame, Motherboard } from './telemetry';
import { boardProfile, memorySlots } from './motherboard.ts';

// Machine focus is separate from the host instruction-sampling component types.
export type MachineComponent = Component | 'NPU';
export interface MachineNode {
  id: string;
  kind: MachineComponent;
  gpuId?: string;
  integrated?: boolean | null;
  name: string;
  label: string;
  role: string;
  detail: string;
  activity: number; // display intensity; storage/network use throughput scales
  temperatureC: number;
}
const number = (value: number, digits = 1) => Number.isFinite(value) ? value.toFixed(digits) : 'N/A';
function throughputActivity(inbound: number, outbound: number, scale: number): number {
  const available = [inbound, outbound].filter(Number.isFinite);
  return available.length ? available.reduce((sum, value) => sum + value, 0) * scale : NaN;
}
export const gpuNodeId = (id: string) => `GPU:${id}`;
export function machineGpus(frame: TelemetryFrame): MachineNode[] {
  const adapters = frame.gpu.adapters;
  if (!adapters.length) {
    // Older Agents can report a measured primary GPU without an inventory.
    return frame.gpu.id && frame.gpu.measurementAvailable ? [{
      id: gpuNodeId(frame.gpu.id), kind: 'GPU', gpuId: frame.gpu.id, name: frame.gpu.name,
      label: `GPU // ${frame.gpu.name}`, role: 'GRAPHICS ADAPTER', activity: frame.gpu.usage,
      temperatureC: frame.gpu.temperatureC,
      detail: `${number(frame.gpu.usage)}% · ${number(frame.gpu.temperatureC)}°C`,
    }] : [];
  }
  return adapters.map(a => {
    const usage = a.telemetry?.usage ?? NaN;
    const temperatureC = a.telemetry?.temperatureC ?? NaN;
    return { id: gpuNodeId(a.id), kind: 'GPU', gpuId: a.id, integrated: a.integrated, name: a.name,
      label: `${a.integrated ? 'iGPU' : 'GPU'} // ${a.name}`,
      role: a.integrated === true ? 'INTEGRATED GPU' : a.integrated === false ? 'DISCRETE GPU' : 'GRAPHICS ADAPTER',
      activity: usage, temperatureC, detail: `${number(usage)}% · ${number(temperatureC)}°C` };
  });
}
export function machineNodes(frame: TelemetryFrame): MachineNode[] {
  return [
    { id: 'CPU', kind: 'CPU', name: frame.cpu.name, label: 'CPU // LOGICAL TOPOLOGY', role: 'PROCESSOR',
      activity: frame.cpu.usage, temperatureC: frame.cpu.temperatureC,
      detail: `${number(frame.cpu.usage)}% · ${frame.cpu.logicalCpuCount} LOGICAL CPUs` },
    { id: 'MEMORY', kind: 'MEMORY', name: 'System memory', label: 'RAM // MEMORY', role: 'SYSTEM MEMORY',
      activity: frame.memory.usedPercent, temperatureC: NaN,
      detail: `${number(frame.memory.usedGiB, 2)} / ${number(frame.memory.totalGiB)} GiB` },
    ...machineGpus(frame),
    ...(frame.npu.available ? [{ id: 'NPU', kind: 'NPU' as const, name: frame.npu.name,
      label: `NPU // ${frame.npu.name}`, role: 'NEURAL ACCELERATOR', activity: frame.npu.usage, temperatureC: NaN,
      detail: `${number(frame.npu.usage)}% · ${number(frame.npu.residentMiB, 2)} MiB resident` }] : []),
    { id: 'NVMe', kind: 'NVMe', name: 'Storage', label: 'STORAGE // I/O', role: 'STORAGE SUBSYSTEM',
      activity: throughputActivity(frame.storage.readMiBs, frame.storage.writeMiBs, 1 / 1.5), temperatureC: NaN,
      detail: `R ${number(frame.storage.readMiBs)} / W ${number(frame.storage.writeMiBs)} MiB/s` },
    { id: 'NETWORK', kind: 'NETWORK', name: 'Network', label: 'NETWORK // I/O', role: 'NETWORK SUBSYSTEM',
      activity: throughputActivity(frame.network.rxMiBs, frame.network.txMiBs, 3), temperatureC: NaN,
      detail: `RX ${number(frame.network.rxMiBs)} / TX ${number(frame.network.txMiBs)} MiB/s` },
  ];
}
export function focusId(kind: MachineComponent, gpuId: string | null): string {
  return kind === 'GPU' ? gpuNodeId(gpuId ?? '') : kind;
}
export const intensity = (value: number) => Number.isFinite(value) ? Math.max(0, Math.min(100, value)) / 100 : 0;

/** Coordinates are illustrative. Reflow to one column in narrow content panes. */
export function logicalLayout(nodes: MachineNode[], width: number) {
  const columns = width < 520 ? 1 : 2;
  const placed = nodes.map((node, i) => {
    const row = i === 0 ? 0 : 1 + Math.floor((i - 1) / columns);
    const center = i === 0 || columns === 1 ? .5 : (i - 1) % 2 === 0 ? .265 : .735;
    const w = width * (i === 0 && columns === 2 ? .56 : columns === 1 ? .88 : .42);
    return { ...node, x: width * center - w / 2, y: 88 + row * 162, width: w, height: 130 };
  });
  return { nodes: placed, height: (placed.at(-1)?.y ?? 88) + 184 };
}

/** Model profiles describe board features, never infer which PCIe/M.2 socket is occupied.
 * Extra GPUs and package accelerators live in a separate detail area, keeping the PCB's shape fixed.
 */
export function motherboardLayout(nodes: MachineNode[], identity?: Motherboard | null) {
  const profile = boardProfile(identity);
  const portable = profile.family === 'portable';
  const graphics = nodes.filter(n => n.kind === 'GPU' && n.integrated !== true);
  const details = nodes.filter(n => n.kind === 'NPU' || n.kind === 'GPU' && (n.integrated === true || graphics.indexOf(n) > 0));
  const pcbHeight = portable ? 620 : profile.family === 'server' ? 1060 : 900;
  const hasDetails = details.length > 0;
  const width = hasDetails ? 1200 : 900;
  const height = Math.max(pcbHeight, hasDetails ? 130 + details.length * 150 : 0);
  const placed = nodes.map(node => {
    let x = 0, y = 0, w = 0, h = 0;
    const detail = details.indexOf(node);
    if (detail >= 0) {
      const card = node.kind === 'GPU' && node.integrated !== true;
      x = card ? 934 : 1000; y = 120 + detail * 150; w = card ? 244 : 100; h = card ? 90 : 100;
    } else if (node.kind === 'CPU') { x = portable ? 285 : 340; y = portable ? 120 : 160; w = 215; h = 204; }
    else if (node.kind === 'MEMORY') { x = 638; y = portable ? 100 : 110; w = 152; h = portable ? 220 : profile.matched ? 440 : 340; }
    else if (node.kind === 'NETWORK') { x = 44; y = portable ? 285 : 270; w = 100; h = 120; }
    else if (node.kind === 'NVMe') { x = portable ? 540 : 242; y = portable ? 400 : 435; w = 300; h = 74; }
    else { x = 180; y = portable ? 378 : 562; w = portable ? 114 : 440; h = portable ? 114 : 162; }
    return { ...node, x, y, width: w, height: h, detached: detail >= 0 };
  });
  return { nodes: placed, width, height, pcbHeight, profile, memorySlots: memorySlots(identity, profile),
    graphicsY: portable ? 378 : 562, acceleratorRows: 0, hasDetails };
}
