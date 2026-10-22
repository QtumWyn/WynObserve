<script lang="ts">
  import { formatNumber } from '../data/bridge';
  import TrendChart from './TrendChart.svelte';
  import type { TelemetryFrame } from '../data/telemetry';

  interface Props {
    frame: TelemetryFrame;
    history: { time: number; value: TelemetryFrame }[];
    openProcesses: () => void;
  }
  let { frame, history, openProcesses }: Props = $props();
  let hoverIndex = $state<number | null>(null);
  let pinnedIndex = $state<number | null>(null);
  let revealLegacyFields = $state<boolean>(false);
  let availableGiB = $derived(frame.memory.availableGiB);
  let times = $derived(history.map((entry) => entry.time));
  let usageHistory = $derived(history.map((entry) => entry.value.memory.usedPercent));
  let faultHistory = $derived(history.map((entry) => entry.value.memory.faults));
  let topProcesses = $derived([...frame.processes].sort((a, b) => b.memoryGiB - a.memoryGiB));
</script>

<section class="hw-chamber" aria-label="Memory page cathedral">
  <div class="hw-heading"><div><div class="eyebrow">LIVE / MEMORY / 005</div><h1>MEMORY <em>// PAGE CATHEDRAL</em></h1>
    <p>Physical memory composition, pressure, fault activity, and process-memory pathways.</p></div>
    <span class="truth-tag">{frame.source === 'demo' ? '◇ SYNTHETIC PREVIEW' : '● AGENT TELEMETRY'}</span></div>

  <div class="hw-kpis">
    <article><span>PHYSICAL USED</span><strong>{formatNumber(frame.memory.usedGiB, 2)}<small>GiB</small></strong><p>{formatNumber(frame.memory.usedPercent, 1)}% of {formatNumber(frame.memory.totalGiB, 0)} GiB</p></article>
    <article><span>AVAILABLE</span><strong>{formatNumber(availableGiB, 2)}<small>GiB</small></strong><p>Reported available memory in live mode</p></article>
    <article><span>PAGE FAULT RATE</span><strong>{formatNumber(frame.memory.faults, 0)}<small>/s</small></strong><p>Agent fault rate; N/A when unavailable</p></article>
    <article><span>MEMORY CAPACITY</span><strong>{formatNumber(frame.memory.totalGiB, 0)}<small>GiB</small></strong><p>Selected data source</p></article>
  </div>

  <div class="hw-panel-head"><div><span class="eyebrow">GLOBAL MEMORY MAP / PHYSICAL MEMORY</span><h2>Composition</h2></div><span>PHYSICAL MEMORY</span></div>
  <article class="hw-card hw-memory-map">
    <div class="hw-memory-track" role="img" aria-label={`${formatNumber(frame.memory.usedPercent, 1)} percent used and ${formatNumber(availableGiB, 2)} GiB available`}>
      <div class="hw-memory-used" style={`width:${Math.max(0, Math.min(100, frame.memory.usedPercent))}%`}></div>
    </div>
    <div class="hw-memory-key"><span>◼ USED · {formatNumber(frame.memory.usedGiB, 2)} GiB</span><span>□ AVAILABLE · {formatNumber(availableGiB, 2)} GiB</span></div>
    <p class="hw-note">Live used and available memory come directly from the Agent. Cached, active, and dirty memory are not yet displayed.</p>
  </article>

  <div class="hw-panel-head"><div><span class="eyebrow">PHYSICAL / PAGE BEHAVIOR</span><h2>Pressure signals</h2></div>
    <button class="hw-text-button" onclick={() => pinnedIndex = null}>CLEAR PIN</button></div>
  <div class="hw-charts">
    <TrendChart title="Memory pressure" subtitle="PHYSICAL / USED" values={usageHistory} {times} tint="info" unit="%" maximum={100} {hoverIndex} {pinnedIndex} onHover={(i) => hoverIndex = i} onPin={(i) => pinnedIndex = i} />
    <TrendChart title="Page faults" subtitle="MEMORY / PER SECOND" values={faultHistory} {times} tint="warning" unit="/s" maximum={1500} {hoverIndex} {pinnedIndex} onHover={(i) => hoverIndex = i} onPin={(i) => pinnedIndex = i} />
  </div>
  <div class="hw-bottom-grid">
    <article class="hw-card"><span class="eyebrow">PROCESS RESIDENT MEMORY</span><h3>Resident footprints</h3>
      {#each topProcesses as process (process.pid)}
        <div class="hw-simple-row"><span>{process.name} <small>PID {process.pid}</small></span><b>{formatNumber(process.memoryGiB, 2)} GiB</b></div>
      {/each}
      <p class="hw-note">The process list is partial; shared pages can occur in multiple resident footprints.</p>
      <button class="hw-text-button" onclick={openProcesses}>OPEN PROCESSES →</button>
    </article>
    <article class="hw-card"><span class="eyebrow">ORIGINAL MEMORY TOOLS</span><h3>Address-space descent</h3>
      <p class="hw-note">The original MEMORY page inspects a selected process's address space through the Hub. The Svelte frontend does not yet have a Hub-linked memory-map request.</p>
      <div class="hw-simple-row"><span>PROCESS MAP</span><b>AWAITING BRIDGE</b></div>
      <button class="hw-text-button" aria-expanded={revealLegacyFields} onclick={() => revealLegacyFields = !revealLegacyFields}>{revealLegacyFields ? 'HIDE DETAILS −' : 'SHOW REMAINING FIELDS +'}</button>
      {#if revealLegacyFields}
        {#each ['Active bytes', 'Cached bytes', 'Dirty bytes', 'Region permissions', 'Mapped image breakdown'] as missing}
          <div class="hw-simple-row"><span>{missing}</span><b>N/A</b></div>
        {/each}
      {/if}
    </article>
  </div>
  <p class="hw-provenance">{frame.source === 'demo' ? '◇ Synthetic demonstration data.' : '● Agent telemetry. N/A denotes an unavailable measurement; diagrams remain schematic.'}</p>
</section>
