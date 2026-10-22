import { test } from 'node:test';
import assert from 'node:assert/strict';
import { boardProfile, boardName, memorySlots, memoryInventoryText } from '../src/lib/data/motherboard.ts';
import { motherboardLayout, machineNodes } from '../src/lib/data/machine.ts';
import { mockFrame } from '../src/lib/data/telemetry.ts';
const board = () => ({ manufacturer: 'Gigabyte Technology Co., Ltd.', model: 'B860M DS3H WIFI6E', version: null,
  systemModel: 'PC', chassisType: 3, memorySlotCount: null, memoryDevices: [] });
test('known board profile requires vendor and exact model, independent of CPU brand', () => {
  assert.equal(boardName(board()), 'Gigabyte B860M DS3H WIFI6E');
  const p = boardProfile(board());
  assert.equal(p.matched, true); assert.equal(p.memoryLabels.length, 4); assert.equal(p.pcieLabels.length, 4);
  assert.equal(p.m2Labels.length, 2); assert.equal(p.sataPorts, 4);
  for (const b of [{ ...board(), model: 'B860M DS3H' }, { ...board(), manufacturer: 'Other' }, null]) {
    assert.equal(boardProfile(b).matched, false);
    assert.equal(boardProfile(b).pcieLabels.length, 0);
  }
});
test('memory socket occupancy matches exact firmware locators, never enumeration order or assumed pairs', () => {
  const b = board(); b.memorySlotCount = 4;
  assert.ok(memorySlots(b).every(s => s.state === 'unknown'));
  b.memoryDevices = [{ locator: 'DIMM_B2', sizeBytes: 16*1024**3, formFactor: 9 }, { locator: 'DIMM_A1', sizeBytes: 0 },
    { locator: 'DIMM_A2', sizeBytes: 16*1024**3 }, { locator: 'DIMM_B1', sizeBytes: null }];
  assert.deepEqual(memorySlots(b).map(s => s.state), ['empty', 'installed', 'unknown', 'installed']);
  b.memoryDevices[0].locator = 'Controller0-ChannelB-DIMM0';
  assert.equal(memorySlots(b)[3].state, 'unknown');
  b.memoryDevices.push({ locator:'DIMM_A2', sizeBytes:0 });
  assert.equal(memorySlots(b)[1].state, 'unknown'); // ambiguous locator
});
test('old or restricted Agents keep unknown counts and unmeasured population', () => {
  assert.deepEqual(memorySlots(null), []);
  assert.match(memoryInventoryText(board()), /not reported/);
  const b = { ...board(), model: 'Unknown board', memorySlotCount: 2, memoryDevices: [{ locator:'Slot 1', sizeBytes:16*1024**3 }] };
  assert.deepEqual(memorySlots(b).map(s => s.state), ['installed','unknown']);
  assert.match(memoryInventoryText(b), /1 installed.*1 unknown/);
});
test('profiles reflow every reported component without changing PCB shape or overlapping hit targets', () => {
  for (const chassisType of [3,9,23,null]) for (const gpus of [0,1,3,7]) {
    const frame = mockFrame(0);
    frame.motherboard = { ...board(), model: chassisType === 3 ? board().model : 'Other', chassisType };
    frame.gpu.adapters = Array.from({length:gpus}, (_,i) => ({id:`gpu-${i}`,name:`GPU ${i}`,integrated:i === 0,telemetry:null}));
    frame.gpu.measurementAvailable = false;
    const nodes = machineNodes(frame), layout = motherboardLayout(nodes, frame.motherboard);
    assert.equal(layout.nodes.length, nodes.length);
    assert.equal(layout.pcbHeight, chassisType === 9 ? 620 : chassisType === 23 ? 1060 : 900);
    for (const n of layout.nodes) {
      assert.ok(n.x >= 0 && n.y >= 0 && n.x+n.width <= layout.width && n.y+n.height <= layout.height);
      for (const other of layout.nodes.filter(o => o.id !== n.id)) {
        assert.ok(n.x+n.width <= other.x || other.x+other.width <= n.x || n.y+n.height <= other.y || other.y+other.height <= n.y, `${n.id} overlaps ${other.id}`);
      }
    }
  }
});
