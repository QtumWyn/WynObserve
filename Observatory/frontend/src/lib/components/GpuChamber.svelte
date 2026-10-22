<script lang="ts">
  import { formatNumber, gpuReading } from '../data/bridge';
  import TrendChart from './TrendChart.svelte';
  import type { TelemetryFrame } from '../data/telemetry';

  interface Props {
    frame: TelemetryFrame;
    history: { time: number; value: TelemetryFrame }[];
  }
  let { frame, history }: Props = $props();
  let hoverIndex = $state<number | null>(null);
  let pinnedTime = $state<number | null>(null);
  let pinnedIndex = $derived.by(() => { const i = history.findIndex(h => h.time === pinnedTime); return i < 0 ? null : i; });
  const pin = (i: number | null) => pinnedTime = i === null ? null : history[i]?.time ?? null;
  let selectedId = $state<string | null>(null);
  let effectiveId = $derived(selectedId ?? frame.gpu.id ?? frame.gpu.adapters[0]?.id ?? null);
  let gpu = $derived(gpuReading(frame, effectiveId) ?? { name: 'Selected GPU unavailable', usage: NaN, temperatureC: NaN, temperatureSource: null, vramGiB: NaN, powerWatts: NaN, clockMHz: NaN, source: 'Unavailable' });
  const select = (id: string) => { selectedId = id; hoverIndex = null; pinnedTime = null; };
  let expanded = $state<boolean>(false);
  const cells = Array.from({ length: 48 }, (_, i) => i);
  let illuminated = $derived(Number.isFinite(gpu.usage) ? Math.round(Math.max(0, Math.min(100, gpu.usage)) / 100 * cells.length) : null);
  let times = $derived(history.map((entry) => entry.time));
  let loadHistory = $derived(history.map((entry) => gpuReading(entry.value, effectiveId)?.usage ?? NaN));
  let thermalHistory = $derived(history.map((entry) => gpuReading(entry.value, effectiveId)?.temperatureC ?? NaN));
</script>

