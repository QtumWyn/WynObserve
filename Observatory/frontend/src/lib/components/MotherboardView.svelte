<script lang="ts">
  import type { HistoryPoint } from '../data/bridge';
  import { boardSignals, partStyle, thermalColor, packageNodes, fittedSize, clampCamera, zoomCamera, type BoardMode, type BoardCamera } from '../data/boardEffects';
  import type { MachineNode } from '../data/machine';
  import type { TelemetryFrame } from '../data/telemetry';
  import { machineNodes, motherboardLayout, focusId, intensity, type MachineComponent } from '../data/machine';
  import BoardHardware from './BoardHardware.svelte';
  import BoardSurface from './BoardSurface.svelte';
  import BoardPopover from './BoardPopover.svelte';
  interface Props {
    frame: TelemetryFrame; history: HistoryPoint[]; selected: MachineComponent; selectedGpuId: string | null;
    choose: (component: MachineComponent, gpuId?: string) => void; paused?: boolean;
  }
  let { frame, history, selected, selectedGpuId, choose, paused = false }: Props = $props();
  const uid = $props.id();
  let board = $derived(motherboardLayout(machineNodes(frame), frame.motherboard));
  let selectedId = $derived(focusId(selected, selectedGpuId));
  let opened = $state<{ id: string; anchor: HTMLButtonElement; packageOpen: boolean } | null>(null);
  let mode = $state<BoardMode>('activity');
  let hovered = $state<string | null>(null), focused = $state<string | null>(null);
  let boardElement: HTMLElement;
  let viewport: HTMLDivElement;
  let viewWidth = $state(700), viewHeight = $state(600);
  let camera = $state<BoardCamera>({ zoom: 1, x: 0, y: 0 });
  let dragging = $state(false);
  let view = $derived({ width: viewWidth, height: viewHeight });
  let fitted = $derived(fittedSize(view, board.width / board.height));
  let packageMembers = $derived(packageNodes(board.nodes, frame));
  let highlight = $derived(hovered ?? focused ?? selectedId);
  $effect(() => {
    const next = clampCamera(camera, view, fitted);
    if (next.x !== camera.x || next.y !== camera.y || next.zoom !== camera.zoom) camera = next;
    boardElement?.dispatchEvent(new Event('boardviewchange'));
  });
  function zoomTo(zoom: number, point = { x: 0, y: 0 }) { camera = zoomCamera(camera, zoom, point, view, fitted); }
  function fit() { camera = { zoom: 1, x: 0, y: 0 }; }
  function focusPart(event: FocusEvent, id: string) {
    focused = id;
    const target = event.currentTarget as HTMLButtonElement;
    const rect = target.getBoundingClientRect(), bounds = viewport.getBoundingClientRect();
    const dx = rect.left < bounds.left + 16 ? bounds.left + 16 - rect.left : rect.right > bounds.right - 16 ? bounds.right - 16 - rect.right : 0;
    const dy = rect.top < bounds.top + 16 ? bounds.top + 16 - rect.top : rect.bottom > bounds.bottom - 16 ? bounds.bottom - 16 - rect.bottom : 0;
    if (dx || dy) camera = clampCamera({ ...camera, x: camera.x + dx, y: camera.y + dy }, view, fitted);
  }
  function cameraKeys(event: KeyboardEvent) {
    if (event.target !== event.currentTarget) return;
    if (['+', '=', '-', '0', 'ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(event.key)) event.preventDefault();
    if (event.key === '+' || event.key === '=') zoomTo(camera.zoom * 1.25);
    else if (event.key === '-') zoomTo(camera.zoom / 1.25);
    else if (event.key === '0') fit();
    else {
      const delta = event.shiftKey ? 100 : 40;
      const x = event.key === 'ArrowLeft' ? delta : event.key === 'ArrowRight' ? -delta : 0;
      const y = event.key === 'ArrowUp' ? delta : event.key === 'ArrowDown' ? -delta : 0;
      if (x || y) camera = clampCamera({ ...camera, x: camera.x + x, y: camera.y + y }, view, fitted);
    }
  }
  function gestures(element: HTMLDivElement) {
    const pointers = new Map<number, { x: number; y: number }>();
    let last: { x: number; y: number; distance: number } | null = null;
    let moved = false, travel = 0;
    function position() {
      const points = [...pointers.values()], a = points[0], b = points[1] ?? a;
      return a ? { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2, distance: Math.hypot(a.x - b.x, a.y - b.y) } : null;
    }
    function down(event: PointerEvent) {
      if (!pointers.size) { moved = false; travel = 0; }
      if (event.button !== 0 || (event.target instanceof Element && event.target.closest('button') && event.pointerType !== 'touch')) return;
      pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
      if (pointers.size === 1) moved = false;
      last = position();
      if (!(event.target instanceof Element && event.target.closest('button'))) element.focus({ preventScroll: true });
      if (!(event.target instanceof Element && event.target.closest('button'))) element.setPointerCapture(event.pointerId);
    }
    function move(event: PointerEvent) {
      if (!pointers.has(event.pointerId)) return;
      pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
      const next = position();
      if (last && next && (camera.zoom > 1 || pointers.size > 1)) {
        const bounds = element.getBoundingClientRect();
        let current = camera;
        if (last.distance > 0 && next.distance > 0) current = zoomCamera(current, current.zoom * next.distance / last.distance, { x: last.x - bounds.left - view.width / 2, y: last.y - bounds.top - view.height / 2 }, view, fitted);
        const dx = next.x - last.x, dy = next.y - last.y;
        travel += Math.hypot(dx, dy);
        if (travel > 4 || next.distance !== last.distance) { moved = true; dragging = true; element.setPointerCapture(event.pointerId); }
        camera = clampCamera({ ...current, x: current.x + dx, y: current.y + dy }, view, fitted);
      }
      last = next;
    }
    function up(event: PointerEvent) {
      pointers.delete(event.pointerId); last = position();
      if (!pointers.size) dragging = false;
    }
    function lost(event: PointerEvent) { if (event.target === element) up(event); }
    function click(event: MouseEvent) { if (moved) { event.preventDefault(); event.stopPropagation(); moved = false; } }
    function wheel(event: WheelEvent) {
      if (!event.ctrlKey && !event.metaKey) return;
      event.preventDefault();
      const rect = element.getBoundingClientRect();
      zoomTo(camera.zoom * Math.exp(-event.deltaY * .006), { x: event.clientX - rect.left - view.width / 2, y: event.clientY - rect.top - view.height / 2 });
    }
    element.addEventListener('pointerdown', down); element.addEventListener('pointermove', move);
    element.addEventListener('pointerup', up); element.addEventListener('pointercancel', up);
    element.addEventListener('lostpointercapture', lost); element.addEventListener('click', click, true);
    element.addEventListener('wheel', wheel, { passive: false });
    return { destroy() {
      element.removeEventListener('pointerdown', down); element.removeEventListener('pointermove', move);
      element.removeEventListener('pointerup', up); element.removeEventListener('pointercancel', up);
      element.removeEventListener('lostpointercapture', lost); element.removeEventListener('click', click, true);
      element.removeEventListener('wheel', wheel);
    } };
  }
  function togglePackage() {
    if (!opened) return;
    choose('CPU');
    opened = { ...opened, id: 'CPU', packageOpen: !opened.packageOpen };
  }
  function choosePackage(node: MachineNode) {
    if (!opened) return;
    choose(node.kind, node.gpuId);
    opened = { ...opened, id: node.id, packageOpen: true };
  }
  let openNode = $derived(board.nodes.find(n => n.id === opened?.id));
  $effect(() => { if (opened && (!openNode || opened.id !== selectedId)) opened = null; });
  function inspect(node: typeof board.nodes[number], anchor: HTMLButtonElement) {
    choose(node.kind, node.gpuId);
    opened = opened?.id === node.id ? null : { id: node.id, anchor, packageOpen: false };
  }
  function dismiss(restoreFocus = false) {
    const anchor = opened?.anchor;
    if (restoreFocus && anchor?.isConnected) anchor.focus({ preventScroll: true });
    opened = null;
  }
  function trace(node: typeof board.nodes[number]) {
    const cpu = board.nodes.find(n => n.kind === 'CPU')!;
    const cx = cpu.x + cpu.width / 2, cy = cpu.y + cpu.height / 2;
    const nx = node.x + node.width / 2, ny = node.y + node.height / 2;
    if (node.detached) return `M${cpu.x+cpu.width} ${cy}H600V92H${nx}V${node.y}`;
    if (node.kind === 'MEMORY') return `M${cpu.x+cpu.width} ${cy}H600V${ny}H${node.x}`;
    if (node.kind === 'NETWORK') return `M${cpu.x} ${cy}H160V${ny}H${node.x+node.width}`;
    return `M${cx} ${cpu.y+cpu.height}V${node.y-18}H${nx}V${node.y}`;
  }

</script>

<section bind:this={boardElement} class="motherboard-view" aria-label="Illustrative motherboard">
  <div class="board-caption"><span>◈ BOARD EXPLORER</span><span>{frame.source === 'demo' ? 'DEMO' : 'AGENT TELEMETRY'} · {board.nodes.length} COMPONENTS</span></div>
  <div class="board-toolbar">
    <div class="mode-switch" role="group" aria-label="Board coloring">
      <button class:chosen={mode === 'activity'} aria-pressed={mode === 'activity'} onclick={() => mode = 'activity'}>◉ Activity</button>
      <button class:chosen={mode === 'thermal'} aria-pressed={mode === 'thermal'} onclick={() => mode = 'thermal'}>♨ Thermal</button>
    </div>
    <div class="zoom-controls" role="group" aria-label="Board zoom">
      <button aria-label="Zoom out" disabled={camera.zoom <= 1} onclick={() => zoomTo(camera.zoom / 1.25)}>−</button>
      <output aria-label="Board zoom level">{Math.round(camera.zoom * 100)}%</output>
      <button aria-label="Zoom in" disabled={camera.zoom >= 4} onclick={() => zoomTo(camera.zoom * 1.25)}>+</button>
      <button onclick={fit} aria-label="Fit whole motherboard">Fit</button>
    </div>
  </div>
  <p class="board-instructions" id={`${uid}-help`}>Click hardware to inspect · Drag to pan when zoomed · Ctrl/⌘ + scroll or pinch to zoom · Keyboard: + / −, arrows, 0 to fit</p>
  {#if mode === 'thermal'}<div class="thermal-legend"><span class="thermal-ramp"></span><span>30°C → 95°C</span><span>Gray = no sensor · Color scale, not thermal limits</span></div>{/if}
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions (This bounded pan/zoom canvas accepts arrow keys; native hardware buttons remain reachable with Tab.) -->
  <div bind:this={viewport} bind:clientWidth={viewWidth} bind:clientHeight={viewHeight} use:gestures class="board-viewport" class:dragging class:zoomed={camera.zoom > 1}
    role="application" tabindex="0" aria-label="Interactive motherboard canvas" aria-describedby={`${uid}-help`} onkeydown={cameraKeys}>
    <div class="board-canvas" class:paused class:thermal={mode === 'thermal'} style={`width:${fitted.width}px;height:${fitted.height}px;transform:translate(calc(-50% + ${camera.x}px),calc(-50% + ${camera.y}px)) scale(${camera.zoom})`}>
      <BoardSurface {board} identity={frame.motherboard} {paused}/>
      <svg class="pcb" viewBox={`0 0 ${board.width} ${board.height}`} aria-hidden="true">
        {#each board.nodes.filter(n => n.kind !== 'CPU') as node (node.id)}
          <path d={trace(node)} class="board-trace" class:focused={highlight === node.id || highlight === 'CPU'}/>
          {#each boardSignals(node, frame) as signal (signal.direction)}
            <path d={trace(node)} class="board-signal" class:inbound={signal.direction === 'in'} pathLength="100"
              style={`--signal-duration:${8-signal.strength*6}s;opacity:${.2+signal.strength*.65};--signal-color:${signal.direction === 'in' ? 'var(--info)' : 'var(--secondary)'}`}/>
          {/each}
        {/each}
      </svg>
      {#each board.nodes as node (node.id)}
        <button class="hardware-button" data-kind={node.kind} data-node-id={node.id} class:active={selectedId === node.id} class:inspecting={opened?.anchor?.dataset.nodeId === node.id}
          class:powered={mode === 'thermal' ? thermalColor(node.temperatureC) !== null : intensity(node.activity) > 0}
          class:cpu-pulse={node.kind === 'CPU' && mode === 'activity' && intensity(node.activity) > 0}
          class:unknown-thermal={mode === 'thermal' && thermalColor(node.temperatureC) === null}
          style={`left:${node.x/board.width*100}%;top:${node.y/board.height*100}%;width:${node.width/board.width*100}%;height:${node.height/board.height*100}%;${partStyle(node,mode)}--label-scale:${1/camera.zoom}`}
          aria-label={`Inspect ${node.role}: ${node.name}`} aria-pressed={selectedId === node.id}
          aria-haspopup="dialog" aria-expanded={opened?.anchor?.dataset.nodeId === node.id} aria-controls={opened?.anchor?.dataset.nodeId === node.id ? `${uid}-details` : undefined}
          onpointerenter={() => hovered = node.id} onpointerleave={() => hovered = null}
          onfocus={event => focusPart(event, node.id)} onblur={() => focused = null}
          onclick={event => inspect(node, event.currentTarget)}>
          <BoardHardware {node} {paused} slots={board.memorySlots} memoryDevices={frame.motherboard?.memoryDevices ?? []} compactGpu={board.profile.family === 'portable' && !node.detached}/>
          <span class="heat-wash" aria-hidden="true"></span>
          <span class="part-label" aria-hidden="true"><small>{node.role}</small>{node.name}</span>
        </button>
      {/each}
    </div>
  </div>
  <div class="board-legend"><span>{mode === 'thermal' ? '♨ MEASURED TEMPERATURE COLOR' : '◉ ACTIVITY GLOW'}</span><span class="in-key">← READ / RECEIVE</span><span class="out-key">WRITE / TRANSMIT →</span><span>{paused ? 'Ⅱ ANIMATION PAUSED' : '⌁ SIMULATED SIGNALS'}</span></div>
  <p class="board-explanation">{board.profile.matched ? 'Board shape and connector counts follow the model profile. GPU/storage overlays do not identify occupied sockets.' : 'Layout adapts to reported hardware and chassis; board shape and placement are illustrative.'} RAM: solid = installed, empty socket = unoccupied, dashed = unknown. Enlarged details and signals are illustrative.</p>
  {#if opened && openNode}
    {#key opened.id}<BoardPopover id={`${uid}-details`} anchor={opened.anchor} node={openNode} {frame} {history} {mode} {packageMembers} packageOpen={opened.packageOpen} {togglePackage} {choosePackage} {paused} {dismiss}/>{/key}
  {/if}
</section>

<style>
  .motherboard-view { position: relative; contain: paint; margin-bottom: 24px; border: 1px solid var(--border); border-radius: 12px; background: var(--deep); overflow: hidden; }
  .board-caption, .board-legend { display: flex; flex-wrap: wrap; justify-content: space-between; gap: 10px; padding: 14px 18px; font: 9px 'JetBrains Mono', monospace; color: var(--secondary); letter-spacing: .08em; }
  .board-caption { border-bottom: 1px solid var(--border); }
  .board-caption span:last-child, .board-legend { color: var(--muted); }
  .board-instructions, .board-explanation { margin: 0; padding: 12px 18px 0; color: var(--muted); font-size: 11px; line-height: 1.6; }
  .board-explanation { border-top: 1px solid var(--border); padding: 12px 18px; font-size: 10px; }
  .board-toolbar { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 10px; padding: 12px 18px 0; }
  .mode-switch, .zoom-controls { display: flex; align-items: center; gap: 4px; }
  .board-toolbar button { padding: 7px 10px; border: 1px solid var(--border); border-radius: 5px; color: var(--muted); background: var(--panel); font: 10px 'JetBrains Mono',monospace; }
  .board-toolbar button.chosen, .board-toolbar button:hover:not(:disabled) { border-color: var(--primary); color: var(--text); }
  .board-toolbar button:disabled { opacity: .35; cursor: default; }
  .board-toolbar button:focus-visible, .board-viewport:focus-visible { outline: 2px solid var(--info); outline-offset: -2px; }
  .zoom-controls output { min-width: 40px; text-align: center; color: var(--muted); font: 10px 'JetBrains Mono',monospace; }
  .thermal-legend { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; padding: 10px 18px 0; color: var(--muted); font: 9px 'JetBrains Mono',monospace; }
  .thermal-ramp { width: 70px; height: 6px; border-radius: 4px; background: linear-gradient(90deg,hsl(210 88% 65%),hsl(140 88% 65%),hsl(70 88% 65%),hsl(0 88% 65%)); }
  .board-viewport { position: relative; height: clamp(360px,65vh,760px); overflow: hidden; touch-action: none; user-select: none; margin-top: 10px; background: radial-gradient(ellipse at center,#2b51451c,transparent 70%); }
  .board-viewport.zoomed { cursor: grab; }
  .board-viewport.dragging, .board-viewport.dragging * { cursor: grabbing; }
  .board-canvas { position: absolute; left: 50%; top: 50%; transform-origin: center; isolation: isolate; }
  .in-key { color: var(--info); } .out-key { color: var(--secondary); }
  .pcb { position: absolute; width: 100%; height: 100%; pointer-events: none; }
  .board-trace { fill: none; stroke: #7c9e8c; stroke-width: 2; opacity: .35; }
  .board-trace.focused { stroke: var(--primary); opacity: .85; filter: drop-shadow(0 0 4px var(--primary)); }
  .board-signal { fill: none; stroke: var(--signal-color); stroke-width: 2.5; stroke-linecap: round; stroke-dasharray: 2 23; filter: drop-shadow(0 0 3px var(--signal-color)); animation: signal-flow var(--signal-duration) linear infinite; }
  .board-signal.inbound { animation-name: signal-return; }
  @keyframes signal-return { from { stroke-dashoffset: 12.5; } to { stroke-dashoffset: 112.5; } }
  @keyframes signal-flow { to { stroke-dashoffset: -100; } }
  .paused .board-signal, .paused .cpu-pulse .heat-wash { animation-play-state: paused; }
  .hardware-button { --part-accent: var(--info); position: absolute; z-index: 1; padding: 0; border: 0; border-radius: 4px; background: transparent; color: var(--text); filter: drop-shadow(0 3px 3px #000c); transition: filter .25s; }
  .hardware-button.powered { filter: drop-shadow(0 0 var(--glow-size) color-mix(in srgb, var(--part-accent) var(--glow-strength), transparent)) drop-shadow(0 3px 3px #000b); }
  .hardware-button.active::after, .hardware-button:hover::after { content: ''; position: absolute; inset: -5px; border: 1px solid color-mix(in srgb, var(--part-accent) 70%, transparent); border-radius: 5px; pointer-events: none; }
  .hardware-button.active::after { border-color: var(--primary); box-shadow: 0 0 12px color-mix(in srgb, var(--primary) 15%, transparent); }
  .hardware-button:focus-visible { outline: 2px solid var(--info); outline-offset: 6px; }
  .heat-wash { position: absolute; inset: 5%; border-radius: 8px; pointer-events: none; opacity: 0; background: var(--part-accent); mix-blend-mode: screen; }
  .thermal .powered .heat-wash { opacity: .2; }
  .unknown-thermal { filter: saturate(.25) drop-shadow(0 3px 3px #000c); }
  .cpu-pulse .heat-wash { animation: cpu-breathe var(--pulse-duration) ease-in-out infinite alternate; }
  @keyframes cpu-breathe { from { opacity: .02; } to { opacity: var(--pulse-opacity); } }
  .part-label { position: absolute; bottom: calc(100% + 10px); left: 50%; transform: translateX(-50%) scale(var(--label-scale)); transform-origin: bottom center; display: none; padding: 5px 8px; border: 1px solid var(--part-accent); border-radius: 4px; background: #111d24ee; color: #e0ece9; width: max-content; max-width: 200px; text-align: center; white-space: normal; font: 10px 'JetBrains Mono',monospace; pointer-events: none; }
  .part-label small { display: block; margin-bottom: 4px; color: var(--part-accent); font-size: 8px; letter-spacing: .08em; }
  .hardware-button:hover, .hardware-button:focus-visible { z-index: 3; }
  .hardware-button:hover .part-label, .hardware-button:focus-visible .part-label { display: block; }
  .hardware-button.inspecting .part-label { display: none; }
  @media (prefers-reduced-motion: reduce) { .board-signal, .cpu-pulse .heat-wash { animation: none; } .hardware-button { transition: none; } }
</style>
