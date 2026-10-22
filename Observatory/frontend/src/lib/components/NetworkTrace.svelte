<script lang="ts">
  import { onMount } from 'svelte';
  import {
    demoDevices, demoSites, type FleetDevice, type FleetSite, type SiteId,
  } from '../data/fleet';
  import {
    traceNodes, traceEvents, simulatedActivity,
    type TraceNode, type TraceEvent,
  } from '../data/networkTrace';
  import './network-trace.css';

  interface Props {
    selectedDeviceId: string | null;
    selectDevice: (deviceId: string) => void;
  }
  let { selectedDeviceId, selectDevice }: Props = $props();

  let activeSite = $state<SiteId | 'all'>('all');
  let paused = $state<boolean>(false);
  let motion = $state<boolean>(true);
  let step = $state<number>(4);
  let activeProtocol = $state<string>('ALL');

  const protocolOptions: string[] = ['ALL', 'HTTPS', 'DNS', 'SMB', 'QUIC', 'SSH', 'RDP'];
  let visibleSites = $derived(demoSites.filter((site: FleetSite) => activeSite === 'all' || site.id === activeSite));
  let selectedDevice = $derived(demoDevices.find((device: FleetDevice) => device.id === selectedDeviceId));
  let selectedSite = $derived(selectedDevice ? demoSites.find((site: FleetSite) => site.id === selectedDevice.siteId) : undefined);
  let allEvents = $derived(visibleSites.flatMap((site: FleetSite): TraceEvent[] => traceEvents(site.id, step)));
  let filteredEvents = $derived(allEvents.filter((event: TraceEvent) => activeProtocol === 'ALL' || event.protocol === activeProtocol));
  let visibleDevices = $derived(demoDevices.filter((device: FleetDevice) => activeSite === 'all' || device.siteId === activeSite));
  let onlineCount = $derived(visibleDevices.filter((device: FleetDevice) => device.status !== 'offline').length);
  let activitySum = $derived(visibleDevices.reduce((sum: number, device: FleetDevice) => sum + simulatedActivity(device.id, step), 0));

  onMount((): (() => void) => {
    const timer: number = window.setInterval((): void => {
      if (!paused) step += 1;
    }, 1100);
    return (): void => window.clearInterval(timer);
  });

  function selectSite(siteId: SiteId | 'all'): void {
    activeSite = siteId;
  }

  function siteActivity(siteId: SiteId): number {
    return traceNodes(siteId).reduce((total: number, node: TraceNode) => total + simulatedActivity(node.device.id, step), 0);
  }

  function setFocus(deviceId: string): void {
    selectDevice(deviceId);
  }
</script>

