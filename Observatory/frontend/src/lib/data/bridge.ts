import type { TelemetryFrame, GpuReading } from './telemetry';
import type { FleetDevice } from './fleet';

// Mirrors the existing wyn-protocol feed. The desktop owns the configured endpoint.
export interface WireSnapshot {
  schema_version: number; captured_at_unix_ms: number;
  motherboard?: { manufacturer?: string | null; model?: string | null; version?: string | null;
    system_manufacturer?: string | null; system_model?: string | null; chassis_type?: number | null; source?: string;
    memory_slot_count?: number | null; memory_devices?: { locator?: string | null; bank?: string | null;
      size_bytes?: number | null; form_factor?: number | null; memory_type?: number | null }[] } | null;
  cpu: { brand: string; vendor: string; physical_core_count?: number | null; temperature?: { celsius: number; source: string } | null; global_usage_percent: number; logical_cpu_count: number; logical_cpus: { logical_id: number; usage_percent: number; frequency_mhz: number }[] };
  memory: { total_bytes: number; used_bytes: number; available_bytes: number; page_faults_per_second: number; fault_rates_available: boolean };
  scheduler: { context_switches_per_second: number; context_switch_rate_available: boolean };
  npu?: { available: boolean; utilization_available?: boolean; frequency_available?: boolean; memory_available?: boolean;
    model: string; pci_address: string; utilization_percent: number; current_frequency_mhz: number;
    max_frequency_mhz: number; memory_used_bytes: number; power_state: string };
  gpu: { primary_adapter_id?: string | null; available: boolean; model: string; utilization_percent: number; temperature_c: number; vram_used_bytes: number;
    adapters?: { id: string; model: string; vendor: string; integrated: boolean | null; dedicated_memory_bytes: number | null; shared_memory_limit_bytes: number | null; source: string;
    telemetry?: { source: string; utilization_percent: number | null; temperature_c: number | null; temperature_source: string | null;
      memory_used_bytes: number | null; power_watts: number | null; graphics_clock_mhz: number | null } | null }[] };
  storage: { available: boolean; rates_available: boolean; read_mib_s: number; write_mib_s: number };
  network: { available: boolean; rates_available: boolean; rx_mib_s: number; tx_mib_s: number };
  processes_available: boolean;
  processes: { name: string; pid: number; cpu_usage_percent: number; cpu_rate_available: boolean; memory_bytes: number; memory_available: boolean }[];
}
export interface WireMachine {
  machine: { machine_id: string; machine_name: string; role: string; os: string; agent_version: string };
  status: 'online' | 'stale' | 'offline'; last_seen_ms: number; latest_sequence: number | null;
  snapshot: WireSnapshot | null;
}
export interface FleetFeed {
  endpoint: string; connection: 'connecting' | 'connected' | 'disconnected';
  error: string | null; received_age_ms: number | null; stale: boolean; machines: WireMachine[];
}

declare global {
  interface Window { __TAURI__?: { core: { invoke: <T>(command: string) => Promise<T> } } }
}
export function isDesktop(): boolean { return typeof window !== 'undefined' && !!window.__TAURI__?.core; }
export async function readFleet(): Promise<FleetFeed> {
  if (!window.__TAURI__?.core) throw new Error('Live telemetry requires the Tauri desktop app.');
  return window.__TAURI__.core.invoke<FleetFeed>('fleet_snapshot');
}

