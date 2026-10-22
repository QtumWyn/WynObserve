<script lang="ts">
  import { tick } from 'svelte';
  import { memoryInventoryText } from '../data/motherboard';
  import type { HistoryPoint } from '../data/bridge';
  import { boardMetrics, type BoardMode } from '../data/boardEffects';
  import BoardSparkline from './BoardSparkline.svelte';
  import BoardPackage from './BoardPackage.svelte';
  import { formatNumber, gpuReading } from '../data/bridge';
  import { placePopover, type Bounds } from '../data/popover';
  import type { MachineNode } from '../data/machine';
  import type { TelemetryFrame } from '../data/telemetry';
  interface Props {
    id: string; anchor: HTMLButtonElement; node: MachineNode; frame: TelemetryFrame;
    history: HistoryPoint[]; mode: BoardMode; packageMembers: MachineNode[]; packageOpen: boolean;
    togglePackage: () => void; choosePackage: (node: MachineNode) => void;
    paused: boolean; dismiss: (restoreFocus?: boolean) => void;
  }
  let { id, anchor, node, frame, history, mode, packageMembers, packageOpen, togglePackage, choosePackage, paused, dismiss }: Props = $props();
  let metrics = $derived(boardMetrics(node));
  let left = $state(0), top = $state(0), width = $state(320), maxHeight = $state(400), positioned = $state(false);
  const value = (n: number, unit: string, digits = 1) => Number.isFinite(n) ? `${formatNumber(n, digits)} ${unit}` : 'N/A';
  let readings = $derived.by(() => {
    switch (node.kind) {
      case 'CPU': return [
        ['Activity', value(frame.cpu.usage, '%')], ['Temperature', value(frame.cpu.temperatureC, '°C')],
        ['Physical cores', frame.cpu.physicalCoreCount?.toString() ?? 'N/A'], ['Logical CPUs', String(frame.cpu.logicalCpuCount)],
      ];
      case 'MEMORY': return [
        ['Used', value(frame.memory.usedGiB, 'GiB', 2)], ['Capacity', value(frame.memory.totalGiB, 'GiB')],
        ['Pressure', value(frame.memory.usedPercent, '%')], ['Available', value(frame.memory.availableGiB, 'GiB', 2)],
      ];
      case 'GPU': {
        const gpu = gpuReading(frame, node.gpuId ?? null);
        return [['Activity', value(gpu?.usage ?? NaN, '%')], ['Temperature', value(gpu?.temperatureC ?? NaN, '°C')],
          ['Used GPU memory', value(gpu?.vramGiB ?? NaN, 'GiB', 2)], ['Graphics clock', value(gpu?.clockMHz ?? NaN, 'MHz', 0)]];
      }
      case 'NPU': return [
        ['Activity', value(frame.npu.usage, '%')], ['Resident memory', value(frame.npu.residentMiB, 'MiB', 2)],
        ['Clock', value(frame.npu.frequencyMHz, 'MHz', 0)], ['Power state', frame.npu.powerState ?? 'N/A'],
      ];
      case 'NVMe': return [['Read', value(frame.storage.readMiBs, 'MiB/s')], ['Write', value(frame.storage.writeMiBs, 'MiB/s')]];
      case 'NETWORK': return [['Receive', value(frame.network.rxMiBs, 'MiB/s')], ['Transmit', value(frame.network.txMiBs, 'MiB/s')]];
    }
  });

  // Intersect clipping ancestors so a scrolled/narrow board cannot strand the popup off-screen.
  function visibleBounds(target: HTMLButtonElement): Bounds {
    const bounds = { left: 0, top: 0, right: window.innerWidth, bottom: window.innerHeight };
    for (let element = target.parentElement; element; element = element.parentElement) {
      const style = getComputedStyle(element), rect = element.getBoundingClientRect();
      if (/(auto|scroll|hidden|clip)/.test(style.overflowX)) {
        bounds.left = Math.max(bounds.left, rect.left); bounds.right = Math.min(bounds.right, rect.right);
      }
      if (/(auto|scroll|hidden|clip)/.test(style.overflowY)) {
        bounds.top = Math.max(bounds.top, rect.top); bounds.bottom = Math.min(bounds.bottom, rect.bottom);
      }
    }
    return bounds;
  }
  function attachPopup(element: HTMLDivElement, _packageOpen: boolean) {
    // Snapshot the keyed trigger: reactive props can be cleared before focus events finish.
    const target = anchor;
    // Remain inside the themed app, but outside the board's scrolling/clipping layers.
    const host = target.closest('.observatory-app');
    host?.append(element);
    let alive = true, scheduled = 0;
    async function position() {
      scheduled = 0;
      if (!alive || !target.isConnected) return;
      const bounds = visibleBounds(target), rect = target.getBoundingClientRect();
      if (rect.bottom <= bounds.top || rect.top >= bounds.bottom || rect.right <= bounds.left || rect.left >= bounds.right) {
        dismiss(); return;
      }
      width = Math.max(1, Math.min(packageOpen ? 390 : 320, bounds.right - bounds.left - 16));
      maxHeight = Math.max(1, bounds.bottom - bounds.top - 16);
      await tick();
      if (!alive) return;
      const next = placePopover(rect, element.getBoundingClientRect(), bounds);
      left = next.left; top = next.top; positioned = true;
    }
    function schedule() { if (!scheduled) scheduled = requestAnimationFrame(() => { void position(); }); }
    function outside(event: PointerEvent) {
      const target = event.target;
      if (!(target instanceof Element) || element.contains(target) || target.closest('.hardware-button')) return;
      dismiss();
    }
    function keyboard(event: KeyboardEvent) {
      if (event.key === 'Escape') { event.preventDefault(); dismiss(true); }
      // A non-modal popup: Tab returns to the board's ordinary keyboard sequence.
      if (event.key === 'Tab' && element.contains(document.activeElement)) {
        const controls = [...element.querySelectorAll<HTMLButtonElement>('button:not(:disabled)')];
        if ((event.shiftKey && document.activeElement === controls[0]) || (!event.shiftKey && document.activeElement === controls.at(-1))) dismiss(true);
      }
    }
    function focusChanged(event: FocusEvent) {
      if (event.target instanceof Node && !element.contains(event.target) && event.target !== target) dismiss();
    }
    const board = target.closest('.motherboard-view');
    board?.addEventListener('boardviewchange', schedule);
    const observer = new ResizeObserver(schedule);
    observer.observe(element); observer.observe(target);
    document.addEventListener('pointerdown', outside, true);
    document.addEventListener('keydown', keyboard);
    document.addEventListener('focusin', focusChanged);
    window.addEventListener('scroll', schedule, true);
    window.addEventListener('resize', schedule);
    void position().then(() => { if (alive) element.querySelector<HTMLButtonElement>('button')?.focus({ preventScroll: true }); });
    return { update(_packageOpen: boolean) { schedule(); }, destroy() {
      alive = false; cancelAnimationFrame(scheduled); observer.disconnect();
      board?.removeEventListener('boardviewchange', schedule);
      document.removeEventListener('pointerdown', outside, true);
      document.removeEventListener('keydown', keyboard);
      document.removeEventListener('focusin', focusChanged);
      window.removeEventListener('scroll', schedule, true); window.removeEventListener('resize', schedule);
      element.remove();
    } };
  }
