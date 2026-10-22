<script lang="ts">
  import { onMount, tick } from 'svelte';
  import CpuChamber from './lib/components/CpuChamber.svelte';
  import CpuIdentity from './lib/components/CpuIdentity.svelte';
  import BoardIdentity from './lib/components/BoardIdentity.svelte';
  import MemoryChamber from './lib/components/MemoryChamber.svelte';
  import NpuChamber from './lib/components/NpuChamber.svelte';
  import GpuChamber from './lib/components/GpuChamber.svelte';
  import ProcessChamber from './lib/components/ProcessChamber.svelte';
  import './lib/components/hardware-chambers.css';
  import MachineSchematic from './lib/components/MachineSchematic.svelte';
  import MotherboardView from './lib/components/MotherboardView.svelte';
  import { machineNodes, machineGpus, focusId, type MachineComponent } from './lib/data/machine';
  import TrendChart from './lib/components/TrendChart.svelte';
  import InstructionVein from './lib/components/InstructionVein.svelte';
  import FamiliarLayer from './lib/components/FamiliarLayer.svelte';
  import './lib/components/familiars.css';
  import './lib/components/instruction-vein.css';
  import FleetView from './lib/components/FleetView.svelte';
  import { demoDevices, demoSites, type FleetDevice, type FleetSite } from './lib/data/fleet';
  import { mockFrame, type Instruction, type TelemetryFrame } from './lib/data/telemetry';

  import { isDesktop, readFleet, frameFromWire, fleetDevices, chooseMachine, appendFrame, formatNumber, gpuReading, type FleetFeed } from './lib/data/bridge';

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
  const chartSpecs: Record<MachineComponent, [MetricSeries, MetricSeries]> = {
    CPU: [
      { title: 'CPU activity', subtitle: 'PACKAGE / UTILIZATION', tint: 'primary', unit: '%', maximum: 100, read: (s: TelemetryFrame) => s.cpu.usage },
      { title: 'CPU temperature', subtitle: 'CPU / THERMAL', tint: 'warning', unit: '°C', maximum: 105, read: (s: TelemetryFrame) => s.cpu.temperatureC },
    ],
    MEMORY: [
      { title: 'Memory pressure', subtitle: 'PHYSICAL / UTILIZATION', tint: 'info', unit: '%', maximum: 100, read: (s: TelemetryFrame) => s.memory.usedPercent },
      { title: 'Memory resident', subtitle: 'PHYSICAL / USED GiB', tint: 'secondary', unit: 'GiB', maximum: 32, read: (s: TelemetryFrame) => s.memory.usedGiB },
    ],
    GPU: [
      { title: 'GPU activity', subtitle: 'GPU / COMPUTE', tint: 'secondary', unit: '%', maximum: 100, read: (s: TelemetryFrame) => s.gpu.usage },
      { title: 'GPU temperature', subtitle: 'GPU / THERMAL', tint: 'warning', unit: '°C', maximum: 110, read: (s: TelemetryFrame) => s.gpu.temperatureC },
    ],
    NPU: [
      { title: 'NPU activity', subtitle: 'NPU / DEVICE UTILIZATION', tint: 'success', unit: '%', maximum: 100, read: (s: TelemetryFrame) => s.npu.available ? s.npu.usage : NaN },
      { title: 'NPU resident memory', subtitle: 'NPU / RESIDENT MiB', tint: 'info', unit: 'MiB', maximum: 1, read: (s: TelemetryFrame) => s.npu.available ? s.npu.residentMiB : NaN },
    ],
    NVMe: [
      { title: 'Storage reads', subtitle: 'STORAGE / READ', tint: 'info', unit: 'MiB/s', maximum: 160, read: (s: TelemetryFrame) => s.storage.readMiBs },
      { title: 'Storage writes', subtitle: 'STORAGE / WRITE', tint: 'primary', unit: 'MiB/s', maximum: 100, read: (s: TelemetryFrame) => s.storage.writeMiBs },
    ],
    NETWORK: [
      { title: 'Network receive', subtitle: 'NETWORK / RX', tint: 'success', unit: 'MiB/s', maximum: 35, read: (s: TelemetryFrame) => s.network.rxMiBs },
      { title: 'Network transmit', subtitle: 'NETWORK / TX', tint: 'info', unit: 'MiB/s', maximum: 25, read: (s: TelemetryFrame) => s.network.txMiBs },
    ],
  };

  type ThemeName = 'gothic' | 'pastel' | 'cyberpunk';
  let theme = $state<ThemeName>('gothic');
  let activePage = $state<string>('THE MACHINE');
  let contentScrollElement: HTMLElement | undefined;
  let workspaceScrollElement: HTMLElement | undefined;
  let previousNavigationPage: string | undefined;
  // The same scroll container is reused across tabs. Reset only on navigation,
  // not on telemetry updates, selection, or normal user scrolling.
  $effect((): void => {
    const currentPage: string = activePage;
    if (previousNavigationPage === undefined) {
      previousNavigationPage = currentPage;
      return;
    }
    if (previousNavigationPage === currentPage) return;
    previousNavigationPage = currentPage;
    void tick().then((): void => {
      if (contentScrollElement) contentScrollElement.scrollTop = 0;
      if (workspaceScrollElement) workspaceScrollElement.scrollTop = 0;
      const inspector = document.querySelector<HTMLElement>('.inspector');
      if (inspector) inspector.scrollTop = 0;
    });
  });

  let selected = $state<MachineComponent>('CPU');
  let selectedGpuId = $state<string | null>(null);
  let machineView = $state<'logical' | 'motherboard'>('logical');
  let playing = $state<boolean>(true);
  let speed = $state<number>(1);
  let elapsed = $state<number>(0);
  let settingsOpen = $state<boolean>(false);
  const initialFrame: TelemetryFrame = mockFrame(0);

  const initialSource = isDesktop() ? 'live' : 'demo';
  let source = $state<'demo' | 'live'>(initialSource);
  let feed = $state<FleetFeed | null>(null);
  let bridgeError = $state<string | null>(null);
  let liveTargetId = $state<string | null>(null);
  let frame = $state<TelemetryFrame>(initialFrame);
  let liveDevices = $derived(feed ? fleetDevices(feed) : []);
  let devices = $derived(source === 'demo' ? demoDevices : liveDevices);
  let liveTarget = $derived(feed?.machines.find(m => m.machine.machine_id === liveTargetId));
  let hasFrame = $state(false);
  let targetStale = $derived(source === 'live' && (feed?.stale !== false || !liveTarget || liveTarget.status !== 'online' || liveTarget.last_seen_ms > 5000));
  function resetSignals(): void {
    history = []; veinBuffer = []; hoverIndex = null; pinnedIndex = null; hasFrame = false;
    selected = 'CPU'; selectedGpuId = null;
  }
  function selectMachine(id: string): void {
    if (source === 'demo') { chosenDeviceId = id; return; }
    if (liveTargetId !== id) { liveTargetId = id; resetSignals(); }
  }
  function setSource(next: 'demo' | 'live'): void {
    source = next; resetSignals();
    if (next === 'demo') { frame = mockFrame(elapsed); hasFrame = true; }
  }
  // Keep the underlying frames rather than baking in CPU/memory-only histories.
  // This allows every hardware selection to reuse the same accurate timeline.
  let history = $state<{ time: number; value: TelemetryFrame }[]>(
    Array.from({ length: initialSource === 'demo' ? 80 : 0 }, (_, i: number) => {
      const time: number = (i - 79) * 0.25;
      return { time, value: mockFrame(time) };
    }),
  );
  let hoverIndex = $state<number | null>(null);
  let pinnedIndex = $state<number | null>(null);
  let chosenDeviceId = $state<string | null>(demoDevices[0]?.id ?? null);
  let hardware = $derived(machineNodes(frame));
  let graphics = $derived(machineGpus(frame));
  let effectiveGpuId = $derived(selectedGpuId ?? frame.gpu.id ?? graphics[0]?.gpuId ?? null);
  let focusedHardware = $derived(hardware.find(node => node.id === focusId(selected, effectiveGpuId)));
  let focusName = $derived(focusedHardware?.name ?? `${selected} unavailable`);
  let selectedGpu = $derived(gpuReading(frame, effectiveGpuId));
  let chartPair = $derived.by((): [MetricSeries, MetricSeries] => {
    if (selected !== 'GPU') return chartSpecs[selected];
    // Bind every historical sample to one adapter identity, even after inventory reordering.
    const id = effectiveGpuId;
    return [
      { ...chartSpecs.GPU[0], subtitle: focusName, read: s => gpuReading(s, id)?.usage ?? NaN },
      { ...chartSpecs.GPU[1], subtitle: focusName, read: s => gpuReading(s, id)?.temperatureC ?? NaN },
    ];
  });
  let firstValues = $derived(history.map((sample) => chartPair[0].read(sample.value)));
  let secondValues = $derived(history.map((sample) => chartPair[1].read(sample.value)));
  let sampleTimes = $derived(history.map((sample) => sample.time));
  let secondMaximum = $derived(selected === 'NPU'
    ? Math.max(1, ...secondValues.filter(Number.isFinite)) * 1.1
    : selected === 'MEMORY' && Number.isFinite(frame.memory.totalGiB)
      ? Math.max(frame.memory.totalGiB, 1) : chartPair[1].maximum);
  let focusedDevice = $derived(devices.find((device: FleetDevice) => device.id === (source === 'demo' ? chosenDeviceId : liveTargetId)));
  // Distinct instruction sequences are retained across telemetry frames.
  // Keep this view model separate from the mutable, 250ms telemetry snapshot.
  const VEIN_BUFFER_LIMIT: number = 256;
  let veinBuffer = $state<Instruction[]>(initialSource === 'demo' ? [...initialFrame.instructions] : []);

  // WYNOBSERVE-HARDWARE-CHAMBERS-MIGRATION-01
  // Navigation keeps the Machine inspector synchronized with a hardware page.
  function navigateTo(page: string): void {
    if (page === 'CPU' || page === 'MEMORY' || page === 'GPU') {
      chooseComponent(page);
    }
    activePage = page;
  }

  function chooseComponent(component: MachineComponent, gpuId?: string): void {
    selected = component;
    if (component === 'GPU') selectedGpuId = gpuId ?? effectiveGpuId;
    hoverIndex = null;
    pinnedIndex = null;
  }
  function pinSample(index: number | null): void {
    pinnedIndex = index;
  }

  onMount((): (() => void) => {
    let last = performance.now();
    let alive = true;
    let pending = false;
    async function refreshFleet(): Promise<void> {
      if (source !== 'live' || pending) return;
      pending = true;
      try {
        const next = await readFleet();
        if (!alive || source !== 'live') return;
        feed = next; bridgeError = null;
        liveTargetId = chooseMachine(liveTargetId, next.machines);
        const machine = next.machines.find(m => m.machine.machine_id === liveTargetId);
        if ((playing || !hasFrame) && machine?.snapshot && !next.stale && machine.status === 'online' && machine.last_seen_ms <= 5000) {
          const updated = frameFromWire(machine.snapshot);
          frame = updated; hasFrame = true;
          const previousLength = history.length;
          const updatedHistory = appendFrame(history, updated);
          if (updatedHistory !== history && pinnedIndex !== null && previousLength >= 80) pinnedIndex = pinnedIndex > 0 ? pinnedIndex - 1 : null;
          history = updatedHistory;
        }
      } catch (error) {
        if (alive) { bridgeError = String(error); if (feed) feed = { ...feed, stale: true, connection: 'disconnected' }; }
      } finally { pending = false; }
    }
    void refreshFleet();
    const timer = window.setInterval((): void => {
      const now = performance.now();
      const delta = (now - last) / 1000;
      last = now;
      if (source === 'live') { void refreshFleet(); return; }
      hasFrame = true;
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
      history = [...history.slice(-79), { time: elapsed, value: frame }];
      if (pinnedIndex !== null) pinnedIndex = pinnedIndex > 0 ? pinnedIndex - 1 : null;
    }, 250);
    return (): void => { alive = false; window.clearInterval(timer); };
  });

  function metricSummary(component: MachineComponent): { title: string; value: string; detail: string } {
    switch (component) {
      case 'CPU': return { title: 'PACKAGE LOAD', value: `${formatNumber(frame.cpu.usage, 1)}%`, detail: `${formatNumber(frame.cpu.temperatureC, 1)}°C · ${formatNumber(frame.cpu.contextSwitches, 0)} switches/s` };
      case 'MEMORY': return { title: 'MEMORY USED', value: `${formatNumber(frame.memory.usedGiB, 2)} GiB`, detail: `${formatNumber(frame.memory.usedPercent, 1)}% · ${formatNumber(frame.memory.faults, 0)}/s page faults` };
      case 'GPU': return { title: 'GPU UTILIZATION', value: `${formatNumber(selectedGpu?.usage ?? NaN, 1)}%`, detail: `${formatNumber(selectedGpu?.temperatureC ?? NaN, 1)}°C · ${formatNumber(selectedGpu?.vramGiB ?? NaN, 1)} GiB VRAM` };
      case 'NPU': return { title: 'NPU UTILIZATION', value: `${formatNumber(frame.npu.available ? frame.npu.usage : NaN, 1)}%`, detail: `${formatNumber(frame.npu.available ? frame.npu.residentMiB : NaN, 2)} MiB resident · ${frame.npu.powerState ?? 'Power state unavailable'}` };
      case 'NVMe': return { title: 'STORAGE READ', value: `${formatNumber(frame.storage.readMiBs, 1)} MiB/s`, detail: `${formatNumber(frame.storage.writeMiBs, 1)} MiB/s write` };
      case 'NETWORK': return { title: 'NETWORK RX', value: `${formatNumber(frame.network.rxMiBs, 1)} MiB/s`, detail: `${formatNumber(frame.network.txMiBs, 1)} MiB/s TX` };
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
    <label class="source-selector">SOURCE <select aria-label="Telemetry source" value={source} onchange={(event) => setSource(event.currentTarget.value as 'demo' | 'live')}><option value="live">LIVE HUB</option><option value="demo">DEMO</option></select></label>
    <span class="status-chip"><i class="status-dot"></i>{!playing ? 'PAUSED' : source === 'demo' ? 'SIMULATING' : targetStale ? 'WAITING / STALE' : 'LIVE'}</span>
    <span class="target-label">TARGET <b>{source === 'demo' ? 'DEMO-01' : liveTarget?.machine.machine_name ?? 'SELECT IN FLEET'}</b></span>
    <button class="chrome-button" onclick={() => settingsOpen = true}>⚙ SETTINGS</button>
  </header>

  <div class="workspace" bind:this={workspaceScrollElement}>
    <nav class="sidebar" aria-label="Observatory navigation">
      <div class="sidebar-top"><span class="micro-label">NAVIGATION</span><span class="sidebar-index">01 / ∞</span></div>
      {#each groups as group}
        <section class="nav-group">
          <h2>{group.label}</h2>
          {#each group.pages as page}
            <button class:current={activePage === page} onclick={() => navigateTo(page)}>
              <span class="nav-glyph">{activePage === page ? '◆' : '◇'}</span>{page}
            </button>
          {/each}
        </section>
      {/each}
      <div class="sidebar-note"><strong>◉ VISUAL PROTOTYPE</strong><span>Svelte 5 / Tauri 2</span><span>Data source: {source === 'demo' ? 'synthetic' : 'Agent → Hub'}</span></div>
    </nav>

    <main class="content" bind:this={contentScrollElement}>
      {#key `${source}:${liveTargetId}`}
      <!-- WYNOBSERVE FAMILIARS V1 / additive overlay -->
      <div class="familiars-anchor">
        {#if source === 'demo' || hasFrame}<FamiliarLayer {frame} {devices} selected={selected === 'NPU' ? undefined : selected} source={source === 'demo' ? 'demo' : 'observed'} />{/if}
      </div>
      {#if source === 'live'}
        <div class="fleet-banner" role="status"><span>{targetStale ? '◇ WAITING / STALE' : !playing ? 'Ⅱ PAUSED READINGS' : '● LIVE TELEMETRY'} · {liveTarget?.machine.machine_name ?? 'NO TARGET'}</span><p>{bridgeError ?? feed?.error ?? (targetStale ? 'Waiting for a fresh snapshot from the selected machine. Last readings, when present, are retained.' : `Connected to ${feed?.endpoint}. Fleet selection controls the hardware pages.`)}</p></div>
      {/if}
      {#if source === 'live' && !hasFrame && activePage !== 'FLEET'}
        <div class="coming-soon"><h2>Waiting for live telemetry</h2><p>Select a connected machine in Fleet, or choose Demo to explore the interface.</p><button class="chrome-button" onclick={() => navigateTo('FLEET')}>OPEN FLEET →</button></div>
      {:else if activePage === 'THE MACHINE'}
        <div class="page-heading machine-page-heading">
          <div><div class="eyebrow">CORE / OBSERVATORY / 001</div><h1>THE <em>MACHINE</em></h1>
            <p>Explore the selected machine’s hardware. Select a component in either view to follow its signals.</p></div>
          <div class="fleet-view-switch machine-view-switch" role="group" aria-label="Machine display mode">
            <button class:chosen={machineView === 'logical'} aria-pressed={machineView === 'logical'} onclick={() => machineView = 'logical'}>◇ LOGICAL VIEW</button>
            <button class:chosen={machineView === 'motherboard'} aria-pressed={machineView === 'motherboard'} onclick={() => machineView = 'motherboard'}>▦ MOTHERBOARD</button>
          </div>
        </div>

        <BoardIdentity board={frame.motherboard} demo={source === 'demo'}/>
        {#if machineView === 'logical'}
        <div class="summary-grid">
          <CpuIdentity cpu={frame.cpu} />
          <div class="metric-card"><span>PACKAGE LOAD</span><strong>{formatNumber(frame.cpu.usage, 1)}<small>%</small></strong><div class="metric-bottom" title={frame.cpu.temperatureSource ?? 'CPU temperature unavailable'}><b class="pink">●</b> {formatNumber(frame.cpu.temperatureC, 1)}°C · {frame.cpu.cores.length} logical CPUs</div></div>
          <div class="metric-card"><span>MEMORY PRESSURE</span><strong>{formatNumber(frame.memory.usedPercent, 1)}<small>%</small></strong><div class="metric-bottom"><b class="blue">●</b> {formatNumber(frame.memory.usedGiB, 2)} / {formatNumber(frame.memory.totalGiB, 1)} GiB</div></div>
          {#each graphics as gpu (gpu.id)}
            <div class="metric-card"><span>{gpu.role}</span><strong>{formatNumber(gpu.activity, 1)}<small>%</small></strong><div class="metric-bottom" title={gpu.name}><b class="violet">●</b> {formatNumber(gpu.temperatureC, 1)}°C · {gpu.name}</div></div>
          {/each}
          {#if frame.npu.available}
            <div class="metric-card"><span>NPU · {frame.npu.name}</span><strong>{formatNumber(frame.npu.usage, 1)}<small>%</small></strong><div class="metric-bottom"><b class="green">●</b> {formatNumber(frame.npu.residentMiB, 2)} MiB resident</div></div>
          {/if}
          <div class="metric-card"><span>NETWORK INGRESS</span><strong>{formatNumber(frame.network.rxMiBs, 1)}<small>MiB/s</small></strong><div class="metric-bottom"><b class="green">●</b> {formatNumber(frame.network.txMiBs, 1)} MiB/s TX</div></div>
        </div>

        {/if}

        <div class="panel-heading"><div><span class="eyebrow">THE OBSERVABLE SYSTEM</span><h2>{machineView === 'logical' ? 'Logical topology' : 'Motherboard'}</h2></div><span class="quiet">SELECT A COMPONENT TO INSPECT ↗</span></div>
        {#if machineView === 'logical'}
          <MachineSchematic {frame} {selected} selectedGpuId={effectiveGpuId} choose={chooseComponent} paused={!playing || (source === 'live' && targetStale)} />
        {:else}
          <MotherboardView {frame} {history} {selected} selectedGpuId={effectiveGpuId} choose={chooseComponent} paused={!playing || (source === 'live' && targetStale)} />
        {/if}

        <div class="panel-heading chart-group-heading"><div><span class="eyebrow">FOCUSED SIGNALS / {focusName}</span><h2>Telemetry channels</h2></div>
          <div class="focus-hint">HOVER TO SYNC · CLICK TO PIN {#if pinnedIndex !== null}<button class="unpin-button" onclick={() => pinSample(null)}>✕ UNPIN</button>{/if}</div></div>
        <div class="charts-grid">
          <TrendChart title={chartPair[0].title} subtitle={chartPair[0].subtitle} values={firstValues} times={sampleTimes}
            tint={chartPair[0].tint} unit={chartPair[0].unit} maximum={selected === 'MEMORY' ? 100 : chartPair[0].maximum}
            {hoverIndex} {pinnedIndex} onHover={(index: number | null) => hoverIndex = index} onPin={pinSample} />
          <TrendChart title={chartPair[1].title} subtitle={chartPair[1].subtitle} values={secondValues} times={sampleTimes}
            tint={chartPair[1].tint} unit={chartPair[1].unit} maximum={secondMaximum}
            {hoverIndex} {pinnedIndex} onHover={(index: number | null) => hoverIndex = index} onPin={pinSample} />
        </div>

        <div class="process-panel">
          <div class="panel-heading"><div><span class="eyebrow">WORKLOAD</span><h2>Active processes</h2></div><span class="truth-tag">{source === 'demo' ? '◇ SIMULATED' : '● OBSERVED'}</span></div>
          <div class="process-header"><span>PROCESS</span><span>PID</span><span>CPU</span><span>MEMORY</span></div>
          {#each [...frame.processes].sort((a, b) => b.cpu - a.cpu) as process (process.pid)}
            <div class="process-row"><span><i class="proc-indicator"></i>{process.name}</span><span>{process.pid}</span><span>{formatNumber(process.cpu, 1)}%</span><span>{formatNumber(process.memoryGiB, 2)} GiB</span></div>
          {/each}
        </div>
      {:else if activePage === 'CPU'}
        <CpuChamber {frame} {history} />
      {:else if activePage === 'MEMORY'}
        <MemoryChamber {frame} {history} openProcesses={() => navigateTo('PROCESSES')} />
      {:else if activePage === 'GPU'}
        <GpuChamber {frame} {history} />
      {:else if activePage.startsWith('NPU ')}
        {#key activePage}<NpuChamber {frame} {history} view={activePage === 'NPU MEMORY' ? 'memory' : activePage === 'NPU RUNTIME' ? 'runtime' : 'overview'} />{/key}
      {:else if activePage === 'PROCESSES'}
        <ProcessChamber {frame} openMemory={() => navigateTo('MEMORY')} />
      {:else if activePage === 'FLEET'}
        <FleetView {devices} {source} selectedDeviceId={source === 'demo' ? chosenDeviceId : liveTargetId} selectDevice={selectMachine} />
      {:else}
        <div class="page-heading"><div><div class="eyebrow">WYNOBSERVE / MIGRATION</div><h1>{activePage}</h1><p>This section exists in the Rust Observatory and has not been migrated yet.</p></div></div>
        <div class="coming-soon"><div>◇</div><h2>Awaiting instrumentation</h2><p>Navigation has been migrated. Next we will port this view using the existing Rust data contracts, not substitute invented measurements.</p><button class="chrome-button" onclick={() => activePage = 'THE MACHINE'}>← RETURN TO THE MACHINE</button></div>
      {/if}
      {/key}
    </main>

    <aside class="inspector" class:with-vein={activePage !== 'FLEET' && activePage !== 'GPU' && !activePage.startsWith('NPU ') && selected !== 'NPU'}>
      {#if activePage === 'FLEET'}
        <div class="inspector-heading"><span class="eyebrow">FLEET / ENDPOINT</span><h2>{focusedDevice?.name ?? 'NO SELECTION'}</h2><p>SELECT AN ENDPOINT IN THE LIST OR MAP</p></div>
        {#if focusedDevice}
          <div class="inspector-data"><span>CONNECTION STATUS</span><strong class:inspector-online={focusedDevice.status === 'online'}>{focusedDevice.status.toUpperCase()}</strong>
            <small>OFFICE · {demoSites.find((site: FleetSite) => site.id === focusedDevice.siteId)?.name ?? 'UNASSIGNED'}</small></div>
          <div class="fleet-inspector-detail">
            <div><span>ROLE</span><strong>{focusedDevice.role}</strong></div>
            <div><span>OS</span><strong>{focusedDevice.os}</strong></div>
            <div><span>CPU</span><strong>{focusedDevice.status === 'offline' ? 'N/A' : `${formatNumber(focusedDevice.cpu)}%`}</strong></div>
            <div><span>MEMORY</span><strong>{focusedDevice.status === 'offline' ? 'N/A' : `${formatNumber(focusedDevice.memory)}%`}</strong></div>
            <div><span>AGENT</span><strong>V{focusedDevice.agentVersion}</strong></div>
            <div><span>LAST SEEN</span><strong>{focusedDevice.lastSeen}</strong></div>
          </div>
        {/if}
        <div class="inspector-provenance"><span class="truth-tag">{source === 'demo' ? '◇ DEMO ENDPOINT' : '● HUB ENDPOINT'}</span><p>{source === 'demo' ? 'Synthetic preview devices and coordinates.' : 'Fleet selection controls every hardware page. Office locations are not yet configured.'}</p></div>
      {:else if activePage.startsWith('NPU ') && (source === 'demo' || hasFrame)}
        <div class="inspector-heading"><span class="eyebrow">NPU / SELECTED MACHINE</span><h2>{frame.npu.name}</h2><p>{focusedDevice?.name ?? 'Selected machine'}</p></div>
        <div class="inspector-data"><span>NPU ACTIVITY</span><strong>{formatNumber(frame.npu.usage, 1)}%</strong><small>{frame.npu.powerState ?? 'Power state unavailable'}</small></div>
        <div class="inspector-provenance"><span class="truth-tag">{source === 'demo' ? '◇ SIMULATED' : '● AGENT TELEMETRY'}</span><p>Load, clocks and resident memory depend on driver support. Per-model inference attribution is not collected.</p></div>
      {:else if activePage === 'GPU' && (source === 'demo' || hasFrame)}
        <div class="inspector-heading"><span class="eyebrow">GPU / SELECTED MACHINE</span><h2>Graphics adapters</h2><p>Select an adapter in the GPU page to inspect its signals.</p></div>
        {#each frame.gpu.adapters as adapter (adapter.id)}
          <div class="inspector-data"><span>{adapter.name}</span><strong>{formatNumber(adapter.telemetry?.usage ?? NaN, 1)}%</strong><small>{formatNumber(adapter.telemetry?.temperatureC ?? NaN, 1)}°C · {adapter.integrated ? 'Integrated' : 'Graphics'}</small></div>
        {/each}
      {:else if source === 'demo' || hasFrame}
      <div class="inspector-heading"><span class="eyebrow">FOCUSED INSPECTION</span><h2>{focusName}</h2><p>{selected} · CLICK A NODE TO CHANGE FOCUS</p></div>
      <div class="inspector-data"><span>{inspected.title}</span><strong>{inspected.value}</strong><small>{inspected.detail}</small></div>
      <div class="inspector-provenance"><span class="truth-tag">{source === 'demo' ? '◇ SIMULATED' : '● OBSERVED'}</span><p>{source === 'demo' ? 'Generated locally; no live measurements.' : 'Agent measurements from the selected Fleet machine. N/A means unavailable.'}</p></div>
      {#if selected === 'NPU'}<p class="quiet">NPU device counters are available above. Per-model inference and instruction attribution are not collected.</p>{:else if source === 'demo'}<InstructionVein samples={veinBuffer} component={selected} globalPaused={!playing} />{:else}<p class="quiet">Live Instruction Vein awaits process targeting. No synthetic samples are mixed into live telemetry.</p>{/if}
      {/if}
    </aside>
  </div>

  <div class="timeline">
    <button class="control primary-control" onclick={() => playing = !playing}>{playing ? 'Ⅱ PAUSE' : '▶ RESUME'}</button>
    <span class="vertical-rule"></span>
    {#each (source === 'demo' ? [0.25, 1, 4] : []) as factor}
      <button class:chosen={speed === factor} class="control" onclick={() => speed = factor}>{factor}×</button>
    {/each}
    <span class="vertical-rule"></span>
    <span class="timer">{source === 'demo' ? `T+${elapsed.toFixed(2)}s` : hasFrame ? new Date(frame.capturedAt).toLocaleTimeString() : 'AWAITING SNAPSHOT'}</span>
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

<!-- WYNOBSERVE-UI-REPAIR-02 -->
