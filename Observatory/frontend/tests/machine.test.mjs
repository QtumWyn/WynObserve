import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mockFrame } from '../src/lib/data/telemetry.ts';
import { machineNodes, machineGpus, focusId, logicalLayout, motherboardLayout, intensity } from '../src/lib/data/machine.ts';
import { gpuReading } from '../src/lib/data/bridge.ts';

function frameWith(gpus = 2, npu = true) {
  const frame = mockFrame(0);
  frame.npu.available = npu;
  frame.gpu.id = gpus ? 'adapter-0' : null;
  frame.gpu.measurementAvailable = gpus > 0;
  frame.gpu.adapters = Array.from({ length: gpus }, (_, i) => ({
    id: `adapter-${i}`, name: i === 0 ? 'Radeon Graphics' : 'Same model name',
    vendor: i === 0 ? 'AMD' : 'NVIDIA', integrated: i === 0,
    dedicatedGiB: 1, sharedLimitGiB: 16, source: 'test fixture',
    telemetry: { usage: i === 0 ? 17 : 81, temperatureC: i === 0 ? 47 : 72,
      temperatureSource: 'test fixture', vramGiB: 0.25, powerWatts: 5, clockMHz: 400, source: 'test fixture' },
  }));
  return frame;
}

test('Machine shows each reported adapter and NPU, with its own identity and readings', () => {
  const frame = frameWith(3);
  const nodes = machineNodes(frame);
  assert.equal(nodes.length, 8);
  const graphics = nodes.filter(n => n.kind === 'GPU');
  assert.deepEqual(graphics.map(n => n.activity), [17, 81, 81]);
  assert.deepEqual(graphics.map(n => n.temperatureC), [47, 72, 72]);
  assert.equal(graphics[0].role, 'INTEGRATED GPU');
  assert.equal(graphics[1].role, 'DISCRETE GPU');
  assert.equal(new Set(graphics.map(n => n.id)).size, 3);
  assert.equal(nodes.find(n => n.kind === 'NPU').name, frame.npu.name);
  assert.ok(!machineNodes(frameWith(0, false)).some(n => n.kind === 'GPU' || n.kind === 'NPU'));
});

test('unknown sensors retain hardware but cannot glow or borrow other measurements', () => {
  const frame = frameWith();
  frame.gpu.adapters[0].telemetry = null;
  frame.npu.usage = NaN;
  frame.npu.residentMiB = NaN;
  for (const node of machineNodes(frame).filter(n => n.id === 'GPU:adapter-0' || n.kind === 'NPU')) {
    assert.ok(Number.isNaN(node.activity));
    assert.ok(Number.isNaN(node.temperatureC));
    assert.equal(intensity(node.activity), 0);
    assert.match(node.detail, /N\/A/);
  }
  assert.equal(machineGpus(frame)[1].activity, 81);
  assert.equal(intensity(0), 0);
  assert.equal(intensity(Infinity), 0);
  assert.equal(intensity(500), 1);
  assert.equal(intensity(-1), 0);
});

test('legacy primary GPU is shown only with an available measured identity', () => {
  const frame = frameWith(0, false);
  frame.gpu.id = 'legacy-primary'; frame.gpu.name = 'Legacy GPU';
  frame.gpu.measurementAvailable = true; frame.gpu.usage = 31;
  assert.equal(machineGpus(frame)[0].activity, 31);
  assert.equal(machineGpus(frame)[0].gpuId, 'legacy-primary');
  frame.gpu.measurementAvailable = false;
  assert.deepEqual(machineGpus(frame), []);
});

test('stable focus survives inventory reordering; a removed adapter never becomes a different GPU', () => {
  const frame = frameWith(3);
  const id = focusId('GPU', 'adapter-0');
  frame.gpu.adapters.reverse();
  assert.equal(machineNodes(frame).find(n => n.id === id).activity, 17);
  assert.equal(gpuReading(frame, 'adapter-0').temperatureC, 47);
  frame.gpu.adapters = frame.gpu.adapters.filter(a => a.id !== 'adapter-0');
  frame.gpu.id = 'adapter-1';
  assert.equal(machineNodes(frame).find(n => n.id === id), undefined);
  assert.equal(gpuReading(frame, 'adapter-0'), null);
  assert.equal(focusId('NPU', 'adapter-0'), 'NPU');
});

function assertLayout(layout, width, inventory) {
  assert.deepEqual(layout.nodes.map(n => n.id).sort(), inventory.map(n => n.id).sort());
  for (const node of layout.nodes) {
    assert.ok(node.x >= 0 && node.y >= 0, `${node.id} begins in the canvas`);
    assert.ok(node.x + node.width <= width + 0.001, `${node.id} fits horizontally`);
    assert.ok(node.y + node.height <= layout.height, `${node.id} fits vertically`);
    assert.ok(node.width > 0 && node.height > 0);
    for (const other of layout.nodes) {
      if (other.id === node.id) continue;
      assert.ok(node.x + node.width <= other.x || other.x + other.width <= node.x ||
        node.y + node.height <= other.y || other.y + other.height <= node.y,
        `${node.id} overlaps ${other.id}`);
    }
  }
}
test('logical diagram reflows without missing or overlapping components at narrow/wide widths', () => {
  for (const count of [0, 1, 2, 7]) for (const npu of [false, true]) for (const width of [280, 519, 520, 900, 1600]) {
    const nodes = machineNodes(frameWith(count, npu));
    assertLayout(logicalLayout(nodes, width), width, nodes);
  }
});
test('motherboard fits all combinations of integrated/discrete GPUs and NPU without overlap', () => {
  for (const count of [0, 1, 2, 7]) for (const npu of [false, true]) for (const integratedCount of [0, 1, count]) {
    const frame = frameWith(count, npu);
    frame.gpu.adapters.forEach((a, i) => { a.integrated = i < integratedCount; });
    const nodes = machineNodes(frame);
    const board = motherboardLayout(nodes);
    assertLayout(board, board.width, nodes);
  }
});

test('outbound-only I/O drives activity while absent rates stay unavailable', () => {
  const frame = frameWith();
  frame.storage.readMiBs = 0; frame.storage.writeMiBs = 30;
  frame.network.rxMiBs = 0; frame.network.txMiBs = 10;
  const nodes = machineNodes(frame);
  assert.equal(nodes.find(n => n.kind === 'NVMe').activity, 20);
  assert.equal(nodes.find(n => n.kind === 'NETWORK').activity, 30);
  frame.network.rxMiBs = NaN; frame.network.txMiBs = NaN;
  assert.ok(Number.isNaN(machineNodes(frame).find(n => n.kind === 'NETWORK').activity));
});