</script>

<div use:attachPopup={packageOpen} {id} class="board-popover" role="dialog" aria-modal="false" aria-labelledby={`${id}-title`} tabindex="-1"
  style:left={`${left}px`} style:top={`${top}px`} style:width={`${width}px`} style:max-height={`${maxHeight}px`} style:visibility={positioned ? 'visible' : 'hidden'}>
  <header><span>{node.role}</span><button aria-label="Close component details" onclick={() => dismiss(true)}>✕</button></header>
  <h3 id={`${id}-title`}>{node.name}</h3>
  <div class="reading-state"><i class:held={paused}></i>{frame.source === 'demo' ? 'DEMO READINGS' : 'AGENT READINGS'}{paused ? ' · HELD' : ''}</div>
  {#if node.kind === 'CPU' || packageOpen}
    <button class="package-toggle" aria-expanded={packageOpen} onclick={togglePackage}>{packageOpen ? '− Close package lens' : '◈ Explore CPU package'}</button>
  {/if}
  {#if packageOpen}<BoardPackage nodes={packageMembers} selectedId={node.id} choose={choosePackage} {mode}/>{/if}
  <dl>{#each readings as [label, reading]}<div><dt>{label}</dt><dd>{reading}</dd></div>{/each}</dl>
  <div class="mini-history">{#each metrics as metric}<BoardSparkline {history} {metric}/>{/each}</div>
  {#if node.kind === 'NPU' || node.integrated === true}<p>Enlarged accelerator block. Physical placement is illustrative.</p>{:else if node.kind === 'NVMe'}<p>Aggregate storage I/O; the M.2 shape is illustrative.</p>{:else if node.kind === 'MEMORY'}
    <p>{memoryInventoryText(frame.motherboard)}. Physical slot placement is matched only when firmware locators agree with the board profile.</p>
    {#if frame.motherboard?.memoryDevices.length}
      <div class="memory-inventory" aria-label="Firmware memory inventory">
        {#each frame.motherboard.memoryDevices as device, i}
          <div><span>{device.locator ?? `Memory ${i+1}`}{device.formFactor === 13 ? ' · SO-DIMM' : device.formFactor === 9 ? ' · DIMM' : ''}</span><b>{device.sizeBytes === null ? 'Unknown' : device.sizeBytes === 0 ? 'Empty' : value(device.sizeBytes/1024**3, 'GiB')}</b></div>
        {/each}
      </div>
    {/if}{/if}
  <footer>N/A = unavailable · Charts follow this component.</footer>
</div>

<style>
  .board-popover { position: fixed; z-index: 80; overflow: auto; padding: 16px; border: 1px solid color-mix(in srgb, var(--primary) 58%, var(--border)); border-radius: 10px; background: var(--panel); color: var(--text); box-shadow: 0 16px 48px #000b, 0 0 0 3px color-mix(in srgb, var(--primary) 7%, transparent); scrollbar-width: thin; scrollbar-color: var(--border) transparent; }
  .package-toggle { width: 100%; margin-top: 12px; padding: 8px; border: 1px solid var(--border); border-radius: 5px; background: var(--deep); color: var(--secondary); font: 10px 'JetBrains Mono',monospace; }
  .package-toggle:hover { border-color: var(--secondary); }
  .memory-inventory { margin: 10px 0; font-size: 10px; }
  .memory-inventory > div { display: flex; justify-content: space-between; gap: 8px; padding: 6px 0; border-bottom: 1px solid var(--border); }
  .memory-inventory span { color: var(--muted); overflow-wrap: anywhere; }
  .memory-inventory b { white-space: nowrap; color: var(--info); font-weight: 500; }
  .mini-history { display: grid; gap: 6px; margin-bottom: 12px; }
  button:focus-visible { outline: 2px solid var(--info); outline-offset: 2px; }
  header { display: flex; gap: 10px; justify-content: space-between; align-items: center; }
  header > span { font: 9px 'JetBrains Mono', monospace; letter-spacing: .08em; color: var(--primary); }
  header button { flex: 0 0 28px; width: 28px; height: 28px; border-radius: 5px; border: 1px solid var(--border); background: var(--deep); color: var(--muted); }
  header button:hover { color: var(--bright); border-color: var(--primary); }
  h3 { font-size: 16px; line-height: 1.4; overflow-wrap: anywhere; margin: 6px 0 10px; }
  .reading-state { display: flex; align-items: center; gap: 6px; font: 8px 'JetBrains Mono', monospace; letter-spacing: .06em; color: var(--muted); }
  .reading-state i { width: 5px; height: 5px; border-radius: 50%; background: var(--success); }
  .reading-state i.held { background: var(--warning); }
  dl { display: grid; grid-template-columns: repeat(2,minmax(0,1fr)); gap: 14px 10px; padding: 14px 0; margin: 10px 0 0; border-top: 1px solid var(--border); }
  dt { font-size: 10px; color: var(--muted); margin-bottom: 5px; }
  dd { font: 600 15px 'JetBrains Mono', monospace; color: var(--info); margin: 0; overflow-wrap: anywhere; }
  p, footer { font-size: 10px; line-height: 1.6; color: var(--muted); margin: 0; }
  footer { border-top: 1px solid var(--border); margin-top: 10px; padding-top: 10px; }
</style>
