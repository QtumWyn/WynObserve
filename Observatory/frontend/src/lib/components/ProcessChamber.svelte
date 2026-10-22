<script lang="ts">
  import { formatNumber } from '../data/bridge';
  import type { TelemetryFrame } from '../data/telemetry';

  interface Props {
    frame: TelemetryFrame;
    openMemory: () => void;
  }
  let { frame, openMemory }: Props = $props();
  type Process = TelemetryFrame['processes'][number];
  type SortKey = 'cpu' | 'memoryGiB' | 'pid' | 'name';
  let query = $state<string>('');
  let sort = $state<SortKey>('cpu');
  let selectedPid = $state<number | null>(null);
  let expanded = $state<boolean>(false);
  let matching = $derived([...frame.processes]
    .filter((process: Process) => `${process.name} ${process.pid}`.toLowerCase().includes(query.trim().toLowerCase()))
    .sort((a: Process, b: Process) => sort === 'name' ? a.name.localeCompare(b.name) :
      sort === 'pid' ? a.pid - b.pid : sort === 'cpu' ? b.cpu - a.cpu : b.memoryGiB - a.memoryGiB));
  let inspected = $derived(frame.processes.find((process: Process) => process.pid === selectedPid));
  let totalCpu = $derived(frame.processes.reduce((sum: number, process: Process) => sum + process.cpu, 0));
</script>

<section class="hw-chamber" aria-label="Process workload">
  <div class="hw-heading"><div><div class="eyebrow">LIVE / PROCESSES / 007</div><h1>PROCESSES <em>// LIVING WORKLOAD</em></h1>
    <p>Explore and inspect exported workloads. Processes follow the selected data source.</p></div>
    <span class="truth-tag">{frame.source === 'demo' ? '◇ SYNTHETIC PREVIEW' : '● AGENT TELEMETRY'}</span></div>

  <div class="hw-kpis">
    <article><span>EXPORTED RECORDS</span><strong>{frame.processes.length}</strong><p>Records exported by the selected data source</p></article>
    <article><span>SUMMED LOAD</span><strong>{formatNumber(totalCpu, 1)}<small>%</small></strong><p>Sum of per-process CPU rates; can exceed 100% across cores</p></article>
    <article><span>THREAD COUNT</span><strong>N/A</strong><p>Not present in the Svelte data model</p></article>
    <article><span>PROCESS STATE</span><strong>N/A</strong><p>Running/sleeping groups require real state values</p></article>
  </div>

  <div class="hw-panel-head"><div><span class="eyebrow">WORKLOAD REGISTER</span><h2>Process ledger</h2></div><span>{matching.length} MATCHES</span></div>
  <div class="hw-process-tools">
    <label>FILTER BY NAME / PID<input type="search" bind:value={query} placeholder="Search processes…" aria-label="Filter process list by name or PID" /></label>
    <label>SORT BY<select bind:value={sort} aria-label="Sort processes">
      <option value="cpu">CPU (high to low)</option><option value="memoryGiB">MEMORY (high to low)</option><option value="name">NAME</option><option value="pid">PID</option>
    </select></label>
  </div>
  <div class="hw-process-table" role="table" aria-label="Selected machine processes">
    <div class="hw-process-header" role="row"><span role="columnheader">PROCESS</span><span role="columnheader">PID</span><span role="columnheader">CPU</span><span role="columnheader">RSS</span><span role="columnheader">ACTION</span></div>
    {#each matching as process (process.pid)}
      <div class="hw-process-row" role="row" class:chosen={selectedPid === process.pid}>
        <span role="cell" class="hw-process-name">{process.name}</span><span role="cell">{process.pid}</span><span role="cell">{formatNumber(process.cpu, 1)}%</span><span role="cell">{formatNumber(process.memoryGiB, 2)} GiB</span>
        <span role="cell"><button aria-pressed={selectedPid === process.pid} onclick={() => selectedPid = process.pid}>INSPECT</button></span>
      </div>
    {:else}
      <p class="hw-note">No processes match that search.</p>
    {/each}
  </div>
  {#if inspected}
    <article class="hw-card hw-process-inspection"><span class="eyebrow">SELECTED WORKLOAD</span><h3>{inspected.name} // PID {inspected.pid}</h3>
      <div class="hw-simple-row"><span>CPU</span><b>{formatNumber(inspected.cpu, 2)}%</b></div>
      <div class="hw-simple-row"><span>RESIDENT MEMORY</span><b>{formatNumber(inspected.memoryGiB, 3)} GiB</b></div>
      <div class="hw-simple-row"><span>THREADS / STATE / LAST CPU / I/O</span><b>N/A</b></div>
      <div class="hw-simple-row"><span>ADDRESS SPACE MAP</span><b>BACKEND NOT CONNECTED</b></div>
      <button class="hw-text-button" onclick={openMemory}>VIEW SYSTEM MEMORY →</button>
      <p class="hw-note">The original MAP action uses PID and expected process start time for safe Hub requests. Those values and that request path are not available in this prototype, so no simulated MAP request is made.</p>
    </article>
  {/if}
  <article class="hw-card"><span class="eyebrow">RUST/EGUI FEATURE PRESERVATION</span><h3>Remaining process instrumentation</h3>
    <button class="hw-text-button" aria-expanded={expanded} onclick={() => expanded = !expanded}>{expanded ? 'HIDE DETAILS −' : 'SHOW LEGACY FIELDS +'}</button>
    {#if expanded}<p class="hw-note">The original implementation also has running/sleeping tabs, parent PID, last scheduled CPU, thread count, state, executable hover, per-process I/O, start-time validation and live memory-map action. All remain available in the original egui app while their Agent/Hub contracts are migrated.</p>{/if}
  </article>
  <p class="hw-provenance">{frame.source === 'demo' ? '◇ Synthetic demonstration data.' : '● Agent telemetry. N/A denotes an unavailable measurement; diagrams remain schematic.'}</p>
</section>
