<script lang="ts">
  import { formatNumber } from '../data/bridge';
  import type { TelemetryFrame } from '../data/telemetry';

  let { cpu }: { cpu: TelemetryFrame['cpu'] } = $props();
</script>

<section class="cpu-identity" aria-label="CPU identity">
  <div class="processor">
    <span class="eyebrow">PROCESSOR</span>
    <h2>{cpu.name.trim() || 'CPU model not reported'}</h2>
  </div>
  <dl>
    <div><dt>VENDOR</dt><dd>{cpu.vendor.trim() || 'Not reported'}</dd></div>
    <div><dt>PHYSICAL CORES</dt><dd>{cpu.physicalCoreCount !== null && cpu.physicalCoreCount > 0 ? formatNumber(cpu.physicalCoreCount, 0) : 'Not reported'}</dd></div>
    <div><dt>LOGICAL CPUs</dt><dd>{cpu.logicalCpuCount > 0 ? formatNumber(cpu.logicalCpuCount, 0) : 'Not reported'}</dd></div>
  </dl>
</section>

<style>
  .cpu-identity { grid-column: 1 / -1; display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 16px 24px; min-width: 0; padding: 16px 18px; border: 1px solid var(--border); border-radius: 9px; background: var(--panel); }
  .processor { flex: 1 1 240px; min-width: 0; }
  h2 { margin: 6px 0 0; color: var(--bright); font-size: clamp(17px, 1.6vw, 22px); line-height: 1.4; overflow-wrap: anywhere; }
  dl { display: flex; flex-wrap: wrap; gap: 14px 24px; margin: 0; min-width: 0; }
  dl > div { min-width: 0; }
  dt { color: var(--muted); font: 10px 'JetBrains Mono', monospace; letter-spacing: .05em; }
  dd { margin: 7px 0 0; color: var(--info); font: 12px/1.5 'JetBrains Mono', monospace; overflow-wrap: anywhere; }
</style>
