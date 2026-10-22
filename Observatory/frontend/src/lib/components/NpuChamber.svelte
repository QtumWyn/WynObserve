<script lang="ts">
  import { formatNumber } from '../data/bridge';
  import type { TelemetryFrame } from '../data/telemetry';
  import TrendChart from './TrendChart.svelte';
  interface Props { frame: TelemetryFrame; history: { time: number; value: TelemetryFrame }[]; view: 'overview' | 'memory' | 'runtime' }
  let { frame, history, view }: Props = $props();
  let hoverIndex = $state<number | null>(null);
  let pinnedTime = $state<number | null>(null);
  let pinnedIndex = $derived.by(() => { const i = history.findIndex(h => h.time === pinnedTime); return i < 0 ? null : i; });
  const pin = (i: number | null) => pinnedTime = i === null ? null : history[i]?.time ?? null;
  const scale = (values: number[], floor: number) => Math.max(floor, ...values.filter(Number.isFinite)) * 1.1;
  let npu = $derived(frame.npu);
  let times = $derived(history.map(h => h.time));
  let loads = $derived(history.map(h => h.value.npu.usage));
  let memory = $derived(history.map(h => h.value.npu.residentMiB));
  let clocks = $derived(history.map(h => h.value.npu.frequencyMHz));
</script>
<section class="hw-chamber" aria-label={`NPU ${view}`}>
  <div class="hw-heading"><div><div class="eyebrow">NPU / {view.toUpperCase()}</div><h1>NPU <em>// {view.toUpperCase()}</em></h1>
    <p>{view === 'memory' ? 'Driver-reported resident memory on the selected machine.' : view === 'runtime' ? 'Device activity, clocks and power state.' : 'Neural accelerator identity and live device signals.'}</p></div>
    <span class="truth-tag">{frame.source === 'demo' ? '◇ SYNTHETIC PREVIEW' : '● AGENT TELEMETRY'}</span></div>
  <article class="hw-card hw-gpu-identity"><span class="eyebrow">NEURAL PROCESSING UNIT</span><h2>{npu.name}</h2>
    <p>{npu.available ? `PCI address: ${npu.pciAddress ?? 'not reported by this driver'}` : 'No supported NPU was reported by this machine’s Agent.'}</p>
    {#if !npu.available}<p class="hw-note">CPU and GPU inference does not imply that a dedicated NPU is present.</p>{/if}
  </article>
  <div class="hw-kpis">
    <article><span>NPU ACTIVITY</span><strong>{formatNumber(npu.usage, 1)}<small>%</small></strong><p>Device busy time</p></article>
    <article><span>CURRENT CLOCK</span><strong>{formatNumber(npu.frequencyMHz, 0)}<small>MHz</small></strong><p>Driver-reported frequency</p></article>
    <article><span>RESIDENT MEMORY</span><strong>{formatNumber(npu.residentMiB, 2)}<small>MiB</small></strong><p>Resident device allocations</p></article>
    <article><span>POWER STATE</span><strong class="npu-power">{npu.powerState ?? 'N/A'}</strong><p>Driver-reported state</p></article>
  </div>
  <div class="hw-panel-head"><div><span class="eyebrow">{view === 'memory' ? 'RESIDENT ALLOCATIONS' : view === 'runtime' ? 'RUNTIME SIGNALS' : 'DEVICE HISTORY'}</span><h2>{view === 'memory' ? 'Memory residency' : view === 'runtime' ? 'Activity and clocks' : 'Activity and residency'}</h2></div><button class="hw-text-button" onclick={() => pinnedTime = null}>CLEAR PIN</button></div>
  <div class="hw-charts">
    {#if view !== 'memory'}<TrendChart title="NPU activity" subtitle="DEVICE / BUSY TIME" values={loads} {times} unit="%" maximum={100} tint="secondary" {hoverIndex} {pinnedIndex} onHover={i => hoverIndex = i} onPin={pin} />{/if}
    {#if view === 'runtime'}
      <TrendChart title="NPU clock" subtitle="DEVICE / FREQUENCY" values={clocks} {times} unit="MHz" maximum={scale([...clocks, npu.maxFrequencyMHz], 100)} tint="info" {hoverIndex} {pinnedIndex} onHover={i => hoverIndex = i} onPin={pin} />
    {:else}
      <TrendChart title="NPU resident memory" subtitle="DEVICE / RESIDENT" values={memory} {times} unit="MiB" maximum={scale(memory, 1)} tint="info" {hoverIndex} {pinnedIndex} onHover={i => hoverIndex = i} onPin={pin} />
    {/if}
  </div>
  {#if view === 'memory'}
    <article class="hw-card"><span class="eyebrow">MEMORY ACCOUNTING</span><h3>Resident allocations</h3>
      <p class="hw-note">This is memory currently resident for the NPU. Shared-memory limits, total capacity, per-process allocations and per-model tensor sizes are not reported by the current Agent, so no capacity percentage is calculated.</p>
      <div class="hw-simple-row"><span>Resident memory counter</span><b>{Number.isFinite(npu.residentMiB) ? 'AVAILABLE' : 'UNAVAILABLE'}</b></div></article>
  {:else if view === 'runtime'}
    <article class="hw-card"><span class="eyebrow">DEVICE RUNTIME</span><h3>Execution signals</h3>
      <div class="hw-simple-row"><span>Maximum clock</span><b>{formatNumber(npu.maxFrequencyMHz, 0)} MHz</b></div>
      <div class="hw-simple-row"><span>Power state</span><b>{npu.powerState ?? 'N/A'}</b></div>
      <p class="hw-note">These counters describe device activity. Model names, inference latency, queue depth and per-process attribution require a separate inference-runtime integration.</p></article>
  {:else}
    <article class="hw-card"><span class="eyebrow">DRIVER CAPABILITIES</span><h3>Available signals</h3>
      <div class="hw-simple-row"><span>Utilization</span><b>{Number.isFinite(npu.usage) ? 'AVAILABLE' : 'UNAVAILABLE'}</b></div>
      <div class="hw-simple-row"><span>Frequency</span><b>{Number.isFinite(npu.frequencyMHz) ? 'AVAILABLE' : 'UNAVAILABLE'}</b></div>
      <div class="hw-simple-row"><span>Resident memory</span><b>{Number.isFinite(npu.residentMiB) ? 'AVAILABLE' : 'UNAVAILABLE'}</b></div>
      <p class="hw-note">An idle reading is zero. A missing counter or initial rate sample is unavailable.</p></article>
  {/if}
  <p class="hw-provenance">{frame.source === 'demo' ? '◇ Synthetic demonstration data.' : '● Selected machine’s Agent telemetry. N/A and chart gaps denote unavailable measurements.'}</p>
</section>
<style>.npu-power { font-size: 1.2rem; overflow-wrap: anywhere; }</style>
