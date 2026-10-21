<script lang="ts">
  import type { Component, TelemetryFrame } from '../data/telemetry';
  import './machine-schematic-v2.css';

  interface Props {
    frame: TelemetryFrame;
    selected: Component;
    choose: (component: Component) => void;
  }

  type DisplayMode = 'activity' | 'thermal' | 'topology';
  interface Link {
    id: string;
    from: Component;
    to: Component;
    d: string;
  }

  let { frame, selected, choose }: Props = $props();
  let displayMode = $state<DisplayMode>('activity');

  // The five nodes and their selection callback are the original contract.
  // These connections are conceptual, not measured motherboard/bus topology.
  const components: Component[] = ['CPU', 'MEMORY', 'GPU', 'NVMe', 'NETWORK'];
  const links: Link[] = [
    { id: 'cpu-memory', from: 'CPU', to: 'MEMORY', d: 'M400 146 V180 H164 V215' },
    { id: 'cpu-gpu', from: 'CPU', to: 'GPU', d: 'M400 180 H636 V215' },
    { id: 'memory-storage', from: 'MEMORY', to: 'NVMe', d: 'M164 307 V347 H292 V386' },
    { id: 'gpu-network', from: 'GPU', to: 'NETWORK', d: 'M636 307 V347 H508 V386' },
  ];
  const label: Record<Component, string> = {
    CPU: 'CPU // LOGICAL TOPOLOGY',
    MEMORY: 'RAM // MEMORY',
    GPU: 'GPU // COMPUTE',
    NVMe: 'NVMe // STORAGE',
    NETWORK: 'NETWORK // I/O',
  };
  const topologyDetail: Record<Component, string> = {
    CPU: 'LOGICAL PROCESSORS · NOT PHYSICAL CORES',
    MEMORY: 'SHARED SYSTEM MEMORY · LOGICAL VIEW',
    GPU: 'COMPUTE DEVICE · LOGICAL VIEW',
    NVMe: 'STORAGE PATH · LOGICAL VIEW',
    NETWORK: 'NETWORK PATH · LOGICAL VIEW',
  };

  function clampPercent(value: number): number {
    return Number.isFinite(value) ? Math.max(0, Math.min(100, value)) : 0;
  }

  // For NVMe and network the existing bar is an illustrative throughput scale,
  // NOT a hardware utilization percentage. Preserve its original conversion.
  function progress(component: Component): number {
    switch (component) {
      case 'CPU': return clampPercent(frame.cpu.usage);
      case 'MEMORY': return clampPercent(frame.memory.usedPercent);
      case 'GPU': return clampPercent(frame.gpu.usage);
      case 'NVMe': return clampPercent(frame.storage.readMiBs / 1.5);
      case 'NETWORK': return clampPercent(frame.network.rxMiBs * 3);
    }
  }

  function temperature(component: Component): number | null {
    if (component === 'CPU') return frame.cpu.temperatureC;
    if (component === 'GPU') return frame.gpu.temperatureC;
    return null; // The mock contract provides no sensor for these nodes.
  }

  function heatOpacity(component: Component): number {
    const measurement = temperature(component);
    if (measurement === null || !Number.isFinite(measurement)) return 0;
    // A decorative intensity only; NOT a thermal alarm or safety threshold.
    return Math.max(0.07, Math.min(0.65, (measurement - 25) / 100));
  }

  function detail(component: Component): string {
    if (displayMode === 'topology') return topologyDetail[component];
    if (displayMode === 'thermal') {
      const measurement = temperature(component);
      return measurement === null ? 'NO TEMPERATURE SENSOR IN DEMO MODEL' :
        `${measurement.toFixed(1)}°C · SIMULATED SENSOR`;
    }

    // Activity readouts match the original Svelte component exactly.
    switch (component) {
      case 'CPU': return `${frame.cpu.usage.toFixed(1)}% PACKAGE · ${frame.cpu.cores.length} THREADS`;
      case 'MEMORY': return `${frame.memory.usedGiB.toFixed(2)} / ${frame.memory.totalGiB} GiB`;
      case 'GPU': return `${frame.gpu.usage.toFixed(1)}% · ${frame.gpu.temperatureC.toFixed(0)}°C`;
      case 'NVMe': return `R ${frame.storage.readMiBs.toFixed(1)} / W ${frame.storage.writeMiBs.toFixed(1)} MiB/s`;
      case 'NETWORK': return `RX ${frame.network.rxMiBs.toFixed(1)} / TX ${frame.network.txMiBs.toFixed(1)} MiB/s`;
    }
  }

  function focusText(): string {
    switch (displayMode) {
      case 'activity': return detail(selected);
      case 'thermal': return detail(selected);
      case 'topology': return `CONCEPTUAL NODE / ${selected}`;
    }
  }
</script>