// NaN is an internal presentation-only missing-value sentinel; never sent over IPC.
// Charts leave a gap and formatNumber renders N/A rather than a fabricated zero.
export function formatNumber(value: number | undefined | null, digits = 1): string {
  return typeof value === 'number' && Number.isFinite(value) ? value.toLocaleString(undefined, { minimumFractionDigits: digits, maximumFractionDigits: digits }) : 'N/A';
}
const GiB = 1024 ** 3;
const measured = (value: unknown): number => typeof value === 'number' && Number.isFinite(value) && value >= 0 ? value : NaN;
const missingGpu: GpuReading = { usage: NaN, temperatureC: NaN, temperatureSource: null, vramGiB: NaN, powerWatts: NaN, clockMHz: NaN, source: 'Unavailable' };
export function gpuReading(frame: TelemetryFrame, id: string | null): (GpuReading & { name: string }) | null {
  const adapter = frame.gpu.adapters.find(a => a.id === id);
  if (adapter) return { name: adapter.name, ...(adapter.telemetry ?? missingGpu) };
  return id === frame.gpu.id ? frame.gpu : null;
}
export function frameFromWire(s: WireSnapshot): TelemetryFrame {
  const cpuTemperature = s.cpu.temperature;
  const hasCpuTemperature = typeof cpuTemperature?.celsius === 'number' && Number.isFinite(cpuTemperature.celsius);
  const adapters = (s.gpu.adapters ?? []).map(a => ({ id: a.id, name: a.model, vendor: a.vendor, integrated: a.integrated,
    dedicatedGiB: measured(a.dedicated_memory_bytes) / GiB, sharedLimitGiB: measured(a.shared_memory_limit_bytes) / GiB, source: a.source,
    telemetry: a.telemetry ? { source: a.telemetry.source, usage: measured(a.telemetry.utilization_percent),
      temperatureC: measured(a.telemetry.temperature_c), temperatureSource: a.telemetry.temperature_source,
      vramGiB: measured(a.telemetry.memory_used_bytes) / GiB, powerWatts: measured(a.telemetry.power_watts),
      clockMHz: measured(a.telemetry.graphics_clock_mhz) } : null }));
  const primary = adapters.find(a => a.id === s.gpu.primary_adapter_id)
    ?? (!s.gpu.available ? adapters.find(a => a.telemetry && [a.telemetry.usage, a.telemetry.temperatureC].some(Number.isFinite)) : undefined);
  const gpu = primary ? { id: primary.id, name: primary.name, ...(primary.telemetry ?? missingGpu) }
    : { id: s.gpu.available ? 'legacy-primary' : null, name: s.gpu.available ? s.gpu.model : 'GPU measurements unavailable', ...missingGpu,
      usage: s.gpu.available ? measured(s.gpu.utilization_percent) : NaN,
      temperatureC: s.gpu.available ? measured(s.gpu.temperature_c) : NaN,
      vramGiB: s.gpu.available ? measured(s.gpu.vram_used_bytes) / GiB : NaN, source: s.gpu.available ? 'Agent primary GPU' : 'Unavailable' };
  const npu = s.npu;
  const b = s.motherboard;
  const text = (v: unknown) => typeof v === 'string' && v.trim() ? v.trim().slice(0, 160) : null;
  const integer = (v: unknown) => typeof v === 'number' && Number.isSafeInteger(v) && v >= 0 ? v : null;
  return {
    source: 'live', capturedAt: s.captured_at_unix_ms,
    motherboard: b ? { manufacturer: text(b.manufacturer), model: text(b.model), version: text(b.version),
      systemManufacturer: text(b.system_manufacturer), systemModel: text(b.system_model), chassisType: integer(b.chassis_type),
      source: text(b.source) ?? 'Agent firmware', memorySlotCount: integer(b.memory_slot_count),
      memoryDevices: (b.memory_devices ?? []).slice(0, 256).map(d => ({ locator: text(d.locator), bank: text(d.bank),
        sizeBytes: integer(d.size_bytes), formFactor: integer(d.form_factor), memoryType: integer(d.memory_type) })) } : null,
    cpu: { name: s.cpu.brand, usage: s.cpu.global_usage_percent,
      temperatureC: hasCpuTemperature ? cpuTemperature!.celsius : NaN,
      temperatureSource: hasCpuTemperature ? cpuTemperature!.source : null,
      vendor: s.cpu.vendor, physicalCoreCount: s.cpu.physical_core_count ?? null, logicalCpuCount: s.cpu.logical_cpu_count,
      contextSwitches: s.scheduler.context_switch_rate_available ? s.scheduler.context_switches_per_second : NaN,
      cores: s.cpu.logical_cpus.map(c => ({ id: c.logical_id, usage: c.usage_percent, frequencyMHz: c.frequency_mhz > 0 ? c.frequency_mhz : NaN })) },
    memory: { totalGiB: s.memory.total_bytes / GiB, usedGiB: s.memory.used_bytes / GiB,
      availableGiB: s.memory.available_bytes / GiB,
      usedPercent: s.memory.total_bytes > 0 ? s.memory.used_bytes / s.memory.total_bytes * 100 : NaN,
      faults: s.memory.fault_rates_available ? s.memory.page_faults_per_second : NaN },
    gpu: { ...gpu, adapters, measurementAvailable: [gpu.usage, gpu.temperatureC, gpu.vramGiB].some(Number.isFinite) },
    npu: { available: npu?.available === true, name: npu?.available ? npu.model : 'NPU unavailable',
      pciAddress: npu?.available && npu.pci_address ? npu.pci_address : null,
      usage: npu?.available && npu.utilization_available ? measured(npu.utilization_percent) : NaN,
      frequencyMHz: npu?.available && npu.frequency_available ? measured(npu.current_frequency_mhz) : NaN,
      maxFrequencyMHz: npu?.available && npu.frequency_available ? measured(npu.max_frequency_mhz) : NaN,
      residentMiB: npu?.available && npu.memory_available ? measured(npu.memory_used_bytes) / 1024 ** 2 : NaN,
      powerState: npu?.available && npu.power_state && npu.power_state !== 'unknown' ? npu.power_state : null },
    storage: { readMiBs: s.storage.available && s.storage.rates_available ? s.storage.read_mib_s : NaN,
      writeMiBs: s.storage.available && s.storage.rates_available ? s.storage.write_mib_s : NaN },
    network: { rxMiBs: s.network.available && s.network.rates_available ? s.network.rx_mib_s : NaN,
      txMiBs: s.network.available && s.network.rates_available ? s.network.tx_mib_s : NaN },
    processes: s.processes_available ? s.processes.map(p => ({ name: p.name, pid: p.pid,
      cpu: p.cpu_rate_available ? p.cpu_usage_percent : NaN, memoryGiB: p.memory_available ? p.memory_bytes / GiB : NaN })) : [],
    instructions: [], // Live instruction sampling is a separate request workflow.
  };
}
export function fleetDevices(feed: FleetFeed): FleetDevice[] {
  return feed.machines.map(m => ({ id: m.machine.machine_id, name: m.machine.machine_name, siteId: 'unassigned',
    role: m.machine.role, os: m.machine.os, status: m.status === 'offline' ? 'offline' : feed.stale || m.status === 'stale' || m.last_seen_ms > 5000 ? 'warning' : 'online',
    agentVersion: m.machine.agent_version, lastSeen: `${Math.floor(m.last_seen_ms / 1000)}s ago`,
    cpu: m.snapshot?.cpu.global_usage_percent ?? NaN,
    memory: m.snapshot && m.snapshot.memory.total_bytes > 0 ? m.snapshot.memory.used_bytes / m.snapshot.memory.total_bytes * 100 : NaN }));
}
export function chooseMachine(current: string | null, machines: WireMachine[]): string | null {
  // Retain an explicit selection even if the target disappears: never silently retarget.
  return current ?? machines.find(m => m.status === 'online' && m.snapshot)?.machine.machine_id ?? machines[0]?.machine.machine_id ?? null;
}
export type HistoryPoint = { time: number; value: TelemetryFrame };
export function appendFrame(history: HistoryPoint[], frame: TelemetryFrame): HistoryPoint[] {
  if (history.at(-1)?.value.capturedAt === frame.capturedAt) return history;
  return [...history.slice(-79), { time: frame.capturedAt / 1000, value: frame }];
}
