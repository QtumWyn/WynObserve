<script lang="ts">
  import type { HistoryPoint } from '../data/bridge';
  import { sparkline, type BoardMetric } from '../data/boardEffects';
  let { history, metric }: { history: HistoryPoint[]; metric: BoardMetric } = $props();
  let chart = $derived(sparkline(history, metric));
</script>
<figure>
  <figcaption><span>{metric.label} <small>{metric.unit}</small></span><small>{chart.count ? `${Math.round(chart.seconds)}s history` : 'No samples'}</small></figcaption>
  <svg viewBox="0 0 240 50" role="img" aria-label={`${metric.label}: ${chart.count} measured samples over ${Math.round(chart.seconds)} seconds. Gaps mean unavailable.`}>
    <path d="M2 4H238 M2 25H238 M2 46H238" class="grid"/>
    <path d={chart.path} class="trend"/>
    {#each chart.dots as dot}<circle cx={dot.x} cy={dot.y} r="1.1"/>{/each}
    {#if !chart.count}<text x="120" y="29" text-anchor="middle">NO MEASURED HISTORY</text>{/if}
  </svg>
  <div class="scale"><span>{chart.minimum}</span><span>{chart.maximum.toLocaleString(undefined,{maximumFractionDigits:1})} {metric.unit}</span></div>
</figure>
<style>
  figure { margin: 0; min-width: 0; padding: 10px 0 4px; border-top: 1px solid var(--border); }
  figcaption, .scale { display: flex; justify-content: space-between; gap: 8px; font: 9px 'JetBrains Mono', monospace; color: var(--secondary); }
  small, .scale { font-size: 8px; color: var(--muted); }
  svg { width: 100%; height: 50px; display: block; margin-top: 6px; }
  .grid { stroke: var(--border); stroke-width: .5; fill: none; }
  .trend { stroke: var(--info); stroke-width: 1.5; stroke-linejoin: round; fill: none; }
  circle { fill: var(--info); }
  text { fill: var(--muted); font: 8px 'JetBrains Mono', monospace; }
</style>
