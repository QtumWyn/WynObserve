<script lang="ts">
  import { formatNumber } from '../data/bridge';
  import { tick } from 'svelte';
  import ContagionMap from './ContagionMap.svelte';
  import NetworkTrace from './NetworkTrace.svelte';
  import {
    demoDevices, demoSites, siteDeviceCount, siteStatus,
    type FleetDevice, type FleetSite, type FleetStatus, type SiteId,
  } from '../data/fleet';

  interface Props {
    devices?: FleetDevice[];
    source?: 'demo' | 'live';
    selectedDeviceId: string | null;
    selectDevice: (deviceId: string) => void;
  }
  let { selectedDeviceId, selectDevice, devices = demoDevices, source = 'demo' }: Props = $props();
  let mode = $state<'list' | 'map' | 'trace'>('list');
  let previousFleetMode: typeof mode = 'list';
  $effect((): void => {
    const nextMode: typeof mode = mode;
    if (nextMode === previousFleetMode) return;
    previousFleetMode = nextMode;
    void tick().then((): void => {
      // Fleet list/map/trace are different views sharing the same scroll surface.
      const content = document.querySelector<HTMLElement>('.content');
      const workspace = document.querySelector<HTMLElement>('.workspace');
      if (content) content.scrollTop = 0;
      if (workspace) workspace.scrollTop = 0;
    });
  });

  let selectedSiteId = $state<SiteId | null>(null);
  let query = $state<string>('');
  let statusFilter = $state<'all' | FleetStatus>('all');
  let siteFilter = $state<'all' | SiteId>('all');

  let filtered = $derived(devices.filter((device: FleetDevice): boolean => {
    const matchesQuery: boolean = [device.name, device.role, device.os, device.siteId]
      .some((value: string): boolean => value.toLowerCase().includes(query.toLowerCase()));
    return matchesQuery && (statusFilter === 'all' || device.status === statusFilter)
      && (siteFilter === 'all' || device.siteId === siteFilter);
  }));

  let online = $derived(devices.filter((device: FleetDevice) => device.status === 'online').length);
  let warnings = $derived(devices.filter((device: FleetDevice) => device.status === 'warning').length);
  let offline = $derived(devices.filter((device: FleetDevice) => device.status === 'offline').length);
  let selectedSite = $derived(demoSites.find((site: FleetSite) => site.id === selectedSiteId));
  let selectedSiteDevices = $derived(devices.filter((device: FleetDevice) => device.siteId === selectedSiteId));
</script>

<div class="page-heading fleet-page-heading">
  <div><div class="eyebrow">CORE / FLEET / 003</div><h1>THE <em>FLEET</em></h1>
    <p>Observe endpoint presence, group devices by office and inspect a regional signal map.</p></div>
  <div class="fleet-view-switch" role="group" aria-label="Fleet display mode">
    <button class:chosen={mode === 'list'} onclick={() => mode = 'list'}>☷ LIST VIEW</button>
    <button class:chosen={mode === 'map'} onclick={() => mode = 'map'}>◈ CONTAGION MAP</button>
    <button class:chosen={mode === 'trace'} onclick={() => mode = 'trace'}>⌁ NETWORK TRACE</button>
  </div>
</div>

<div class="fleet-banner"><span>{source === 'demo' ? '◇ DEMONSTRATION DATA' : '● HUB FLEET'}</span><p>{source === 'demo' ? 'Synthetic endpoints and office coordinates.' : 'Select a machine to target the hardware pages. Stale devices retain their last readings.'}</p></div>

<div class="fleet-count-row">
  <div><small>ENROLLED</small><strong>{devices.length}</strong></div>
  <div><small>ONLINE</small><strong class="green">{online}</strong></div>
  <div><small>WARNING</small><strong class="amber">{warnings}</strong></div>
  <div><small>OFFLINE</small><strong class="red">{offline}</strong></div>
  <div><small>OFFICE SITES</small><strong>{source === 'demo' ? demoSites.length : 'N/A'}</strong></div>
