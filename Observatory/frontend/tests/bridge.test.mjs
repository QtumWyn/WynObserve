import { test } from 'node:test';
import assert from 'node:assert/strict';
import { frameFromWire, chooseMachine, appendFrame, fleetDevices, formatNumber } from '../src/lib/data/bridge.ts';

const GiB = 1024 ** 3;
function snapshot() {
  return { schema_version: 1, captured_at_unix_ms: 123456,
    cpu: { brand: 'Test CPU', global_usage_percent: 37, logical_cpu_count: 2, logical_cpus: [{ logical_id: 4, usage_percent: 22, frequency_mhz: 3400 }] },
    memory: { total_bytes: 32*GiB, used_bytes: 8*GiB, available_bytes: 23*GiB, page_faults_per_second: 16, fault_rates_available: true },
    scheduler: { context_switches_per_second: 120, context_switch_rate_available: false },
    gpu: { available: false }, storage: { available: true, rates_available: false }, network: { available: false },
    processes_available: true, processes: [{ name:'process', pid:1, cpu_usage_percent:0, cpu_rate_available:false, memory_bytes:GiB, memory_available:true }] };
}
function machine(id, status = 'online', sample = snapshot()) {
  return { machine: { machine_id:id, machine_name:id, role:'workstation', os:'linux', agent_version:'test' }, status, last_seen_ms:100, latest_sequence:1, snapshot:sample };
}
test('live CPU and memory use wire values, including reported available memory', () => {
  const frame = frameFromWire(snapshot());
  assert.equal(frame.source, 'live'); assert.equal(frame.cpu.usage,37);
  assert.equal(frame.cpu.cores[0].id,4); assert.equal(frame.cpu.cores[0].frequencyMHz,3400);
  assert.equal(frame.memory.usedGiB,8); assert.equal(frame.memory.usedPercent,25);
  assert.equal(frame.memory.availableGiB,23); // not total minus used
  assert.deepEqual(frame.instructions,[]);
});
test('unavailable counters are gaps, never invented zeros', () => {
  const wire = snapshot(); wire.memory.total_bytes=0; wire.memory.fault_rates_available=false; wire.cpu.logical_cpus[0].frequency_mhz=0;
  const frame=frameFromWire(wire);
  for (const value of [frame.cpu.temperatureC,frame.cpu.contextSwitches,frame.cpu.cores[0].frequencyMHz,frame.memory.usedPercent,frame.memory.faults,frame.gpu.usage,frame.storage.readMiBs,frame.network.rxMiBs,frame.processes[0].cpu]) {
    assert.ok(Number.isNaN(value)); assert.equal(formatNumber(value),'N/A');
  }
  assert.equal(frame.processes[0].memoryGiB,1);
});
test('CPU temperature survives the wire adapter and failures clear both value and source', () => {
  const wire = snapshot();
  wire.cpu.temperature = { celsius: 45.125, source: 'coretemp · Package id 0' };
  let frame = frameFromWire(wire);
  assert.equal(frame.cpu.temperatureC, 45.125);
  assert.equal(frame.cpu.temperatureSource, 'coretemp · Package id 0');
  wire.cpu.temperature.celsius = 0;
  assert.equal(frameFromWire(wire).cpu.temperatureC, 0);
  for (const value of [null, undefined, { celsius: null }, { celsius: Infinity }, { celsius: '45' }]) {
    wire.cpu.temperature = value;
    frame = frameFromWire(wire);
    assert.ok(Number.isNaN(frame.cpu.temperatureC));
    assert.equal(frame.cpu.temperatureSource, null);
  }
});
test('integrated GPU identity survives without NVML and never inherits discrete GPU counters', () => {
  const wire = snapshot();
  wire.gpu.adapters = [
    { id: 'dxcore:amd', model: 'AMD Radeon Graphics', vendor: 'AMD', integrated: true, dedicated_memory_bytes: 512 * 1024 ** 2, shared_memory_limit_bytes: 16 * GiB, source: 'DXCore' },
    { id: 'dxcore:nvidia', model: 'NVIDIA GPU', vendor: 'NVIDIA', integrated: false, dedicated_memory_bytes: 12 * GiB, shared_memory_limit_bytes: null, source: 'DXCore' },
  ];
  let frame = frameFromWire(wire);
  assert.equal(frame.gpu.adapters[0].integrated, true);
  assert.equal(frame.gpu.adapters[0].dedicatedGiB, 0.5);
  assert.equal(frame.gpu.adapters[0].sharedLimitGiB, 16);
  assert.ok(Number.isNaN(frame.gpu.adapters[1].sharedLimitGiB));
  assert.equal(frame.gpu.measurementAvailable, false);
  assert.ok(Number.isNaN(frame.gpu.usage));
  wire.gpu = { ...wire.gpu, available: true, model: 'NVIDIA GPU', utilization_percent: 73, temperature_c: 61, vram_used_bytes: 2 * GiB };
  frame = frameFromWire(wire);
  assert.equal(frame.gpu.name, 'NVIDIA GPU');
  assert.equal(frame.gpu.usage, 73);
  assert.equal(frame.gpu.adapters[0].name, 'AMD Radeon Graphics');
  assert.ok(!('usage' in frame.gpu.adapters[0]));
  delete wire.gpu.adapters;
  assert.deepEqual(frameFromWire(wire).gpu.adapters, []);
});
test('shared selection chooses a usable initial target and never silently switches an existing target', () => {
  const machines=[machine('offline','offline'), machine('empty','online',null),machine('ready')];
  assert.equal(chooseMachine(null,machines),'ready');
  assert.equal(chooseMachine('offline',machines),'offline');
  assert.equal(chooseMachine('missing',machines),'missing');
  assert.equal(chooseMachine(null,[]),null);
});
test('fleet includes offline and snapshot-less machines without demo sites', () => {
  const rows=fleetDevices({ stale:true, machines:[machine('a'),machine('b','offline'),machine('c','online',null)] });
  assert.deepEqual(rows.map(r=>r.status),['warning','offline','warning']);
  assert.ok(rows.every(r=>r.siteId==='unassigned')); assert.ok(Number.isNaN(rows[2].cpu));
});
test('history deduplicates snapshots and bounds retention', () => {
  let history=[];
  for (let i=0;i<100;i++) { const wire=snapshot(); wire.captured_at_unix_ms=i*1000; history=appendFrame(history,frameFromWire(wire)); }
  assert.equal(history.length,80); assert.equal(history[0].time,20);
  assert.equal(appendFrame(history,history.at(-1).value),history);
});

