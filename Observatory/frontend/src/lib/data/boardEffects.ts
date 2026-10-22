import type { HistoryPoint } from './bridge';
import type { MachineNode } from './machine';
import type { TelemetryFrame } from './telemetry';

export type BoardMode = 'activity' | 'thermal';
const clamp = (value: number, low: number, high: number) => Math.max(low, Math.min(high, value));

/** A fixed Celsius scale, not a device-specific thermal limit or health diagnosis. */
export function thermalColor(celsius: number): string | null {
  if (!Number.isFinite(celsius)) return null;
  return `hsl(${210 - clamp((celsius - 30) / 65, 0, 1) * 210} 88% 65%)`;
}
export function partStyle(node: MachineNode, mode: BoardMode): string {
  const activity = Number.isFinite(node.activity) ? clamp(node.activity / 100, 0, 1) : 0;
  const heat = thermalColor(node.temperatureC);
  const strength = mode === 'thermal' ? (heat ? .25 + clamp((node.temperatureC - 30) / 65, 0, 1) * .75 : 0) : activity;
  const accent = mode === 'thermal' ? heat ?? '#879591' : node.kind === 'CPU' ? 'var(--primary)' : node.kind === 'GPU' ? 'var(--secondary)' : ['NPU','NETWORK'].includes(node.kind) ? 'var(--success)' : 'var(--info)';
  return `--part-accent:${accent};--glow-size:${4 + strength * 20}px;--glow-strength:${strength ? 18 + strength * 38 : 0}%;--pulse-duration:${4 - activity * 3.2}s;--pulse-opacity:${activity * .35};`;
}
export interface BoardSignal { direction: 'in' | 'out'; strength: number; label: string }
/** Every path starts at the CPU; reads/RX flow back toward it, writes/TX away. */
export function boardSignals(node: MachineNode, frame: TelemetryFrame): BoardSignal[] {
  const signal = (direction: 'in' | 'out', rate: number, scale: number, label: string): BoardSignal[] =>
    Number.isFinite(rate) && rate > 0 ? [{ direction, strength: clamp(rate * scale / 100, 0, 1), label }] : [];
  if (node.kind === 'NVMe') return [...signal('in', frame.storage.readMiBs, 1 / 1.5, 'Read'), ...signal('out', frame.storage.writeMiBs, 1 / 1.5, 'Write')];
  if (node.kind === 'NETWORK') return [...signal('in', frame.network.rxMiBs, 3, 'Receive'), ...signal('out', frame.network.txMiBs, 3, 'Transmit')];
  return signal('out', node.activity, 1, node.kind === 'MEMORY' ? 'Memory pressure' : 'Activity');
}

/** Only reported devices enter the lens. Grouping is explicitly illustrative:
 * integrated GPU flags and recognized vendor NPU names are not physical topology. */
