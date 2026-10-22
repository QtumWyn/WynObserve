/** Presentation contracts for the first Svelte Observatory milestone.
 *  These are local frontend view models, not yet the wire protocol.
 */
export type Component = 'CPU' | 'MEMORY' | 'GPU' | 'NVMe' | 'NETWORK';

export interface CpuCore {
  id: number;
  usage: number;
  frequencyMHz: number;
}

export interface GpuReading {
  usage: number; temperatureC: number; temperatureSource: string | null;
  vramGiB: number; powerWatts: number; clockMHz: number; source: string;
}
export interface GpuAdapter {
  id: string;
  name: string;
  vendor: string;
  integrated: boolean | null;
  telemetry: GpuReading | null;
  dedicatedGiB: number;
  sharedLimitGiB: number;
  source: string;
}

export interface Instruction {
  sequence: number;
  cpu: number;
  process: string;
  mnemonic: string;
  operands: string;
  truth: 'SAMPLED' | 'OBSERVED' | 'INFERRED' | 'SCHEMATIC';
  origin: 'MOCK' | 'LIVE';
  pid: number;
  tid: number;
  address: string;
  bytes: string;
  component: Component;
  note: string;
}

export interface MemoryDevice {
  locator: string | null; bank: string | null; sizeBytes: number | null;
  formFactor: number | null; memoryType: number | null;
}
export interface Motherboard {
  manufacturer: string | null; model: string | null; version: string | null;
  systemManufacturer: string | null; systemModel: string | null; chassisType: number | null;
  source: string; memorySlotCount: number | null; memoryDevices: MemoryDevice[];
}

export interface TelemetryFrame {
  source: 'demo' | 'live';
  capturedAt: number;
  motherboard?: Motherboard | null;
  cpu: {
    name: string;
    vendor: string;
    physicalCoreCount: number | null;
    logicalCpuCount: number;
    usage: number;
    temperatureC: number;
    temperatureSource: string | null;
    contextSwitches: number;
    cores: CpuCore[];
  };
  memory: { usedGiB: number; totalGiB: number; availableGiB: number; usedPercent: number; faults: number };
  gpu: GpuReading & { id: string | null; name: string; measurementAvailable: boolean; adapters: GpuAdapter[] };
  npu: { available: boolean; name: string; pciAddress: string | null; usage: number;
    frequencyMHz: number; maxFrequencyMHz: number; residentMiB: number; powerState: string | null };
  storage: { readMiBs: number; writeMiBs: number };
  network: { rxMiBs: number; txMiBs: number };
  processes: { name: string; pid: number; cpu: number; memoryGiB: number }[];
  instructions: Instruction[];
}

const wave = (t: number, offset: number, speed: number, min: number, max: number): number =>
  min + (max - min) * (Math.sin(t * speed + offset) * 0.5 + 0.5);

/** Ported in spirit from Observatory/src/mock.rs: deterministic synthetic data.
 *  It must NEVER be represented as actual collected telemetry.
 */
