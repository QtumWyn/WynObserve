<script lang="ts">
  import { formatNumber } from '../data/bridge';
  import TrendChart from './TrendChart.svelte';
  import CpuIdentity from './CpuIdentity.svelte';
  import type { CpuCore, TelemetryFrame } from '../data/telemetry';

  interface Props {
    frame: TelemetryFrame;
    history: { time: number; value: TelemetryFrame }[];
  }
  let { frame, history }: Props = $props();
  let selectedCore = $state<number>(0);
  let detailOpen = $state<boolean>(false);
  let hoverIndex = $state<number | null>(null);
  let pinnedIndex = $state<number | null>(null);

  $effect(() => { if (!frame.cpu.cores.some(c => c.id === selectedCore)) selectedCore = frame.cpu.cores[0]?.id ?? 0; });
  let core = $derived(frame.cpu.cores.find((entry: CpuCore) => entry.id === selectedCore));
  let times = $derived(history.map((entry) => entry.time));
  let selectedCoreHistory = $derived(history.map((entry) => entry.value.cpu.cores.find((item) => item.id === selectedCore)?.usage ?? NaN));
  let packageHistory = $derived(history.map((entry) => entry.value.cpu.usage));
  let thermalHistory = $derived(history.map((entry) => entry.value.cpu.temperatureC));
  let busiest = $derived([...frame.cpu.cores].sort((a, b) => b.usage - a.usage).slice(0, 4));
  let averageFrequency = $derived(frame.cpu.cores.length ?
    frame.cpu.cores.reduce((acc, entry) => acc + entry.frequencyMHz, 0) / frame.cpu.cores.length : NaN);
</script>

<section class="hw-chamber" aria-label="CPU scheduler chamber">
  <div class="hw-heading"><div><div class="eyebrow">LIVE / CPU / 004</div><h1>CPU <em>// SCHEDULER CHAMBER</em></h1>
    <p>Logical CPUs, active work, frequencies, and scheduler pressure. Click a core to inspect its signal.</p></div>
    <span class="truth-tag">{frame.source === 'demo' ? '◇ SYNTHETIC PREVIEW' : '● AGENT TELEMETRY'}</span></div>

  <CpuIdentity cpu={frame.cpu} />

  <div class="hw-kpis">
    <article><span>PACKAGE LOAD</span><strong>{formatNumber(frame.cpu.usage, 1)}<small>%</small></strong><p>Across {frame.cpu.cores.length} logical CPUs</p></article>
    <article><span>CPU TEMPERATURE</span><strong>{formatNumber(frame.cpu.temperatureC, 1)}<small>°C</small></strong><p>{frame.cpu.temperatureSource ?? 'No supported CPU sensor reported by this Agent'}</p></article>
    <article><span>CONTEXT SWITCHES</span><strong>{formatNumber(frame.cpu.contextSwitches, 0)}<small>/s</small></strong><p>Agent scheduler counter; N/A when unavailable</p></article>
    <article><span>AVERAGE CLOCK</span><strong>{formatNumber(averageFrequency, 0)}<small>MHz</small></strong><p>Reported per-core frequencies</p></article>
  </div>

  <div class="hw-panel-head"><div><span class="eyebrow">LOGICAL TOPOLOGY</span><h2>Scheduler constellation</h2></div><span>{frame.cpu.cores.length} THREADS · SELECT TO INSPECT</span></div>
  <div class="hw-core-grid">
    {#each frame.cpu.cores as item (item.id)}
      <button class="hw-core" class:chosen={selectedCore === item.id} onclick={() => { selectedCore = item.id; hoverIndex = null; pinnedIndex = null; }} aria-pressed={selectedCore === item.id}
        aria-label={`Inspect CPU ${item.id}, ${formatNumber(item.usage, 1)} percent load`}>
        <span class="hw-core-head"><b>CPU {String(item.id).padStart(2, '0')}</b><strong>{formatNumber(item.usage, 0)}%</strong></span>
        <span class="hw-bar"><i style={`width:${Math.max(0, Math.min(100, item.usage))}%`}></i></span>
        <small>{formatNumber(item.frequencyMHz, 0)} MHz</small>
      </button>
    {/each}
  </div>

  <div class="hw-panel-head"><div><span class="eyebrow">SELECTED LOGICAL CPU / {String(selectedCore).padStart(2, '0')}</span><h2>Signal inspection</h2></div>
    <button class="hw-text-button" onclick={() => pinnedIndex = null}>CLEAR PIN</button></div>
  <div class="hw-selected">
    <div class="hw-selected-big"><span>CPU {String(selectedCore).padStart(2, '0')}</span><strong>{core ? `${formatNumber(core.usage, 1)}%` : 'N/A'}</strong><small>{core ? `${formatNumber(core.frequencyMHz, 0)} MHz` : 'NOT REPORTED'}</small></div>
    <div class="hw-selected-detail"><span>PROCESS / THREAD OWNERSHIP</span><strong>NOT AVAILABLE</strong><p>Per-core process ownership is not connected to this view.</p></div>
  </div>
  <div class="hw-charts">
    <TrendChart title={`CPU ${String(selectedCore).padStart(2, '0')} workload`} subtitle="LOGICAL / UTILIZATION" values={selectedCoreHistory} {times} tint="primary" unit="%" maximum={100} {hoverIndex} {pinnedIndex} onHover={(i) => hoverIndex = i} onPin={(i) => pinnedIndex = i} />
    <TrendChart title="Package workload" subtitle="CPU / AGGREGATE" values={packageHistory} {times} tint="info" unit="%" maximum={100} {hoverIndex} {pinnedIndex} onHover={(i) => hoverIndex = i} onPin={(i) => pinnedIndex = i} />
    <TrendChart title="CPU temperature" subtitle="CPU / THERMAL" values={thermalHistory} {times} tint="warning" unit="°C" maximum={105} {hoverIndex} {pinnedIndex} onHover={(i) => hoverIndex = i} onPin={(i) => pinnedIndex = i} />
  </div>
  <div class="hw-bottom-grid">
    <article class="hw-card"><span class="eyebrow">BUSIEST LOGICAL THREADS</span><h3>Hot lanes</h3>
      {#each busiest as hot (hot.id)}
        <div class="hw-simple-row"><span>CPU {String(hot.id).padStart(2, '0')}</span><b>{formatNumber(hot.usage, 1)}%</b></div>
      {/each}
    </article>
    <article class="hw-card"><span class="eyebrow">LEGACY FUNCTIONALITY</span><h3>Scheduler diagnostics</h3>
      <div class="hw-simple-row"><span>Migration rate</span><b>N/A</b></div>
      <div class="hw-simple-row"><span>Per-core thermal</span><b>N/A</b></div>
      <button class="hw-text-button" aria-expanded={detailOpen} onclick={() => detailOpen = !detailOpen}>{detailOpen ? 'HIDE DETAILS −' : 'DESCEND / FIELD COVERAGE +'}</button>
      {#if detailOpen}<p class="hw-note">The legacy Rust CPU page also provides per-core process, temperature, migrations and descendant affinity/cache/IRQ hooks. Those fields are intentionally not fabricated here. These fields need additional collector and request integration.</p>{/if}
    </article>
  </div>
  <p class="hw-provenance">{frame.source === 'demo' ? '◇ Synthetic demonstration data.' : '● Agent telemetry. N/A denotes an unavailable measurement; diagrams remain schematic.'}</p>
</section>