export function packageNodes(nodes: MachineNode[], frame: TelemetryFrame): MachineNode[] {
  const cpuVendor = /amd/i.test(frame.cpu.vendor) ? 'amd' : /intel/i.test(frame.cpu.vendor) ? 'intel' : '';
  return nodes.filter(node => {
    if (node.kind === 'CPU') return true;
    if (node.kind === 'GPU' && node.integrated === true) {
      const adapter = frame.gpu.adapters.find(a => a.id === node.gpuId);
      return !!cpuVendor && !!adapter && adapter.vendor.toLowerCase().includes(cpuVendor);
    }
    if (node.kind === 'NPU') {
      if (frame.source === 'demo') return true;
      return cpuVendor === 'intel' ? /^Intel\s*(?:\(R\)\s*)?(?:NPU|AI Boost)\b/i.test(node.name)
        : cpuVendor === 'amd' && /^(?:AMD\s+)?(?:Ryzen AI|XDNA)\b/i.test(node.name);
    }
    return false;
  });
}
export interface BoardMetric {
  label: string; unit: string; ceiling?: number; read: (frame: TelemetryFrame) => number;
}
export function boardMetrics(node: MachineNode): BoardMetric[] {
  const gpu = (frame: TelemetryFrame) => {
    const adapter = frame.gpu.adapters.find(a => a.id === node.gpuId);
    return adapter ? adapter.telemetry : frame.gpu.id === node.gpuId ? frame.gpu : null;
  };
  switch (node.kind) {
    case 'CPU': return [{ label: 'Load', unit: '%', ceiling: 100, read: f => f.cpu.usage }, { label: 'Temperature', unit: '°C', read: f => f.cpu.temperatureC }];
    case 'MEMORY': return [{ label: 'Pressure', unit: '%', ceiling: 100, read: f => f.memory.usedPercent }, { label: 'Used', unit: 'GiB', read: f => f.memory.usedGiB }];
    case 'GPU': return [{ label: 'Load', unit: '%', ceiling: 100, read: f => gpu(f)?.usage ?? NaN }, { label: 'Temperature', unit: '°C', read: f => gpu(f)?.temperatureC ?? NaN }];
    case 'NPU': return [{ label: 'Load', unit: '%', ceiling: 100, read: f => f.npu.available ? f.npu.usage : NaN }, { label: 'Resident', unit: 'MiB', read: f => f.npu.available ? f.npu.residentMiB : NaN }];
    case 'NVMe': return [{ label: 'Read', unit: 'MiB/s', read: f => f.storage.readMiBs }, { label: 'Write', unit: 'MiB/s', read: f => f.storage.writeMiBs }];
    case 'NETWORK': return [{ label: 'Receive', unit: 'MiB/s', read: f => f.network.rxMiBs }, { label: 'Transmit', unit: 'MiB/s', read: f => f.network.txMiBs }];
  }
}
/** Preserve missing measurements and time gaps; don't connect across an outage. */
export function sparkline(history: HistoryPoint[], metric: BoardMetric) {
  const points = history.slice(-80).map(sample => ({ time: sample.time, value: metric.read(sample.value) }));
  const valid = points.filter(p => Number.isFinite(p.value) && Number.isFinite(p.time));
  const min = Math.min(0, ...valid.map(p => p.value));
  const max = metric.ceiling ?? Math.max(1, ...valid.map(p => p.value)) * 1.1;
  const first = points[0]?.time ?? 0, last = points.at(-1)?.time ?? first;
  let path = '', continuing = false, previousTime = first;
  const dots: { x: number; y: number }[] = [];
  for (const point of points) {
    if (!Number.isFinite(point.value) || !Number.isFinite(point.time)) { continuing = false; continue; }
    const x = 2 + (last > first ? (point.time - first) / (last - first) : 1) * 236;
    const y = 46 - clamp((point.value - min) / (max - min), 0, 1) * 42;
    const connected = continuing && point.time - previousTime <= 5 && point.time > previousTime;
    path += `${connected ? 'L' : 'M'}${x.toFixed(2)} ${y.toFixed(2)} `;
    dots.push({ x, y }); continuing = true; previousTime = point.time;
  }
  return { path: path.trim(), dots, count: valid.length, seconds: Math.max(0, last - first), minimum: min, maximum: max };
}

export interface Viewport { width: number; height: number }
export interface BoardCamera { zoom: number; x: number; y: number }
export function fittedSize(view: Viewport, aspect: number): Viewport {
  const width = Math.min(Math.max(1, view.width - 24), Math.max(1, view.height - 24) * aspect);
  return { width, height: width / aspect };
}
export function clampCamera(camera: BoardCamera, view: Viewport, fitted: Viewport): BoardCamera {
  const zoom = clamp(camera.zoom, 1, 4);
  const extentX = Math.max(0, (fitted.width * zoom - view.width) / 2 + 12);
  const extentY = Math.max(0, (fitted.height * zoom - view.height) / 2 + 12);
  return { zoom, x: clamp(camera.x, -extentX, extentX), y: clamp(camera.y, -extentY, extentY) };
}
/** Preserve the point under the pointer, then constrain the board to the viewport. */
export function zoomCamera(camera: BoardCamera, zoom: number, point: { x: number; y: number }, view: Viewport, fitted: Viewport): BoardCamera {
  const next = clamp(zoom, 1, 4), ratio = next / camera.zoom;
  return clampCamera({ zoom: next, x: point.x - (point.x - camera.x) * ratio, y: point.y - (point.y - camera.y) * ratio }, view, fitted);
}