export function mockFrame(t: number): TelemetryFrame {
  const cores: CpuCore[] = Array.from({ length: 16 }, (_, id: number) => {
    const usage = wave(t, id * 0.71, 0.9 + (id % 4) * 0.12, 2, 96);
    return { id, usage, frequencyMHz: Math.trunc(3100 + usage * 17) };
  });
  const usage = cores.reduce((sum: number, core: CpuCore) => sum + core.usage, 0) / cores.length;
  const usedGiB = 32 * wave(t, 0.5, 0.18, 0.39, 0.72);
  const gpuUsage = wave(t, 2, 0.47, 4, 86);
  const instructions: Omit<Instruction, 'sequence' | 'cpu' | 'address' | 'truth' | 'origin'>[] = [
    { process: 'rustc', pid: 18472, tid: 18491, bytes: '48 8B 43 08', mnemonic: 'mov', operands: 'rax, [rbx+0x8]', component: 'CPU', note: 'Simulated CPU instruction' },
    { process: 'rustc', pid: 18472, tid: 18491, bytes: '48 85 C0', mnemonic: 'test', operands: 'rax, rax', component: 'CPU', note: 'Simulated CPU instruction' },
    { process: 'cargo', pid: 18430, tid: 18430, bytes: '75 E9', mnemonic: 'jne', operands: '0x7F23A90E', component: 'CPU', note: 'Simulated branch sample' },
    { process: 'firefox', pid: 7291, tid: 7332, bytes: '48 8B 04 24', mnemonic: 'mov', operands: 'rax, [rsp]', component: 'MEMORY', note: 'Simulated host-side memory operation' },
    { process: 'wyn-observatory', pid: 18801, tid: 18808, bytes: '48 83 EC 20', mnemonic: 'sub', operands: 'rsp, 0x20', component: 'MEMORY', note: 'Simulated stack operation' },
    { process: 'kwin_wayland', pid: 2203, tid: 2244, bytes: 'FF 15 62 31 00 00', mnemonic: 'call', operands: '[rip+0x3162]', component: 'GPU', note: 'Simulated x86 host-side GPU submission, NOT GPU assembly' },
    { process: 'cargo', pid: 18430, tid: 18430, bytes: '0F 05', mnemonic: 'syscall', operands: '', component: 'NVMe', note: 'Simulated host-side disk syscall' },
    { process: 'firefox', pid: 7291, tid: 7351, bytes: '0F 05', mnemonic: 'syscall', operands: '', component: 'NETWORK', note: 'Simulated host-side network syscall' },
  ];
  const sequence = 50_000 + Math.floor(t * 3.5);

  return {
    source: 'demo', capturedAt: t * 1000,
    motherboard: { manufacturer: 'WynObserve', model: 'Demo motherboard', version: null,
      systemManufacturer: null, systemModel: 'Synthetic hardware inventory', chassisType: 3, source: 'Synthetic',
      memorySlotCount: 4, memoryDevices: ['A1','A2','B1','B2'].map(slot => ({ locator: `DIMM_${slot}`, bank: null,
        sizeBytes: slot.endsWith('2') ? 16*1024**3 : 0, formFactor: 9, memoryType: 34 })) },
    cpu: {
      name: 'Demo CPU // logical topology', usage,
      vendor: 'Synthetic', physicalCoreCount: null, logicalCpuCount: cores.length,
      temperatureC: 42 + usage * 0.24,
      temperatureSource: 'Synthetic package sensor',
      contextSwitches: Math.round(18_000 + usage * 310), cores,
    },
    memory: {
      usedGiB, totalGiB: 32, availableGiB: 32 - usedGiB,
      usedPercent: usedGiB / 32 * 100,
      faults: Math.round(240 + wave(t, 0, 0.7, 0, 900)),
    },
    gpu: { id: 'demo-gpu', name: 'NVIDIA RTX // demo', measurementAvailable: true, source: 'Demo', temperatureSource: 'Synthetic GPU sensor', powerWatts: 50 + gpuUsage, clockMHz: 1900,
      adapters: [{ id: 'demo-gpu', name: 'NVIDIA RTX // demo', vendor: 'NVIDIA', integrated: false, dedicatedGiB: 12, sharedLimitGiB: NaN, source: 'Demo', telemetry: { usage: gpuUsage, temperatureC: 38 + gpuUsage * 0.42, temperatureSource: 'Synthetic GPU sensor', vramGiB: 12 * (0.24 + gpuUsage / 180), powerWatts: 50 + gpuUsage, clockMHz: 1900, source: 'Demo' } }], usage: gpuUsage,
      temperatureC: 38 + gpuUsage * 0.42,
      vramGiB: 12 * (0.24 + gpuUsage / 180),
    },
    npu: { available: true, name: 'NPU // demo', pciAddress: null,
      usage: wave(t, 1, 0.6, 0, 72), frequencyMHz: 800 + wave(t, 1, 0.6, 0, 72) * 5,
      maxFrequencyMHz: 1400, residentMiB: wave(t, 2, 0.15, 64, 256), powerState: 'Synthetic active' },
    storage: { readMiBs: wave(t, 0.2, 0.42, 1, 140), writeMiBs: wave(t, 1.9, 0.31, 0, 75) },
    network: { rxMiBs: wave(t, 0.4, 0.64, 1, 28), txMiBs: wave(t, 1.5, 0.52, 0.1, 17) },
    processes: [
      { name: 'rustc', pid: 18472, cpu: wave(t, 0, 0.83, 18, 145), memoryGiB: 2.01 },
      { name: 'firefox', pid: 7291, cpu: wave(t, 1, 0.34, 2, 36), memoryGiB: 3.58 },
      { name: 'kwin_wayland', pid: 2203, cpu: wave(t, 4, 0.73, 1, 18), memoryGiB: 0.57 },
      { name: 'wyn-observatory', pid: 18801, cpu: wave(t, 3.1, 0.55, 1, 9), memoryGiB: 0.18 },
    ],
    instructions: Array.from({ length: 14 }, (_, index: number) => {
      const item = instructions[(sequence - index + instructions.length * 100) % instructions.length];
      return {
        ...item,
        sequence: sequence - index,
        cpu: (sequence + index * 3) % 16,
        address: `0x${(0x7f23a900 + (sequence - index) * 4).toString(16).toUpperCase()}`,
        truth: 'SAMPLED' as const,
        origin: 'MOCK' as const,
      };
    }),
  };
}
