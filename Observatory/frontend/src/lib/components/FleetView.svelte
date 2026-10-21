<script lang="ts">
  import ContagionMap from './ContagionMap.svelte';
  import {
    demoDevices, demoSites, siteDeviceCount, siteStatus,
    type FleetDevice, type FleetSite, type FleetStatus, type SiteId,
  } from '../data/fleet';

  interface Props {
    selectedDeviceId: string | null;
    selectDevice: (deviceId: string) => void;
  }
  let { selectedDeviceId, selectDevice }: Props = $props();
  let mode = $state<'list' | 'map'>('list');
  let selectedSiteId = $state<SiteId | null>(null);
  let query = $state<string>('');
  let statusFilter = $state<'all' | FleetStatus>('all');
  let siteFilter = $state<'all' | SiteId>('all');

  let filtered = $derived(demoDevices.filter((device: FleetDevice): boolean => {
    const matchesQuery: boolean = [device.name, device.role, device.os, device.siteId]
      .some((value: string): boolean => value.toLowerCase().includes(query.toLowerCase()));
    return matchesQuery && (statusFilter === 'all' || device.status === statusFilter)
      && (siteFilter === 'all' || device.siteId === siteFilter);
  }));

  let online = $derived(demoDevices.filter((device: FleetDevice) => device.status === 'online').length);
  let warnings = $derived(demoDevices.filter((device: FleetDevice) => device.status === 'warning').length);
  let offline = $derived(demoDevices.filter((device: FleetDevice) => device.status === 'offline').length);
  let selectedSite = $derived(demoSites.find((site: FleetSite) => site.id === selectedSiteId));
  let selectedSiteDevices = $derived(demoDevices.filter((device: FleetDevice) => device.siteId === selectedSiteId));
</script>

<div class="page-heading fleet-page-heading">
  <div><div class="eyebrow">CORE / FLEET / 003</div><h1>THE <em>FLEET</em></h1>
    <p>Observe endpoint presence, group devices by office and inspect a regional signal map.</p></div>
  <div class="fleet-view-switch" role="group" aria-label="Fleet display mode">
    <button class:chosen={mode === 'list'} onclick={() => mode = 'list'}>☷ LIST VIEW</button>
    <button class:chosen={mode === 'map'} onclick={() => mode = 'map'}>◈ CONTAGION MAP</button>
  </div>
</div>

<div class="fleet-banner"><span>◇ DEMONSTRATION DATA</span><p>All endpoints, statuses, and office map coordinates on this page are synthetic preview data. No live Fleet connection or verified site geolocation.</p></div>

<div class="fleet-count-row">
  <div><small>ENROLLED</small><strong>{demoDevices.length}</strong></div>
  <div><small>ONLINE</small><strong class="green">{online}</strong></div>
  <div><small>WARNING</small><strong class="amber">{warnings}</strong></div>
  <div><small>OFFLINE</small><strong class="red">{offline}</strong></div>
  <div><small>OFFICE SITES</small><strong>{demoSites.length}</strong></div>
</div>

{#if mode === 'list'}
  <div class="fleet-filter-row">
    <input aria-label="Search devices" placeholder="⌕ Search devices, OS or office..." bind:value={query} />
    <select aria-label="Office filter" bind:value={siteFilter}>
      <option value="all">ALL OFFICES</option>
      {#each demoSites as site (site.id)}<option value={site.id}>{site.name}</option>{/each}
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
            <td>{demoSites.find((site: FleetSite) => site.id === device.siteId)?.name}</td>
            <td>{device.os}</td><td>{device.agentVersion}</td><td>{device.lastSeen}</td>
            <td>{device.status === 'offline' ? '—' : `${device.cpu}%`}</td>
            <td>{device.status === 'offline' ? '—' : `${device.memory}%`}</td>
          </tr>
        {/each}
      </tbody>
    </table>
    {#if filtered.length === 0}<div class="fleet-empty">No demo devices match these filters.</div>{/if}
  </div>
{:else}
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
{/if}
