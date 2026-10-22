<script lang="ts">
  import type { MachineNode } from '../data/machine';
  import { partStyle, type BoardMode } from '../data/boardEffects';
  let { nodes, selectedId, choose, mode }: { nodes: MachineNode[]; selectedId: string; choose: (node: MachineNode) => void; mode: BoardMode } = $props();
</script>
<div class="package-lens">
  <div class="package-silk">COMPUTE PACKAGE / ENLARGED</div>
  <div class="dies">
    {#each nodes as node (node.id)}
      <button class:unknown={mode === 'thermal' && !Number.isFinite(node.temperatureC)} class:cpu={node.kind === 'CPU'} class:chosen={selectedId === node.id} style={partStyle(node, mode)} aria-pressed={selectedId === node.id} aria-label={`Inspect package ${node.role}: ${node.name}`} onclick={() => choose(node)}>
        <span class="die-cells" aria-hidden="true">{#each Array.from({length: node.kind === 'CPU' ? 12 : 6}) as _}<i></i>{/each}</span>
        <strong>{node.kind === 'CPU' ? 'CPU CORES' : node.kind === 'GPU' ? 'iGPU' : 'NPU'}</strong>
      </button>
    {/each}
  </div>
  <p>Conceptual layout. Grouping uses reported device identities; die placement and cell counts are illustrative.</p>
</div>
<style>
  .package-lens { margin: 12px 0; padding: 12px; border: 4px double #7b9786; border-radius: 8px; background: repeating-linear-gradient(90deg,#28473744 0 1px,transparent 1px 9px), #142522; box-shadow: inset 0 0 16px #0009; }
  .package-silk { color: #b0c6b7; font: 8px 'JetBrains Mono', monospace; letter-spacing: .08em; margin-bottom: 12px; }
  .dies { display: grid; grid-template-columns: repeat(2,minmax(0,1fr)); gap: 10px; }
  button { position: relative; overflow: hidden; padding: 9px; min-height: 64px; border: 1px solid #718a91; background: linear-gradient(135deg,#274c60,#111c28 55%,#416879); color: #d9e7eb; box-shadow: 0 0 var(--glow-size) color-mix(in srgb,var(--part-accent) var(--glow-strength),transparent); }
  button.unknown { filter: saturate(0); box-shadow: none; }
  button.cpu { grid-column: 1 / -1; }
  button.chosen, button:hover { border-color: var(--part-accent); }
  button:focus-visible { outline: 2px solid var(--info); outline-offset: 3px; }
  .die-cells { display: grid; grid-template-columns: repeat(6,minmax(0,1fr)); gap: 3px; margin-bottom: 7px; }
  .die-cells i { height: 10px; border: 1px solid #84acba66; background: repeating-linear-gradient(90deg,#86bbce33 0 1px,transparent 1px 3px); }
  strong { font: 9px 'JetBrains Mono', monospace; letter-spacing: .1em; }
  p { margin: 10px 0 0; color: #a3b4ac; font-size: 9px; line-height: 1.5; }
</style>
