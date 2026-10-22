<script lang="ts">
  import { formatNumber } from '../data/bridge';
  import type { TelemetryFrame } from '../data/telemetry';
  import { machineNodes, logicalLayout, focusId, intensity, type MachineNode, type MachineComponent } from '../data/machine';
  import './machine-schematic-v2.css';
  interface Props {
    frame: TelemetryFrame; selected: MachineComponent; selectedGpuId: string | null;
    choose: (component: MachineComponent, gpuId?: string) => void; paused?: boolean;
  }
  let { frame, selected, selectedGpuId, choose, paused = false }: Props = $props();
  let displayMode = $state<'activity' | 'thermal' | 'topology'>('activity');
  let width = $state(800);
  let layout = $derived(logicalLayout(machineNodes(frame), width));
  let selectedId = $derived(focusId(selected, selectedGpuId));
  let focused = $derived(layout.nodes.find(n => n.id === selectedId));
  function detail(node: MachineNode): string {
    if (displayMode === 'topology') return `${node.role} · LOGICAL VIEW`;
    if (displayMode === 'thermal') return Number.isFinite(node.temperatureC)
      ? `${formatNumber(node.temperatureC)}°C · ${frame.source === 'demo' ? 'SIMULATED' : 'OBSERVED'} SENSOR` : 'TEMPERATURE UNAVAILABLE';
    return node.detail;
  }
  function route(node: typeof layout.nodes[number]): string {
    return `M${width / 2} 218 V${node.y - 18} H${node.x + node.width / 2} V${node.y}`;
  }
</script>
<div class="schematic machine-v2 machine-dynamic" class:signals-paused={paused} data-mode={displayMode} bind:clientWidth={width} style:height={`${layout.height}px`}>
  <div class="schematic-background" aria-hidden="true"></div>
  <div class="machine-v2-noise" aria-hidden="true"></div>
  <div class="machine-v2-corner machine-v2-corner-left" aria-hidden="true"><span>◈ SYSTEM LATTICE</span><small>WYN / OBSERVATORY</small></div>
  <div class="machine-v2-corner machine-v2-corner-right" aria-hidden="true"><span>{frame.source === 'demo' ? '◇ DEMO TELEMETRY' : '● AGENT TELEMETRY'}</span><small>{layout.nodes.length} · LOGICAL NODES</small></div>
  <svg class="schematic-wires" viewBox={`0 0 ${width} ${layout.height}`} preserveAspectRatio="none" aria-hidden="true">
    {#each layout.nodes.filter(n => n.kind !== 'CPU') as node (node.id)}
      <path d={route(node)} />
      {#if intensity(node.activity) > 0}<path d={route(node)} class="schematic-electric" pathLength="100" style:opacity={.2 + intensity(node.activity) * .6} />{/if}
      <path d={route(node)} class="machine-v2-route" class:focused={selectedId === node.id || selectedId === 'CPU'} />
    {/each}
    <circle cx={width / 2} cy="232" r="4" class="schematic-hub" />
  </svg>
  <div class="machine-v2-modes" role="group" aria-label="Schematic display mode">
    <button class:chosen={displayMode === 'activity'} aria-pressed={displayMode === 'activity'} onclick={() => displayMode = 'activity'}>◉ ACTIVITY</button>
    <button class:chosen={displayMode === 'thermal'} aria-pressed={displayMode === 'thermal'} onclick={() => displayMode = 'thermal'}>♨ THERMAL</button>
    <button class:chosen={displayMode === 'topology'} aria-pressed={displayMode === 'topology'} onclick={() => displayMode = 'topology'}>◇ TOPOLOGY</button>
  </div>
  {#each layout.nodes as node (node.id)}
    <button class="machine-node machine-v2-node" class:active={selectedId === node.id}
      class:thermal-available={displayMode === 'thermal' && Number.isFinite(node.temperatureC)} data-kind={node.kind}
      style={`left:${node.x}px;top:${node.y}px;width:${node.width}px;height:${node.height}px;--machine-v2-glow:${intensity(node.temperatureC - 25) * 30}px`}
      onclick={() => choose(node.kind, node.gpuId)} aria-pressed={selectedId === node.id}
      aria-label={`Inspect ${node.role}: ${node.name}`} title={`${node.name} · ${detail(node)}`}>
      <span class="machine-v2-node-heading"><span class="node-kicker">◇ {node.role}</span><span class="machine-v2-node-status" aria-hidden="true">{selectedId === node.id ? '◈ FOCUS' : '◇ SELECT'}</span></span>
      <strong>{node.label}</strong><span class="node-detail">{detail(node)}</span>
      {#if node.kind === 'CPU' && displayMode !== 'thermal'}
        <span class="core-row machine-v2-core-row" style={`grid-template-columns:repeat(${Math.max(1, frame.cpu.cores.length)},minmax(0,1fr))`} aria-label={`${frame.cpu.cores.length} logical CPU activity indicators`}>
          {#each frame.cpu.cores as core (core.id)}<span class="core-cell" title={`${frame.source === 'demo' ? 'Simulated' : 'Observed'} logical CPU ${core.id}: ${formatNumber(core.usage)}%`} style:opacity={displayMode === 'topology' ? .65 : .25 + intensity(core.usage) * .75}></span>{/each}
        </span>
      {:else if displayMode === 'activity'}
        {#if Number.isFinite(node.activity)}<span class="node-progress"><span style:width={`${intensity(node.activity) * 100}%`}></span></span>{:else}<span class="machine-v2-topology-label">ACTIVITY UNAVAILABLE</span>{/if}
      {:else if displayMode === 'thermal'}
        <span class="machine-v2-sensor-bar" class:unavailable={!Number.isFinite(node.temperatureC)} title="Illustrative display scale, not a thermal safety threshold">
          {#if Number.isFinite(node.temperatureC)}<span class="machine-v2-sensor-fill" style:width={`${intensity(node.temperatureC / 105 * 100) * 100}%`}></span>{:else}<span>◇ NO SENSOR CHANNEL</span>{/if}
        </span>
      {:else}<span class="machine-v2-topology-label">CONCEPTUAL CONNECTION · NOT A BUS TRACE</span>{/if}
    </button>
  {/each}
  <span class="machine-v2-focus"><b>◈ {focused?.name ?? 'Selected hardware unavailable'}</b></span>
  <span class="schematic-key">◇ ILLUSTRATIVE CONNECTIONS / SIGNAL ANIMATION</span>
</div>