<section class="nettrace" aria-label="Network tracing visualization preview">
  <div class="nettrace-heading">
    <div>
      <span class="eyebrow">FLEET / NETWORK ACTIVITY / LAB</span>
      <h2>NETWORK <em>TRACE</em></h2>
      <p>Explore three isolated office networks. Select a site, then follow illustrative endpoint activity.</p>
    </div>
    <div class="nettrace-heading-status"><span class="nettrace-mode">◇ SYNTHETIC TRACE</span><span class="nettrace-clock">{paused ? 'Ⅱ FROZEN' : '◉ PLAYBACK'}</span></div>
  </div>

  <div class="nettrace-caution" role="note">
    <span>◇ DEMO ENVIRONMENT</span>
    <p>All nodes, traffic volumes, protocol labels, and events are simulated. This panel does not capture packets, inspect browsing activity, or detect actual LAN topology. Office networks are shown independently.</p>
  </div>

  <div class="nettrace-toolbar">
    <div class="nettrace-tabs" role="group" aria-label="Network site filter">
      <button class:chosen={activeSite === 'all'} onclick={() => selectSite('all')}>ALL SITES</button>
      {#each demoSites as site (site.id)}
        <button class:chosen={activeSite === site.id} onclick={() => selectSite(site.id)}>{site.name}</button>
      {/each}
    </div>
    <div class="nettrace-actions">
      <button class:chosen={paused} onclick={() => paused = !paused}>{paused ? '▶ RESUME' : 'Ⅱ PAUSE'}</button>
      <button class:chosen={motion} onclick={() => motion = !motion} aria-pressed={motion}>✦ FLOW {motion ? 'ON' : 'OFF'}</button>
    </div>
  </div>

  <div class="nettrace-stats">
    <div><small>NETWORK SEGMENTS</small><strong>{visibleSites.length.toString().padStart(2, '0')}</strong><span>ISOLATED SITES</span></div>
    <div><small>DEMO ENDPOINTS</small><strong>{visibleDevices.length.toString().padStart(2, '0')}</strong><span>REGISTERED NODES</span></div>
    <div><small>DEMO AVAILABLE</small><strong>{onlineCount.toString().padStart(2, '0')}</strong><span>NON-OFFLINE</span></div>
    <div><small>ILLUSTRATIVE LOAD</small><strong>{activitySum.toString().padStart(3, '0')}</strong><span>SIMULATED UNITS</span></div>
  </div>

  <div class:single={visibleSites.length === 1} class="nettrace-site-grid">
    {#each visibleSites as site (site.id)}
      <article class="nettrace-site" aria-label={`${site.name} illustrative network`}>
        <header class="nettrace-site-title">
          <div><span class="nettrace-site-index">SEGMENT / {site.id.toUpperCase()}</span><h3>{site.name}</h3></div>
          <span class="nettrace-site-pulse">◉ {siteActivity(site.id)} DEMO UNITS</span>
        </header>
        <div class="nettrace-field" class:still={!motion || paused}>
          <div class="nettrace-grid" aria-hidden="true"></div>
          <svg class="nettrace-wires" viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true">
            <circle cx="50" cy="50" r="31" class="nettrace-ring" />
            <circle cx="50" cy="50" r="19" class="nettrace-ring inner" />
            {#each traceNodes(site.id) as node (node.device.id)}
              <line x1="50" y1="50" x2={node.x} y2={node.y} class:focused={selectedDeviceId === node.device.id} class:offline={node.device.status === 'offline'} class="nettrace-wire" />
              {#if node.device.status !== 'offline'}
                <line x1="50" y1="50" x2={node.x} y2={node.y} class:focused={selectedDeviceId === node.device.id} class="nettrace-packet" style={`--packet-delay: -${(node.x * 17 + node.y * 3) % 43 / 10}s; --packet-duration: ${1.7 + ((node.x + node.y) % 6) / 7}s`} />
              {/if}
            {/each}
          </svg>
          <div class="nettrace-router" aria-label={`${site.name} conceptual local network hub`}>
            <span class="nettrace-router-icon">⌘</span><strong>LAN CORE</strong><small>CONCEPTUAL</small>
          </div>
          {#each traceNodes(site.id) as node (node.device.id)}
            <button
              class="nettrace-node"
              class:selected={selectedDeviceId === node.device.id}
              class:offline={node.device.status === 'offline'}
              class:warning={node.device.status === 'warning'}
              style={`left:${node.x}%;top:${node.y}%`}
              onclick={() => setFocus(node.device.id)}
              aria-label={`Inspect ${node.device.name}, synthetic ${node.device.status} node in ${site.name}`}
              title={`${node.device.name} • demo ${node.device.status}`}
            >
              <span class="nettrace-node-glyph">{node.device.role === 'Server' ? '▣' : '▤'}</span>
              <strong>{node.device.name}</strong>
              <small>{node.device.status === 'offline' ? 'OFFLINE' : `${simulatedActivity(node.device.id, step)} DEMO`}</small>
            </button>
          {/each}
        </div>
        <footer class="nettrace-site-foot">
          <span>⌁ PRIVATE SITE BOUNDARY</span>
          <button onclick={() => selectSite(site.id)} aria-label={`Focus ${site.name} network`}>FOCUS SITE ↗</button>
        </footer>
      </article>
    {/each}
  </div>

  <div class="nettrace-bottom">
    <section class="nettrace-feed" aria-label="Illustrative network activity log">
      <header class="nettrace-section-header"><div><span class="eyebrow">EVENT PLAYBACK</span><h3>ILLUSTRATIVE FLOWS</h3></div>
        <label>PROTOCOL
          <select bind:value={activeProtocol} aria-label="Filter demonstration protocols">
            {#each protocolOptions as protocol}<option value={protocol}>{protocol}</option>{/each}
          </select>
        </label>
      </header>
      <div class="nettrace-log" aria-live="off">
        <div class="nettrace-log-head"><span>DEVICE</span><span>PROTO</span><span>DIRECTION</span><span>DEMO KiB</span></div>
        {#each filteredEvents as event (event.id)}
          <button class:selected={selectedDeviceId === event.deviceId} class="nettrace-event" onclick={() => setFocus(event.deviceId)}>
            <span>{event.deviceName}<small>{event.siteId.toUpperCase()} · T−{event.relativeSeconds}s</small></span>
            <span>{event.protocol}</span><span>{event.direction}</span><strong>{event.amountKiB}</strong>
          </button>
        {:else}
          <p class="nettrace-no-events">No synthetic events match this filter.</p>
        {/each}
      </div>
    </section>
    <section class="nettrace-details" aria-label="Selected demonstration endpoint">
      <div class="nettrace-section-header"><div><span class="eyebrow">SELECTED NODE</span><h3>{selectedDevice?.name ?? 'NO DEVICE SELECTED'}</h3></div></div>
      {#if selectedDevice}
        <div class="nettrace-detail-emblem">▣ <span>DEMO ENDPOINT</span></div>
        <dl>
          <div><dt>SITE</dt><dd>{selectedSite?.name ?? 'UNKNOWN'}</dd></div>
          <div><dt>DEVICE ROLE</dt><dd>{selectedDevice.role}</dd></div>
          <div><dt>AGENT OS</dt><dd>{selectedDevice.os}</dd></div>
          <div><dt>SIMULATED STATE</dt><dd>{selectedDevice.status.toUpperCase()}</dd></div>
          <div><dt>DEMO ACTIVITY</dt><dd>{simulatedActivity(selectedDevice.id, step)} units</dd></div>
          <div><dt>SOURCE</dt><dd>SYNTHETIC ONLY</dd></div>
        </dl>
      {:else}
        <p class="nettrace-hint">Select a demo endpoint on any site diagram or activity row to focus its Fleet inspector.</p>
      {/if}
      <div class="nettrace-roadmap"><span>◇ LIVE DATA CONTRACT</span><p>Future Linux Agent/Hub integration can provide observed connections, process attribution, protocol, destination, rates, timestamps and collection scope. No real network inspection occurs in this preview.</p></div>
    </section>
  </div>
</section>