</div>

{#if mode === 'list'}
  <div class="fleet-filter-row">
    <input aria-label="Search devices" placeholder="⌕ Search devices, OS or office..." bind:value={query} />
    <select aria-label="Office filter" bind:value={siteFilter}>
      <option value="all">ALL OFFICES</option>
      {#if source === 'demo'}{#each demoSites as site (site.id)}<option value={site.id}>{site.name}</option>{/each}{/if}
    </select>
    <select aria-label="Status filter" bind:value={statusFilter}>
      <option value="all">ALL STATES</option>
      <option value="online">ONLINE</option>
      <option value="warning">WARNING</option>
      <option value="offline">OFFLINE</option>
    </select>
  </div>
  <div class="fleet-table-wrap">
    <table class="fleet-table">
      <thead><tr><th>STATUS</th><th>DEVICE / ROLE</th><th>OFFICE</th><th>OS</th><th>AGENT</th><th>LAST SEEN</th><th>CPU</th><th>RAM</th></tr></thead>
      <tbody>
        {#each filtered as device (device.id)}
          <tr class:focused={selectedDeviceId === device.id}>
            <td><span class={`fleet-status ${device.status}`}><i></i>{device.status.toUpperCase()}</span></td>
            <td><button class="fleet-name-button" onclick={() => selectDevice(device.id)}>{device.name}</button><small>{device.role}</small></td>
            <td>{demoSites.find((site: FleetSite) => site.id === device.siteId)?.name ?? 'UNASSIGNED'}</td>
            <td>{device.os}</td><td>{device.agentVersion}</td><td>{device.lastSeen}</td>
            <td>{device.status === 'offline' ? '—' : `${formatNumber(device.cpu)}%`}</td>
            <td>{device.status === 'offline' ? '—' : `${formatNumber(device.memory)}%`}</td>
          </tr>
        {/each}
      </tbody>
    </table>
    {#if filtered.length === 0}<div class="fleet-empty">No devices match these filters.</div>{/if}
  </div>
{:else if source === 'live'}
  <p class="coming-soon">Live map locations and packet tracing are not configured. Use List View for real machine selection; Demo retains the map and trace previews.</p>
{:else if mode === 'map'}
  <div class="fleet-map-layout">
    <div class="fleet-map-sites">
      <div class="map-sidebar-title"><span class="eyebrow">SITE DIRECTORY</span><strong>THREE OFFICES</strong></div>
      {#each demoSites as site (site.id)}
        <button class:focused={selectedSiteId === site.id} class="site-list-button" onclick={() => selectedSiteId = site.id}>
          <span class={`map-site-indicator ${siteStatus(site.id)}`}></span>
          <strong>{site.name}</strong><small>{siteDeviceCount(site.id)} nodes</small>
        </button>
      {/each}
      {#if selectedSite}
        <div class="site-expansion"><div class="eyebrow">SELECTED SITE</div><h3>{selectedSite.name}</h3><p>{selectedSite.description}</p>
          {#each selectedSiteDevices as device (device.id)}
            <button class:focused={selectedDeviceId === device.id} onclick={() => selectDevice(device.id)}>
              <i class={`map-site-indicator ${device.status}`}></i><span>{device.name}</span><small>{device.status}</small>
            </button>
          {/each}
        </div>
      {:else}
        <p class="site-selection-tip">Select an office to reveal its individual demo endpoints on the map.</p>
      {/if}
      <div class="site-map-summary">◉ {online} ONLINE <span>·</span> ▲ {warnings} WARNING <span>·</span> × {offline} OFFLINE</div>
    </div>
    <ContagionMap {selectedSiteId} {selectedDeviceId}
      selectSite={(siteId: SiteId) => selectedSiteId = siteId}
      {selectDevice} />
  </div>
{:else}
    <NetworkTrace selectedDeviceId={selectedDeviceId} {selectDevice} />
{/if}

<!-- WYNOBSERVE-UI-REPAIR-02 -->
