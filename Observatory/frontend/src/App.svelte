<script lang="ts">
  import { onMount } from 'svelte';
  import MachineSchematic from './lib/components/MachineSchematic.svelte';
  import TrendChart from './lib/components/TrendChart.svelte';
  import InstructionVein from './lib/components/InstructionVein.svelte';
  import FamiliarLayer from './lib/components/FamiliarLayer.svelte';
  import './lib/components/familiars.css';
  import './lib/components/instruction-vein.css';
  import FleetView from './lib/components/FleetView.svelte';
  import { demoDevices, demoSites, type FleetDevice, type FleetSite } from './lib/data/fleet';
  import { mockFrame, type Component, type Instruction, type TelemetryFrame } from './lib/data/telemetry';

  interface NavigationGroup { label: string; pages: string[] }
  const groups: NavigationGroup[] = [
    { label: 'CORE', pages: ['THE MACHINE', 'THE SERVER', 'FLEET'] },
    { label: 'LIVE', pages: ['CPU', 'MEMORY', 'GPU', 'PROCESSES', 'NETWORK'] },
    { label: 'NPU', pages: ['NPU OVERVIEW', 'NPU MEMORY', 'NPU RUNTIME'] },
    { label: 'DEEP ANALYSIS', pages: ['TIME MACHINE', 'CAUSALITY', 'SYSCALLS', 'FLAMEGRAPH', 'MEMORY MAP', 'SCHEDULER', 'CACHE / PMU', 'IRQ / SOFTIRQ', 'PACKET FLOW'] },
    { label: 'HARDWARE', pages: ['PCIe FABRIC', 'NUMA', 'STORAGE I/O', 'POWER / THERMAL'] },
    { label: 'SERVER STACK', pages: ['LOCKS', 'CGROUPS', 'DATABASE'] },
    { label: 'DISTRIBUTED', pages: ['TRACES', 'COMPARE', 'SNAPSHOT DIFF', 'CROSS-MACHINE'] },
    { label: 'INTELLIGENCE', pages: ['ANOMALIES', 'INCIDENTS', 'SOURCE → SILICON', 'FIRMWARE', 'SECURITY'] },
    { label: 'FORENSICS', pages: ['BINARY', 'AUTOPSY', 'KERNEL', 'LOGS'] },
  ];

  interface MetricSeries {
    title: string;
    subtitle: string;
    tint: 'primary' | 'info' | 'secondary' | 'success' | 'warning';
    unit: string;
    maximum: number;
    read: (sample: TelemetryFrame) => number;
  }
  const chartSpecs: Record<Component, [MetricSeries, MetricSeries]> = {
    CPU: [
      { title: 'CPU activity', subtitle: 'PACKAGE / UTILIZATION', tint: 'primary', unit: '%', maximum: 100, read: (s: TelemetryFrame) => s.cpu.usage },
      { title: 'CPU temperature', subtitle: 'PACKAGE / THERMAL', tint: 'warning', unit: '°C', maximum: 105, read: (s: TelemetryFrame) => s.cpu.temperatureC },
    ],
    MEMORY: [
      { title: 'Memory pressure', subtitle: 'PHYSICAL / UTILIZATION', tint: 'info', unit: '%', maximum: 100, read: (s: TelemetryFrame) => s.memory.usedPercent },
      { title: 'Memory resident', subtitle: 'PHYSICAL / USED GiB', tint: 'secondary', unit: 'GiB', maximum: 32, read: (s: TelemetryFrame) => s.memory.usedGiB },
    ],
    GPU: [
      { title: 'GPU activity', subtitle: 'GPU / COMPUTE', tint: 'secondary', unit: '%', maximum: 100, read: (s: TelemetryFrame) => s.gpu.usage },
      { title: 'GPU temperature', subtitle: 'GPU / THERMAL', tint: 'warning', unit: '°C', maximum: 110, read: (s: TelemetryFrame) => s.gpu.temperatureC },
    ],
    NVMe: [
      { title: 'Storage reads', subtitle: 'NVMe / INBOUND', tint: 'info', unit: 'MiB/s', maximum: 160, read: (s: TelemetryFrame) => s.storage.readMiBs },
      { title: 'Storage writes', subtitle: 'NVMe / OUTBOUND', tint: 'primary', unit: 'MiB/s', maximum: 100, read: (s: TelemetryFrame) => s.storage.writeMiBs },
    ],
    NETWORK: [
      { title: 'Network receive', subtitle: 'NETWORK / RX', tint: 'success', unit: 'MiB/s', maximum: 35, read: (s: TelemetryFrame) => s.network.rxMiBs },
      { title: 'Network transmit', subtitle: 'NETWORK / TX', tint: 'info', unit: 'MiB/s', maximum: 25, read: (s: TelemetryFrame) => s.network.txMiBs },
    ],
  };

  type ThemeName = 'gothic' | 'pastel' | 'cyberpunk';
  let theme = $state<ThemeName>('gothic');
  let activePage = $state<string>('THE MACHINE');
  let selected = $state<Component>('CPU');
  let playing = $state<boolean>(true);
  let speed = $state<number>(1);
  let elapsed = $state<number>(0);
  let settingsOpen = $state<boolean>(false);
  let frame = $state<TelemetryFrame>(mockFrame(0));
  // Keep the underlying frames rather than baking in CPU/memory-only histories.
  // This allows every hardware selection to reuse the same accurate timeline.
  let history = $state<{ time: number; value: TelemetryFrame }[]>(
    Array.from({ length: 80 }, (_, i: number) => {
      const time: number = (i - 79) * 0.25;
      return { time, value: mockFrame(time) };
    }),
  );
  let hoverIndex = $state<number | null>(null);
  let pinnedIndex = $state<number | null>(null);
  let chosenDeviceId = $state<string | null>(demoDevices[0]?.id ?? null);
  let chartPair = $derived(chartSpecs[selected]);
  let firstValues = $derived(history.map((sample) => chartPair[0].read(sample.value)));
  let secondValues = $derived(history.map((sample) => chartPair[1].read(sample.value)));
  let sampleTimes = $derived(history.map((sample) => sample.time));
  let focusedDevice = $derived(demoDevices.find((device: FleetDevice) => device.id === chosenDeviceId));
  // Distinct instruction sequences are retained across telemetry frames.
  // Keep this view model separate from the mutable, 250ms telemetry snapshot.
  const VEIN_BUFFER_LIMIT: number = 256;
  let veinBuffer = $state<Instruction[]>([...frame.instructions]);

  function chooseComponent(component: Component): void {
    selected = component;
    hoverIndex = null;
    pinnedIndex = null;
  }
  function pinSample(index: number | null): void {
    pinnedIndex = index;
  }

  onMount((): (() => void) => {
    let last = performance.now();
    const timer = window.setInterval((): void => {
      const now = performance.now();
      const delta = (now - last) / 1000;
      last = now;
      if (!playing) return;
      elapsed += delta * speed;
      frame = mockFrame(elapsed);
      const newestSequence: number = veinBuffer[0]?.sequence ?? -1;
      const incoming = frame.instructions
        .filter((sample) => sample.sequence > newestSequence)
        .sort((a, b) => b.sequence - a.sequence);
      if (incoming.length > 0) {
        veinBuffer = [...incoming, ...veinBuffer].slice(0, VEIN_BUFFER_LIMIT);
      }
      history = [...history.slice(1), { time: elapsed, value: frame }];
      if (pinnedIndex !== null) pinnedIndex = pinnedIndex > 0 ? pinnedIndex - 1 : null;
    }, 250);
    return (): void => window.clearInterval(timer);
  });

  function metricSummary(component: Component): { title: string; value: string; detail: string } {
    switch (component) {
      case 'CPU': return { title: 'PACKAGE LOAD', value: `${frame.cpu.usage.toFixed(1)}%`, detail: `${frame.cpu.temperatureC.toFixed(1)}°C · ${frame.cpu.contextSwitches.toLocaleString()} switches/s` };
      case 'MEMORY': return { title: 'MEMORY USED', value: `${frame.memory.usedGiB.toFixed(2)} GiB`, detail: `${frame.memory.usedPercent.toFixed(1)}% · ${frame.memory.faults}/s page faults` };
      case 'GPU': return { title: 'GPU UTILIZATION', value: `${frame.gpu.usage.toFixed(1)}%`, detail: `${frame.gpu.temperatureC.toFixed(1)}°C · ${frame.gpu.vramGiB.toFixed(1)} GiB VRAM` };
      case 'NVMe': return { title: 'STORAGE READ', value: `${frame.storage.readMiBs.toFixed(1)} MiB/s`, detail: `${frame.storage.writeMiBs.toFixed(1)} MiB/s write` };
      case 'NETWORK': return { title: 'NETWORK RX', value: `${frame.network.rxMiBs.toFixed(1)} MiB/s`, detail: `${frame.network.txMiBs.toFixed(1)} MiB/s TX` };
    }
  }
  let inspected = $derived(metricSummary(selected));