<div class="schematic machine-v2" data-mode={displayMode}>
  <div class="schematic-background" aria-hidden="true"></div>
  <div class="machine-v2-noise" aria-hidden="true"></div>

  <div class="machine-v2-corner machine-v2-corner-left" aria-hidden="true">
    <span>◈ SYSTEM LATTICE</span>
    <small>WYN / OBSERVATORY</small>
  </div>
  <div class="machine-v2-corner machine-v2-corner-right" aria-hidden="true">
    <span>◇ MOCK TELEMETRY</span>
    <small>05 · LOGICAL NODES</small>
  </div>

  <!-- Keep the original topology and traveling electricity. Add selection glow
       as a separate SVG overlay rather than replacing the original wiring. -->
  <svg class="schematic-wires" viewBox="0 0 800 510" preserveAspectRatio="none" aria-hidden="true">
    <path d="M400 146 V180 H164 V215 M400 180 H636 V215 M164 307 V347 H292 V386 M636 307 V347 H508 V386" />
    <path d="M400 146 V180 H164 V215 M400 180 H636 V215 M164 307 V347 H292 V386 M636 307 V347 H508 V386"
      class="schematic-electric" pathLength="100" />
    {#each links as link (link.id)}
      <path d={link.d} class="machine-v2-route" class:focused={selected === link.from || selected === link.to} />
    {/each}
    <circle cx="400" cy="180" r="4" class="schematic-hub" />
    <circle cx="400" cy="180" r="10" class="machine-v2-hub-halo" />
  </svg>

  <div class="machine-v2-modes" role="group" aria-label="Schematic display mode">
    <button class:chosen={displayMode === 'activity'} aria-pressed={displayMode === 'activity'}
      onclick={() => displayMode = 'activity'} title="Show the original activity readouts and throughput bars">◉ ACTIVITY</button>
    <button class:chosen={displayMode === 'thermal'} aria-pressed={displayMode === 'thermal'}
      onclick={() => displayMode = 'thermal'} title="Show available synthetic thermal sensors only">♨ THERMAL</button>
    <button class:chosen={displayMode === 'topology'} aria-pressed={displayMode === 'topology'}
      onclick={() => displayMode = 'topology'} title="Show conceptual, non-physical component relationships">◇ TOPOLOGY</button>
  </div>

  {#each components as component (component)}
    <button
      class="machine-node machine-v2-node"
      class:active={selected === component}
      class:thermal-available={displayMode === 'thermal' && temperature(component) !== null}
      data-kind={component}
      style={`--machine-v2-glow: ${(30 * heatOpacity(component)).toFixed(1)}px`}
      onclick={() => choose(component)}
      aria-pressed={selected === component}
      aria-label={`Inspect ${component}: ${detail(component)}`}
      title={`Inspect ${component} · synthetic demo`}
    >
      <span class="machine-v2-node-heading">
        <span class="node-kicker">◇ {component === 'CPU' ? 'PACKAGE 0' : 'SUBSYSTEM'}</span>
        <span class="machine-v2-node-status" aria-hidden="true">{selected === component ? '◈ FOCUS' : '◇ SELECT'}</span>
      </span>
      <strong>{label[component]}</strong>
      <span class="node-detail" title={detail(component)}>{detail(component)}</span>

      {#if component === 'CPU' && displayMode !== 'thermal'}
        <span class="core-row machine-v2-core-row"
          style={`grid-template-columns: repeat(${Math.max(1, frame.cpu.cores.length)}, minmax(0, 1fr))`}
          aria-label={`${frame.cpu.cores.length} simulated logical CPU activity indicators`}>
          {#each frame.cpu.cores as core (core.id)}
            <span class="core-cell" title={`Simulated logical CPU ${core.id}: ${core.usage.toFixed(1)}% usage`}
              style:opacity={displayMode === 'topology' ? 0.65 : Math.min(1, 0.25 + core.usage / 140)}></span>
          {/each}
        </span>
      {:else if displayMode === 'activity'}
        <span class="node-progress"><span style:width={`${progress(component)}%`}></span></span>
      {:else if displayMode === 'thermal'}
        <span class="machine-v2-sensor-bar" class:unavailable={temperature(component) === null}
          title="Illustrative 0–105°C display scale, not a hardware safety threshold">
          {#if temperature(component) === null}
            <span>◇ NO SENSOR CHANNEL</span>
          {:else}
            <span class="machine-v2-sensor-fill" style:width={`${clampPercent((temperature(component) ?? 0) / 105 * 100)}%`}></span>
          {/if}
        </span>
      {:else}
        <span class="machine-v2-topology-label">◇ CONCEPTUAL CONNECTION · NOT A BUS TRACE</span>
      {/if}
    </button>
  {/each}

  <span class="machine-v2-focus" aria-live="off"><b>◈ {selected}</b><span>{focusText()}</span></span>
  <span class="schematic-key">◇ LOGICAL / SCHEMATIC &nbsp;·&nbsp; PHYSICAL PLACEMENT NOT IMPLIED</span>
</div>
