<script lang="ts">
  interface Props {
    title: string;
    subtitle: string;
    values: number[];
    times: number[];
    tint?: 'primary' | 'info' | 'secondary' | 'success' | 'warning';
    unit?: string;
    maximum?: number;
    hoverIndex: number | null;
    pinnedIndex: number | null;
    onHover: (index: number | null) => void;
    onPin: (index: number | null) => void;
  }

  let {
    title, subtitle, values, times,
    tint = 'primary', unit = '%', maximum = 100,
    hoverIndex, pinnedIndex, onHover, onPin,
  }: Props = $props();

  const width: number = 600;
  const height: number = 162;
  const inset: number = 12;

  let points = $derived(values.map((value: number, index: number) => ({
    x: inset + (index / Math.max(values.length - 1, 1)) * (width - inset * 2),
    y: height - inset - (Math.min(maximum, Math.max(0, value)) / Math.max(maximum, 1)) * (height - inset * 2),
    value,
    time: times[index] ?? 0,
  })));
  let line = $derived(points.map((p, i: number) => `${i === 0 ? 'M' : 'L'} ${p.x.toFixed(2)} ${p.y.toFixed(2)}`).join(' '));
  let area = $derived(points.length ? `${line} L ${points[points.length - 1].x} ${height} L ${points[0].x} ${height} Z` : '');
  let focusIndex = $derived(pinnedIndex ?? hoverIndex ?? Math.max(0, points.length - 1));
  let focused = $derived(points[focusIndex]);

  function indexAt(event: PointerEvent): number {
    const target: SVGSVGElement = event.currentTarget as SVGSVGElement;
    const rect: DOMRect = target.getBoundingClientRect();
    // Account for the SVG viewBox's horizontal inset.
    const fraction: number = Math.max(0, Math.min(1, (event.clientX - rect.left) / Math.max(rect.width, 1)));
    return Math.round(fraction * Math.max(values.length - 1, 0));
  }
</script>

<article class="chart-panel" data-tint={tint}>
  <div class="chart-head">
    <div><div class="eyebrow">{subtitle}</div><h3>{title}</h3></div>
    <div class="chart-value">{focused ? focused.value.toFixed(1) : '0.0'}<small>{unit}</small></div>
  </div>
  <div class="chart-plot" role="button" tabindex="0" aria-label={`Pin ${title} sample at cursor`}
    onclick={(event: MouseEvent) => {
      const rect: DOMRect = (event.currentTarget as HTMLDivElement).getBoundingClientRect();
      const index: number = Math.round(Math.max(0, Math.min(1, (event.clientX - rect.left) / Math.max(rect.width, 1))) * Math.max(values.length - 1, 0));
      onPin(pinnedIndex === index ? null : index);
    }}
    onkeydown={(event: KeyboardEvent) => {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        const index: number = hoverIndex ?? Math.max(values.length - 1, 0);
        onPin(pinnedIndex === index ? null : index);
      }
    }}>
    <svg viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="none" role="img"
      aria-label={`${title}. Simulated time series, use pointer to inspect a synchronized sample.`}
      onpointermove={(event: PointerEvent) => onHover(indexAt(event))}
      onpointerleave={() => onHover(null)}>
      <line x1="0" x2={width} y1="40" y2="40" class="chart-grid" />
      <line x1="0" x2={width} y1="82" y2="82" class="chart-grid" />
      <line x1="0" x2={width} y1="122" y2="122" class="chart-grid" />
      <path d={area} class="chart-area" />
      <path d={line} class="chart-line" />
      {#if (hoverIndex !== null || pinnedIndex !== null) && focused}
        <line x1={focused.x} x2={focused.x} y1="0" y2={height} class="chart-guide" />
        <circle cx={focused.x} cy={focused.y} r="4.3" class="chart-dot" />
      {/if}
    </svg>
    {#if (hoverIndex !== null || pinnedIndex !== null) && focused}
      <div class="chart-cursor-readout">T+{focused.time.toFixed(2)}s · {focused.value.toFixed(2)} {unit} {pinnedIndex !== null ? '◈ PINNED' : '◇ HOVER'}</div>
    {/if}
  </div>
  <div class="chart-axis"><span>OLDER</span><span>SHARED CURSOR · CLICK TO PIN</span><span>NOW</span></div>
</article>