</script>

<svelte:head>
  <title>WynObserve // Observatory Preview</title>
  <meta name="theme-color" content="#09070d" />
</svelte:head>

<div class="observatory-app" data-theme={theme}>
  <header class="topbar">
    <div class="brand-mark" aria-hidden="true">◈</div>
    <div class="brand-copy">
      <div class="brand-title">Wyn<span>Observe</span><span class="brand-slash"> // </span><small>OBSERVATORY</small></div>
      <div class="brand-subtitle">systems visualization laboratory <span>·</span> an instrument for the invisible</div>
    </div>
    <div class="topbar-spacer"></div>
    <span class="status-chip demo-chip">◇ MOCK DATA</span>
    <span class="status-chip"><i class="status-dot"></i>{playing ? 'SIMULATING' : 'PAUSED'}</span>
    <span class="target-label">TARGET <b>WYN / DEMO-01</b></span>
    <button class="chrome-button" onclick={() => settingsOpen = true}>⚙ SETTINGS</button>
  </header>

  <div class="workspace">
    <nav class="sidebar" aria-label="Observatory navigation">
      <div class="sidebar-top"><span class="micro-label">NAVIGATION</span><span class="sidebar-index">01 / ∞</span></div>
      {#each groups as group}
        <section class="nav-group">
          <h2>{group.label}</h2>
          {#each group.pages as page}
            <button class:current={activePage === page} onclick={() => activePage = page}>
              <span class="nav-glyph">{activePage === page ? '◆' : '◇'}</span>{page}
            </button>
          {/each}
        </section>
      {/each}
      <div class="sidebar-note"><strong>◉ VISUAL PROTOTYPE</strong><span>Svelte 5 / Tauri 2</span><span>Data source: synthetic</span></div>
    </nav>

    <main class="content">
      <!-- WYNOBSERVE FAMILIARS V1 / additive overlay -->
      <div class="familiars-anchor">
        <FamiliarLayer {frame} devices={demoDevices} {selected} source="demo" />
      </div>
      {#if activePage === 'THE MACHINE'}
        <div class="page-heading">
          <div><div class="eyebrow">CORE / OBSERVATORY / 001</div><h1>THE <em>MACHINE</em></h1>
            <p>An animated logical view of the hardware, with real instrument behavior coming next.</p></div>
          <span class="truth-tag">◇ SCHEMATIC</span>
        </div>

        <div class="summary-grid">
          <div class="metric-card"><span>PACKAGE LOAD</span><strong>{frame.cpu.usage.toFixed(1)}<small>%</small></strong><div class="metric-bottom"><b class="pink">●</b> 16 logical CPUs</div></div>
          <div class="metric-card"><span>MEMORY PRESSURE</span><strong>{frame.memory.usedPercent.toFixed(1)}<small>%</small></strong><div class="metric-bottom"><b class="blue">●</b> {frame.memory.usedGiB.toFixed(2)} / 32 GiB</div></div>
          <div class="metric-card"><span>GRAPHICS LOAD</span><strong>{frame.gpu.usage.toFixed(1)}<small>%</small></strong><div class="metric-bottom"><b class="violet">●</b> {frame.gpu.temperatureC.toFixed(0)}°C · demo RTX</div></div>
          <div class="metric-card"><span>NETWORK INGRESS</span><strong>{frame.network.rxMiBs.toFixed(1)}<small>MiB/s</small></strong><div class="metric-bottom"><b class="green">●</b> {frame.network.txMiBs.toFixed(1)} MiB/s TX</div></div>
        </div>

        <div class="panel-heading"><div><span class="eyebrow">THE OBSERVABLE SYSTEM</span><h2>Logical topology</h2></div><span class="quiet">SELECT A COMPONENT TO INSPECT ↗</span></div>
        <MachineSchematic {frame} {selected} choose={chooseComponent} />

        <div class="panel-heading chart-group-heading"><div><span class="eyebrow">FOCUSED SIGNALS / {selected}</span><h2>Telemetry channels</h2></div>
          <div class="focus-hint">HOVER TO SYNC · CLICK TO PIN {#if pinnedIndex !== null}<button class="unpin-button" onclick={() => pinSample(null)}>✕ UNPIN</button>{/if}</div></div>
        <div class="charts-grid">
          <TrendChart title={chartPair[0].title} subtitle={chartPair[0].subtitle} values={firstValues} times={sampleTimes}
            tint={chartPair[0].tint} unit={chartPair[0].unit} maximum={chartPair[0].maximum}
            {hoverIndex} {pinnedIndex} onHover={(index: number | null) => hoverIndex = index} onPin={pinSample} />
          <TrendChart title={chartPair[1].title} subtitle={chartPair[1].subtitle} values={secondValues} times={sampleTimes}
            tint={chartPair[1].tint} unit={chartPair[1].unit} maximum={chartPair[1].maximum}
            {hoverIndex} {pinnedIndex} onHover={(index: number | null) => hoverIndex = index} onPin={pinSample} />
        </div>

        <div class="process-panel">
          <div class="panel-heading"><div><span class="eyebrow">WORKLOAD</span><h2>Active processes</h2></div><span class="truth-tag">◉ SIMULATED</span></div>
          <div class="process-header"><span>PROCESS</span><span>PID</span><span>CPU</span><span>MEMORY</span></div>
          {#each [...frame.processes].sort((a, b) => b.cpu - a.cpu) as process (process.pid)}
            <div class="process-row"><span><i class="proc-indicator"></i>{process.name}</span><span>{process.pid}</span><span>{process.cpu.toFixed(1)}%</span><span>{process.memoryGiB.toFixed(2)} GiB</span></div>
          {/each}
        </div>
      {:else if activePage === 'FLEET'}
        <FleetView selectedDeviceId={chosenDeviceId} selectDevice={(id: string) => chosenDeviceId = id} />
      {:else}
        <div class="page-heading"><div><div class="eyebrow">WYNOBSERVE / MIGRATION</div><h1>{activePage}</h1><p>This section exists in the Rust Observatory and has not been migrated yet.</p></div></div>
        <div class="coming-soon"><div>◇</div><h2>Awaiting instrumentation</h2><p>Navigation has been migrated. Next we will port this view using the existing Rust data contracts, not substitute invented measurements.</p><button class="chrome-button" onclick={() => activePage = 'THE MACHINE'}>← RETURN TO THE MACHINE</button></div>
      {/if}
    </main>

    <aside class="inspector" class:with-vein={activePage !== 'FLEET'}>
      {#if activePage === 'FLEET'}
        <div class="inspector-heading"><span class="eyebrow">FLEET / ENDPOINT</span><h2>{focusedDevice?.name ?? 'NO SELECTION'}</h2><p>SELECT AN ENDPOINT IN THE LIST OR MAP</p></div>
        {#if focusedDevice}
          <div class="inspector-data"><span>CONNECTION STATUS</span><strong class:inspector-online={focusedDevice.status === 'online'}>{focusedDevice.status.toUpperCase()}</strong>
            <small>OFFICE · {demoSites.find((site: FleetSite) => site.id === focusedDevice.siteId)?.name}</small></div>
          <div class="fleet-inspector-detail">
            <div><span>ROLE</span><strong>{focusedDevice.role}</strong></div>
            <div><span>OS</span><strong>{focusedDevice.os}</strong></div>
            <div><span>CPU</span><strong>{focusedDevice.status === 'offline' ? 'N/A' : `${focusedDevice.cpu}%`}</strong></div>
            <div><span>MEMORY</span><strong>{focusedDevice.status === 'offline' ? 'N/A' : `${focusedDevice.memory}%`}</strong></div>
            <div><span>AGENT</span><strong>V{focusedDevice.agentVersion}</strong></div>
            <div><span>LAST SEEN</span><strong>{focusedDevice.lastSeen}</strong></div>
          </div>
        {/if}
        <div class="inspector-provenance"><span class="truth-tag">◇ DEMO ENDPOINT</span><p>These are not connected machines. Site coordinates, health states, and performance metrics are synthetic previews.</p></div>
      {:else}
      <div class="inspector-heading"><span class="eyebrow">FOCUSED INSPECTION</span><h2>{selected}</h2><p>CLICK A NODE TO CHANGE FOCUS</p></div>
      <div class="inspector-data"><span>{inspected.title}</span><strong>{inspected.value}</strong><small>{inspected.detail}</small></div>
      <div class="inspector-provenance"><span class="truth-tag">◉ SIMULATED</span><p>These values are generated locally. No Agent, Hub, or perf connection is active.</p></div>
      <InstructionVein samples={veinBuffer} component={selected} globalPaused={!playing} />
      {/if}
    </aside>
  </div>

  <div class="timeline">
    <button class="control primary-control" onclick={() => playing = !playing}>{playing ? 'Ⅱ PAUSE' : '▶ RESUME'}</button>
    <span class="vertical-rule"></span>
    {#each [0.25, 1, 4] as factor}
      <button class:chosen={speed === factor} class="control" onclick={() => speed = factor}>{factor}×</button>
    {/each}
    <span class="vertical-rule"></span>
    <span class="timer">T+{elapsed.toFixed(2).padStart(8, '0')}s</span>
    <span class="timeline-spacer"></span>
    <span class="timeline-legend">◇ topology <span>·</span> ≈ flow <span>·</span> ◉ instruction drip</span>
  </div>
  <footer class="version-footer"><span>Agent <b>V0.2.0</b></span><span>Observatory <b>V0.2.1</b></span><span>Updater <b>V0.1.3</b></span><small>PREVIEW LABELS</small></footer>

  {#if settingsOpen}
    <button class="modal-backdrop" aria-label="Close settings" onclick={() => settingsOpen = false}></button>
    <div class="settings-panel" role="dialog" aria-modal="true" aria-label="Preview settings" tabindex="-1">
      <header><div><span class="eyebrow">LOCAL PREVIEW</span><h2>SETTINGS // THEMES</h2></div><button class="chrome-button" onclick={() => settingsOpen = false} aria-label="Close settings">✕</button></header>
      <p>Test the original Observatory color presets. This preview currently lives inside the Tauri window. The separate native Settings viewport comes later.</p>
      <div class="theme-options">
        {#each [['gothic', 'Gothic'], ['pastel', 'Pastel Dusk'], ['cyberpunk', 'Cyberpunk']] as preset}
          <button class:chosen={theme === preset[0]} onclick={() => theme = preset[0] as ThemeName}>{preset[1]}</button>
        {/each}
      </div>
      <p class="muted">Original Rust theme values preserved. Custom color editor and preference persistence are upcoming milestones.</p>
    </div>
  {/if}
</div>