test('NPU capabilities distinguish idle, missing, older agents and driver failure', () => {
  const wire = snapshot();
  assert.equal(frameFromWire(wire).npu.available, false);
  wire.npu = { available: true, model: 'Intel AI Boost', pci_address: '0000:00:0b.0',
    utilization_available: true, frequency_available: true, memory_available: true,
    utilization_percent: 0, current_frequency_mhz: 400, max_frequency_mhz: 1400,
    memory_used_bytes: 64 * 1024 ** 2, power_state: 'D0' };
  let npu = frameFromWire(wire).npu;
  assert.equal(npu.usage, 0); assert.equal(npu.residentMiB, 64); assert.equal(npu.frequencyMHz, 400);
  assert.equal(npu.maxFrequencyMHz, 1400); assert.equal(npu.pciAddress, '0000:00:0b.0'); assert.equal(npu.powerState, 'D0');
  wire.npu.utilization_available = false; delete wire.npu.memory_available; wire.npu.current_frequency_mhz = null;
  npu = frameFromWire(wire).npu;
  assert.ok([npu.usage, npu.residentMiB, npu.frequencyMHz].every(Number.isNaN));
  wire.npu.available = false;
  npu = frameFromWire(wire).npu;
  assert.ok([npu.usage, npu.residentMiB, npu.frequencyMHz, npu.maxFrequencyMHz].every(Number.isNaN));
  assert.equal(npu.powerState, null); assert.equal(npu.pciAddress, null);
});

test('GPU selection and history keep Radeon and discrete measurements separate', async () => {
  const { gpuReading } = await import('../src/lib/data/bridge.ts');
  const wire = snapshot();
  wire.gpu.primary_adapter_id = 'discrete'; wire.gpu.available = true;
  const identity = { vendor: 'AMD', integrated: true, dedicated_memory_bytes: null, shared_memory_limit_bytes: 16 * GiB, source: 'DXCore' };
  wire.gpu.adapters = [
    { ...identity, id: 'radeon', model: 'Radeon Graphics', telemetry: { source: 'AMD ADL', utilization_percent: 0, temperature_c: 46, temperature_source: 'GPU edge', memory_used_bytes: null, power_watts: 5, graphics_clock_mhz: 400 } },
    { ...identity, id: 'discrete', model: 'NVIDIA GPU', vendor: 'NVIDIA', integrated: false, telemetry: { source: 'NVML', utilization_percent: 84, temperature_c: 72, memory_used_bytes: 4 * GiB, power_watts: 150, graphics_clock_mhz: 2200 } },
  ];
  const first = frameFromWire(wire);
  assert.equal(first.gpu.name, 'NVIDIA GPU'); assert.equal(first.gpu.usage, 84);
  assert.equal(gpuReading(first, 'radeon').usage, 0); assert.equal(gpuReading(first, 'radeon').temperatureC, 46);
  assert.ok(Number.isNaN(gpuReading(first, 'radeon').vramGiB)); // shared limit is NOT usage
  wire.gpu.adapters[0].telemetry = { source: 'AMD ADL', utilization_percent: null, temperature_c: null };
  const failed = frameFromWire(wire);
  assert.ok(Number.isNaN(gpuReading(failed, 'radeon').usage));
  assert.ok(Number.isNaN(gpuReading(failed, 'radeon').temperatureC));
  wire.gpu.adapters.shift();
  assert.equal(gpuReading(frameFromWire(wire), 'radeon'), null);
  assert.deepEqual([first, failed].map(f => gpuReading(f, 'discrete').usage), [84,84]);
});

test('motherboard identity and empty/unknown memory inventory survive the bridge and clear on target changes', () => {
  const wire = snapshot();
  assert.equal(frameFromWire(wire).motherboard, null);
  wire.motherboard = { manufacturer:'Gigabyte', model:'B860M DS3H WIFI6E', version:null, source:'Windows SMBIOS',
    chassis_type:3, memory_slot_count:4, memory_devices:[{locator:'DIMM_A1', size_bytes:0}, {locator:'DIMM_A2',size_bytes:16*GiB}, {locator:'DIMM_B1',size_bytes:null}] };
  const b = frameFromWire(wire).motherboard;
  assert.equal(b.model,'B860M DS3H WIFI6E'); assert.equal(b.memorySlotCount,4);
  assert.deepEqual(b.memoryDevices.map(d=>d.sizeBytes),[0,16*GiB,null]);
  wire.motherboard.memory_devices.push({ size_bytes:-1 });
  assert.equal(frameFromWire(wire).motherboard.memoryDevices.at(-1).sizeBytes,null);
  wire.motherboard = null;
  assert.equal(frameFromWire(wire).motherboard,null);
});