<section class="hw-chamber" aria-label="GPU compute nave">
  <div class="hw-heading"><div><div class="eyebrow">LIVE / GPU / 006</div><h1>GPU <em>// COMPUTE NAVE</em></h1>
    <p>Device utilization, thermal behavior, memory footprint, and transport instrumentation.</p></div>
    <span class="truth-tag">{frame.source === 'demo' ? '◇ SYNTHETIC PREVIEW' : '● AGENT TELEMETRY'}</span></div>

  <div class="hw-panel-head"><div><span class="eyebrow">DEVICE INVENTORY</span><h2>Graphics adapters</h2></div><span>{frame.gpu.adapters.length} REPORTED</span></div>
  {#if frame.gpu.adapters.length}
    <div class="hw-bottom-grid">
      {#each frame.gpu.adapters as adapter (adapter.id)}
        <article class="hw-card hw-gpu-identity" class:gpu-selected={adapter.id === effectiveId}>
          <span class="eyebrow">{adapter.integrated === true ? 'INTEGRATED GPU' : adapter.integrated === false ? 'DISCRETE GPU' : 'GRAPHICS ADAPTER'}</span>
          <h2>{adapter.name}</h2><p>{adapter.vendor} · {adapter.source}</p>
          <button class="hw-text-button" aria-pressed={adapter.id === effectiveId} onclick={() => select(adapter.id)}>{adapter.id === effectiveId ? 'SELECTED ADAPTER' : 'INSPECT ADAPTER →'}</button>
          <div class="hw-simple-row"><span>Activity / temperature</span><b>{formatNumber(adapter.telemetry?.usage ?? NaN, 1)}% / {formatNumber(adapter.telemetry?.temperatureC ?? NaN, 1)}°C</b></div>
          <div class="hw-simple-row"><span>Dedicated memory capacity</span><b>{formatNumber(adapter.dedicatedGiB, 2)} GiB</b></div>
          <div class="hw-simple-row"><span>Shared system memory limit</span><b>{formatNumber(adapter.sharedLimitGiB, 2)} GiB</b></div>
        </article>
      {/each}
    </div>
    <p class="hw-note">Memory capacities and limits describe each adapter, not current usage. Live measurements below belong to the named device.</p>
  {:else}
    <p class="hw-note">No graphics adapter inventory was reported by this Agent.</p>
  {/if}

  {#if frame.gpu.id && !frame.gpu.adapters.some(a => a.id === frame.gpu.id)}
    <button class="hw-text-button" onclick={() => select(frame.gpu.id!)}>INSPECT PRIMARY GPU · {frame.gpu.name}</button>
  {/if}
  <div class="hw-card hw-gpu-identity"><span class="eyebrow">MEASURED DEVICE</span><h2>{gpu.name}</h2><p>{gpu.source} · Measurements below belong to this adapter. Unsupported counters remain unavailable.</p></div>
  <div class="hw-kpis">
    <article><span>GPU ACTIVITY</span><strong>{formatNumber(gpu.usage, 1)}<small>%</small></strong><p>Measured device utilization</p></article>
    <article><span>THERMAL</span><strong>{formatNumber(gpu.temperatureC, 1)}<small>°C</small></strong><p>{gpu.temperatureSource ?? 'Temperature sensor unavailable'}</p></article>
    <article><span>VRAM FOOTPRINT</span><strong>{formatNumber(gpu.vramGiB, 2)}<small>GiB</small></strong><p>Used memory on the measured device</p></article>
    <article><span>DEVICE POWER</span><strong>{formatNumber(gpu.powerWatts, 1)}<small>W</small></strong><p>Driver-reported device power</p></article>
  </div>

  <div class="hw-panel-head"><div><span class="eyebrow">AGGREGATE COMPUTE / SCHEMATIC</span><h2>Activity lattice</h2></div><span>{illuminated === null ? 'UTILIZATION UNAVAILABLE' : `${illuminated} / ${cells.length} CELLS`}</span></div>
  <article class="hw-card">
    <div class="hw-gpu-grid" role="img" aria-label={`Schematic representation of ${formatNumber(gpu.usage, 0)} percent aggregate GPU utilization`}>
      {#each cells as cell (cell)}<span class:lit={illuminated !== null && cell < illuminated}></span>{/each}
    </div>
    <p class="hw-note">These 48 visual cells represent aggregate utilization, not physical streaming multiprocessors, CUDA cores, or per-unit measurements.</p>
  </article>

  <div class="hw-panel-head"><div><span class="eyebrow">COMPUTE / THERMAL HISTORY</span><h2>Device signals</h2></div>
    <button class="hw-text-button" onclick={() => pinnedTime = null}>CLEAR PIN</button></div>
  <div class="hw-charts">
    <TrendChart title="Compute activity" subtitle="GPU / UTILIZATION" values={loadHistory} {times} tint="secondary" unit="%" maximum={100} {hoverIndex} {pinnedIndex} onHover={(i) => hoverIndex = i} onPin={pin} />
    <TrendChart title="GPU temperature" subtitle="GPU / THERMAL" values={thermalHistory} {times} tint="warning" unit="°C" maximum={110} {hoverIndex} {pinnedIndex} onHover={(i) => hoverIndex = i} onPin={pin} />
  </div>
  <article class="hw-card"><span class="eyebrow">LEGACY GPU INSTRUMENTATION</span><h3>Clock and transport domains</h3>
    <div class="hw-simple-row"><span>Graphics clock</span><b>{formatNumber(gpu.clockMHz, 0)} MHz</b></div>
    <div class="hw-simple-row"><span>Performance state / fan</span><b>N/A</b></div>
    <div class="hw-simple-row"><span>PCIe RX / TX</span><b>N/A</b></div>
    <button class="hw-text-button" aria-expanded={expanded} onclick={() => expanded = !expanded}>{expanded ? 'HIDE MIGRATION NOTES −' : 'DESCEND / MIGRATION NOTES +'}</button>
    {#if expanded}<p class="hw-note">The original GPU view includes these NVML-derived measurements, a device identity and an aggregate compute grid. The remaining fields still need to be exposed in this view.</p>{/if}
  </article>
  <p class="hw-provenance">{frame.source === 'demo' ? '◇ Synthetic demonstration data.' : '● Agent telemetry. N/A denotes an unavailable measurement; diagrams remain schematic.'}</p>
</section>

<style>.gpu-selected { outline: 1px solid var(--accent-secondary, #b995eb); }</style>
