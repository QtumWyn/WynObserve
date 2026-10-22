<script lang="ts">
  import type { Motherboard } from '../data/telemetry';
  import { boardName, boardProfile, memoryInventoryText } from '../data/motherboard';
  let { board, demo = false }: { board?: Motherboard | null; demo?: boolean } = $props();
  let profile = $derived(boardProfile(board));
  let slots = $derived(board?.memorySlotCount ?? (profile.memoryLabels.length || null));
</script>
<div class="board-identity" aria-label="Motherboard identity">
  <div class="identity-main"><span>MOTHERBOARD {demo ? '· DEMO' : ''}</span><strong>{boardName(board)}</strong>
    <p>{board?.version ? `Revision ${board.version} · ` : ''}{board?.systemModel ?? (demo ? 'Synthetic telemetry' : 'Selected machine firmware')}</p></div>
  <div class="identity-detail"><b>{profile.label}</b><span>{slots ? `${slots} memory positions · ` : ''}{memoryInventoryText(board)}</span>
    <span>{profile.matched ? 'Model profile · installed slot locations unverified' : 'Adaptive layout · exact board geometry unavailable'}</span></div>
</div>
<style>
  .board-identity { display: flex; justify-content: space-between; gap: 16px; flex-wrap: wrap; margin: 8px 0 20px; padding: 17px 20px; border: 1px solid var(--border); border-left: 2px solid var(--secondary); border-radius: 8px; background: var(--panel); }
  .identity-main { min-width: 0; flex: 1 1 260px; }
  .identity-main > span { display: block; color: var(--secondary); font: 9px 'JetBrains Mono',monospace; letter-spacing: .12em; margin-bottom: 8px; }
  strong { color: var(--text); font-size: 17px; font-weight: 500; overflow-wrap: anywhere; }
  p { margin: 6px 0 0; color: var(--muted); font-size: 11px; }
  .identity-detail { display: flex; flex-direction: column; justify-content: center; gap: 6px; color: var(--muted); font-size: 10px; }
  .identity-detail b { color: var(--secondary); font-weight: 500; font-size: 11px; }
</style>
