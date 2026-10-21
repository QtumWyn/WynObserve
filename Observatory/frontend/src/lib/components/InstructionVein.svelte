<script lang="ts">
  import { tick } from 'svelte';
  import type { Component, Instruction } from '../data/telemetry';

  interface Props {
    samples: Instruction[];
    component: Component;
    globalPaused: boolean;
  }

  let { samples, component, globalPaused }: Props = $props();

  // Local view state. Pausing the vein does NOT stop the underlying ring buffer.
  let frozen = $state<boolean>(false);
  let follow = $state<boolean>(true);
  let allComponents = $state<boolean>(false);
  let cpuFilter = $state<string>('all');
  let visible = $state<Instruction[]>([]);
  let selectedSequence = $state<number | null>(null);
  let streamElement = $state<HTMLDivElement | null>(null);

  $effect((): void => {
    if (!frozen) {
      visible = samples;
    }
  });

  // Keep the complete 256-entry ring buffer accessible, not just the newest 100.
  let cpuChoices = $derived(
    [...new Set(visible.map((sample: Instruction): number => sample.cpu))]
      .sort((a: number, b: number): number => a - b),
  );
  let displayed = $derived(
    visible.filter((sample: Instruction): boolean =>
      (allComponents || sample.component === component)
      && (cpuFilter === 'all' || String(sample.cpu) === cpuFilter),
    ),
  );
  let inspected = $derived(
    visible.find((sample: Instruction) => sample.sequence === selectedSequence) ?? null,
  );
  let newestVisibleSequence = $derived(displayed[0]?.sequence ?? null);

  // At the top we follow new arrivals. Otherwise do not steal the user's scroll
  // position while they read an older instruction.
  $effect((): void => {
    const sequence = newestVisibleSequence;
    if (sequence !== null && follow && !frozen) {
      void tick().then((): void => {
        if (streamElement && follow && !frozen) {
          streamElement.scrollTop = 0;
        }
      });
    }
  });

  function onScroll(): void {
    if (streamElement && !frozen) {
      follow = streamElement.scrollTop < 12;
    }
  }

  function setFollow(value: boolean): void {
    follow = value;
    if (value && streamElement) {
      streamElement.scrollTop = 0;
    }
  }

  function inspect(sample: Instruction): void {
    frozen = true;
    selectedSequence = sample.sequence;
  }

  function resume(): void {
    selectedSequence = null;
    frozen = false;
    setFollow(true);
  }

  function toggleHold(): void {
    if (frozen) {
      resume();
    } else {
      frozen = true;
    }
  }
</script>

<div class="vein2">
  <div class="vein2-heading">
    <div>
      <span class="eyebrow">INSTRUCTION VEIN / 02</span>
      <h2>Execution samples</h2>
    </div>
    <span class:holding={frozen || globalPaused} class="vein2-beacon" title="Synthetic instruction feed">◉</span>
  </div>

  <div class="vein2-overview">
    <span class="vein2-demo">◇ MOCK</span>
    <span>{frozen ? 'INSPECTION HOLD' : globalPaused ? 'TIMELINE PAUSED' : 'SIMULATING'}</span>
    <span>{displayed.length} shown · {visible.length} / 256 buffered</span>
  </div>

  <p class="vein2-context">
    {#if component === 'GPU'}
      Host x86-64 activity correlated with GPU work. Not NVIDIA SASS.
    {:else}
      Synthetic x86-64 samples classified near {component} activity.
    {/if}
  </p>

  <div class="vein2-controls" aria-label="Instruction feed controls">
    <button class:chosen={!allComponents} onclick={() => allComponents = false}>FOCUS</button>
    <button class:chosen={allComponents} onclick={() => allComponents = true}>ALL</button>
    <label class="vein2-cpu-picker">
      <span>CORE</span>
      <select bind:value={cpuFilter} aria-label="Filter instruction samples by CPU core">
        <option value="all">ALL</option>
        {#each cpuChoices as cpu (cpu)}
          <option value={String(cpu)}>CPU {String(cpu).padStart(2, '0')}</option>
        {/each}
      </select>
    </label>
    <button class:chosen={follow} onclick={() => setFollow(!follow)} disabled={frozen} title="Follow newest instructions">
      {follow ? '↑ FOLLOW' : '↑ LATEST'}
    </button>
    <button class:holding={frozen} onclick={toggleHold}>
      {frozen ? '▶ RESUME' : 'Ⅱ HOLD'}
    </button>
  </div>

  <div class="vein2-stream" bind:this={streamElement} onscroll={onScroll} role="region" aria-label="Instruction samples" aria-live="off" tabindex="0">
    {#if displayed.length === 0}
      <div class="vein2-empty">No samples in this channel yet.</div>
    {:else}
      {#each displayed as sample (sample.sequence)}
        <button
          class="vein2-row"
          data-lane={sample.cpu % 5}
          class:inspected={sample.sequence === selectedSequence}
          aria-pressed={sample.sequence === selectedSequence}
          aria-label={`Inspect sample ${sample.sequence} from ${sample.process} on CPU ${sample.cpu}`}
          onclick={() => inspect(sample)}
        >
          <span class="vein2-row-head">
            <span class="vein2-seq">#{sample.sequence}</span>
            <span class="vein2-cpu">CPU {sample.cpu.toString().padStart(2, '0')}</span>
          </span>
          <span class="vein2-asm"><b>{sample.mnemonic}</b> {sample.operands}</span>
          <span class="vein2-row-foot">
            <span class="vein2-process">{sample.process}</span>
            <span class="vein2-truth">◇ {sample.origin} · {sample.truth}</span>
          </span>
        </button>
      {/each}
    {/if}
  </div>

  <div class="vein2-caption">
    <span>{allComponents ? 'ALL CHANNELS' : `${component} FOCUS`}</span>
    <span>CLICK TO INSPECT</span>
  </div>

  {#if inspected}
    <section class="vein2-detail" aria-label="Selected instruction details">
      <header class="vein2-detail-head">
        <div><span class="eyebrow">SAMPLE INSPECTOR</span><strong>#{inspected.sequence}</strong></div>
        <button onclick={() => selectedSequence = null} aria-label="Close instruction details">✕</button>
      </header>
      <div class="vein2-detail-body">
        <div><span>ORIGIN</span><strong>{inspected.origin} / {inspected.truth}</strong></div>
        <div><span>PROCESS</span><strong>{inspected.process}</strong></div>
        <div><span>PID / TID</span><strong>{inspected.pid} / {inspected.tid}</strong></div>
        <div><span>CPU</span><strong>{inspected.cpu.toString().padStart(2, '0')}</strong></div>
        <div><span>ADDRESS</span><code>{inspected.address}</code></div>
        <div><span>BYTES</span><code>{inspected.bytes}</code></div>
        <div class="vein2-detail-asm"><span>ASSEMBLY</span><code>{inspected.mnemonic} {inspected.operands}</code></div>
        <p>{inspected.note}</p>
      </div>
      <button class="vein2-resume" onclick={resume}>▶ RESUME LIVE PREVIEW</button>
    </section>
  {/if}
</div>
