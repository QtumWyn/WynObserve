import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mockFrame } from '../src/lib/data/telemetry.ts';
import { machineNodes } from '../src/lib/data/machine.ts';
import { thermalColor, partStyle, boardSignals, boardMetrics, sparkline, packageNodes, fittedSize, clampCamera, zoomCamera } from '../src/lib/data/boardEffects.ts';

function gpuFrame() {
  const frame = mockFrame(0);
  frame.source = 'live'; frame.cpu.vendor = 'AuthenticAMD';
  frame.gpu.adapters = ['igpu','other'].map((id,i) => ({...frame.gpu.adapters[0], id, vendor: i ? 'NVIDIA' : 'AMD', integrated: !i,
    telemetry: {...frame.gpu.adapters[0].telemetry, usage: i ? 81 : 17, temperatureC: i ? 72 : 47} }));
  frame.gpu.id = 'other'; frame.npu.name = 'AMD Ryzen AI';
  return frame;
}
test('thermal palette uses measured Celsius only, clamps its scale, and never colors absent sensors as cool', () => {
  for (const value of [NaN,Infinity,-Infinity]) assert.equal(thermalColor(value),null);
  assert.equal(thermalColor(-10),thermalColor(30));
  assert.equal(thermalColor(110),thermalColor(95));
  assert.notEqual(thermalColor(47),thermalColor(72));
  const node=machineNodes(mockFrame(0)).find(n=>n.kind==='MEMORY');
  assert.match(partStyle(node,'thermal'),/--glow-strength:0%/);
  assert.doesNotMatch(partStyle(node,'activity'),/--glow-strength:0%/);
});
test('read/RX and write/TX animate independently in the correct directions; idle and missing are silent', () => {
  const frame=mockFrame(0), nodes=machineNodes(frame);
  frame.storage={readMiBs:150,writeMiBs:0};
  frame.network={rxMiBs:NaN,txMiBs:10};
  assert.deepEqual(boardSignals(nodes.find(n=>n.kind==='NVMe'),frame),[{direction:'in',strength:1,label:'Read'}]);
  assert.deepEqual(boardSignals(nodes.find(n=>n.kind==='NETWORK'),frame),[{direction:'out',strength:.3,label:'Transmit'}]);
  frame.storage.writeMiBs=30;
  assert.deepEqual(boardSignals(nodes.find(n=>n.kind==='NVMe'),frame).map(s=>s.direction),['in','out']);
  frame.network={rxMiBs:0,txMiBs:NaN};
  assert.deepEqual(boardSignals(nodes.find(n=>n.kind==='NETWORK'),frame),[]);
});
test('popup histories follow adapter IDs through reorder, missing counters and removal', () => {
  const frame=gpuFrame(), node=machineNodes(frame).find(n=>n.gpuId==='igpu');
  const [load,temp]=boardMetrics(node);
  frame.gpu.adapters.reverse();
  assert.equal(load.read(frame),17); assert.equal(temp.read(frame),47);
  frame.gpu.adapters.find(a=>a.id==='igpu').telemetry=null;
  assert.ok(Number.isNaN(load.read(frame))); assert.ok(Number.isNaN(temp.read(frame)));
  frame.gpu.adapters=frame.gpu.adapters.filter(a=>a.id!=='igpu');
  assert.ok(Number.isNaN(load.read(frame)));
});
test('sparklines preserve missing gaps, outages, single points and elapsed time, bounded to 80 samples', () => {
  const metric={label:'Load',unit:'%',ceiling:100,read:f=>f.cpu.usage};
  const point=(time,value)=>({time,value:{...mockFrame(time),cpu:{...mockFrame(time).cpu,usage:value}}});
  const chart=sparkline([point(0,10),point(1,20),point(2,NaN),point(3,30),point(10,40)],metric);
  assert.equal(chart.count,4); assert.equal(chart.seconds,10);
  assert.equal((chart.path.match(/M/g)||[]).length,3);
  assert.equal((chart.path.match(/L/g)||[]).length,1);
  assert.equal(chart.dots.at(-1).x,238);
  assert.equal(sparkline([point(0,20)],metric).dots.length,1);
  assert.equal(sparkline([point(0,NaN)],metric).path,'');
  assert.equal(sparkline(Array.from({length:100},(_,i)=>point(i,50)),metric).count,80);
});
test('package lens groups only reported, compatible integrated GPUs and recognized NPU identities', () => {
  const frame=gpuFrame();
  assert.deepEqual(packageNodes(machineNodes(frame),frame).map(n=>n.id),['CPU','GPU:igpu','NPU']);
  frame.gpu.adapters[0].integrated=null; frame.npu.name='External accelerator';
  assert.deepEqual(packageNodes(machineNodes(frame),frame).map(n=>n.id),['CPU']);
  frame.cpu.vendor='GenuineIntel'; frame.npu.name='Intel(R) AI Boost';
  assert.deepEqual(packageNodes(machineNodes(frame),frame).map(n=>n.id),['CPU','NPU']);
  frame.npu.available=false;
  assert.deepEqual(packageNodes(machineNodes(frame),frame).map(n=>n.id),['CPU']);
});
test('fit and camera bounds work for narrow windows and tall multi-GPU boards', () => {
  for (const view of [{width:240,height:360},{width:1200,height:720}]) for (const aspect of [.3,1,2]) {
    const fitted=fittedSize(view,aspect);
    assert.ok(fitted.width<=view.width-24 && fitted.height<=view.height-24+.001);
    const fit=clampCamera({zoom:1,x:10000,y:-10000},view,fitted);
    assert.equal(Math.abs(fit.x),0); assert.equal(Math.abs(fit.y),0);
    const max=clampCamera({zoom:10,x:10000,y:-10000},view,fitted);
    assert.equal(max.zoom,4); assert.ok(Number.isFinite(max.x) && Number.isFinite(max.y));
    const reset=zoomCamera(max,1,{x:0,y:0},view,fitted);
    assert.equal(Math.abs(reset.x),0); assert.equal(Math.abs(reset.y),0);
  }
});
test('zoom keeps the pointer over the same board point away from pan limits', () => {
  const view={width:600,height:600},fitted=fittedSize(view,1),camera={zoom:2,x:0,y:0},point={x:90,y:-60};
  const next=zoomCamera(camera,3,point,view,fitted);
  assert.equal((point.x-next.x)/next.zoom,(point.x-camera.x)/camera.zoom);
  assert.equal((point.y-next.y)/next.zoom,(point.y-camera.y)/camera.zoom);
});
