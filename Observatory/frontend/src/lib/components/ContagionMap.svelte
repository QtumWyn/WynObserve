<script lang="ts">
  import { onMount } from 'svelte';
  import * as L from 'leaflet';
  import 'leaflet/dist/leaflet.css';
  import './contagion-map-v2.css';
  import {
    demoDevices, demoSites, siteDeviceCount, siteStatus,
    type FleetDevice, type FleetSite, type SiteId,
  } from '../data/fleet';

  interface Props {
    selectedSiteId: SiteId | null;
    selectedDeviceId: string | null;
    selectSite: (siteId: SiteId) => void;
    selectDevice: (deviceId: string) => void;
  }

  let { selectedSiteId, selectedDeviceId, selectSite, selectDevice }: Props = $props();
  let mapHost: HTMLDivElement;
  let map: L.Map | null = null;
  let markers: L.Marker[] = [];
  let lastCenteredSite: SiteId | null = null;
  let tileUnavailable = $state<boolean>(false);

  // Visual instrumentation only. The three offices do NOT have a verified
  // inter-site route topology; these guides never represent network traffic.
  let showIllustrativePaths = $state<boolean>(false);
  let scanEnabled = $state<boolean>(true);
  let pointerCoordinates = $state<string>('MOVE POINTER TO INSPECT');
  let pathLayer: L.LayerGroup | null = null;
  const conceptualLinks: [SiteId, SiteId][] = [
    ['river', 'aa'],
    ['aa', 'hurricane'],
  ];

  function redrawIllustrativePaths(): void {
    if (!map) return;
    if (pathLayer) pathLayer.clearLayers();
    if (!showIllustrativePaths || !pathLayer) return;

    for (const [originId, destinationId] of conceptualLinks) {
      const origin: FleetSite | undefined = demoSites.find((site: FleetSite) => site.id === originId);
      const destination: FleetSite | undefined = demoSites.find((site: FleetSite) => site.id === destinationId);
      if (!origin || !destination) continue;

      const isFocused: boolean = selectedSiteId === null
        || origin.id === selectedSiteId
        || destination.id === selectedSiteId;
      L.polyline(
        [[origin.latitude, origin.longitude], [destination.latitude, destination.longitude]],
        {
          className: isFocused ? 'concept-connection concept-connection--focused' : 'concept-connection',
          weight: 2,
          opacity: isFocused ? 0.8 : 0.24,
          dashArray: '4 12',
          interactive: false,
        },
      ).addTo(pathLayer);
    }
  }

  const htmlEntities: Record<string, string> = {
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  };
  const safe = (value: string): string => value.replace(/[&<>"']/g, (character: string): string => htmlEntities[character] ?? character);

  function buildMarker(site: FleetSite): L.DivIcon {
    const expanded: boolean = selectedSiteId === site.id;
    const devices: FleetDevice[] = demoDevices.filter((device: FleetDevice) => device.siteId === site.id);
    const center: string = `<div class="map-site-marker" data-site="${site.id}" data-health="${siteStatus(site.id)}">
      <div class="map-marker-halo"></div><span class="map-marker-center"></span>
      <strong>${safe(site.name)}</strong><small>${devices.filter((device: FleetDevice) => device.status === 'online').length}/${siteDeviceCount(site.id)} ONLINE</small>
      ${expanded ? '<span class="map-marker-expanded">◇ SELECT ENDPOINT</span>' : ''}
    </div>`;
    const orbit: string = expanded ? devices.map((device: FleetDevice, index: number): string => {
      // Screen-space expansion is NOT a claim of device-level geolocation.
      const angle: number = (index / devices.length) * Math.PI * 2 - Math.PI / 2;
      const dx: number = Math.cos(angle) * 112;
      const dy: number = Math.sin(angle) * 93;
      return `<button class="map-endpoint ${device.id === selectedDeviceId ? 'chosen' : ''}"
        data-device="${safe(device.id)}" data-health="${device.status}"
        style="--dx:${dx.toFixed(1)}px;--dy:${dy.toFixed(1)}px;"
        title="${safe(device.name)} · ${device.status}">
        <span class="map-endpoint-dot"></span><span class="map-endpoint-label">${safe(device.name)}</span>
      </button>`;
    }).join('') : '';
    return L.divIcon({
      className: 'fleet-map-icon',
      html: `<div class="map-marker-cluster ${expanded ? 'expanded' : ''}">${center}${orbit}</div>`,
      iconSize: [1, 1],
      iconAnchor: [0, 0],
    });
  }

  function redrawMarkers(): void {
    if (!map) return;
      markers.forEach((marker: L.Marker): void => {
          marker.remove();
      });
    markers = demoSites.map((site: FleetSite): L.Marker => {
      const marker: L.Marker = L.marker([site.latitude, site.longitude], {
        icon: buildMarker(site),
        keyboard: true,
        zIndexOffset: selectedSiteId === site.id ? 600 : 0,
      }).addTo(map!);
      marker.on('click', (event: L.LeafletMouseEvent): void => {
        const element: HTMLElement | null = (event.originalEvent?.target as HTMLElement | undefined)?.closest?.('[data-device]') ?? null;
        const deviceId: string | undefined = element?.dataset.device;
        if (deviceId) {
          selectDevice(deviceId);
        } else {
          selectSite(site.id);
        }
      });
      return marker;
    });
  }

  $effect((): void => {
    // Rebuild the individual endpoint constellation when selection changes.
    const siteId: SiteId | null = selectedSiteId;
    selectedDeviceId;
    if (siteId && siteId !== lastCenteredSite && map) {
      const site: FleetSite | undefined = demoSites.find((item: FleetSite): boolean => item.id === siteId);
      if (site) map.flyTo([site.latitude, site.longitude], 10, { duration: 0.85 });
    }
    lastCenteredSite = siteId;
    redrawMarkers();
    if (showIllustrativePaths) redrawIllustrativePaths();
    else pathLayer?.clearLayers();
  });

  onMount((): (() => void) => {
    map = L.map(mapHost, {
      zoomControl: false,
      attributionControl: true,
      scrollWheelZoom: true,
      minZoom: 5,
      maxZoom: 13,
      preferCanvas: true,
    });
    map.fitBounds([[36.2, -90.0], [42.0, -77.1]], { padding: [24, 24] });
    L.control.zoom({ position: 'bottomright' }).addTo(map);

      const tiles: L.TileLayer = L.tileLayer(
          'https://tile.openstreetmap.org/{z}/{x}/{y}.png',
          {
              attribution:
                  '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap contributors</a>',

              maxZoom: 19,

              tileSize: 256,

              crossOrigin: true,
          },
      );
    tiles.on('tileerror', (): void => { tileUnavailable = true; });
    tiles.addTo(map);
    pathLayer = L.layerGroup().addTo(map);
    map.on('mousemove', (event: L.LeafletMouseEvent): void => {
      pointerCoordinates = `${event.latlng.lat.toFixed(3)}° N  /  ${Math.abs(event.latlng.lng).toFixed(3)}° ${event.latlng.lng < 0 ? 'W' : 'E'}`;
    });
    map.on('mouseout', (): void => {
      pointerCoordinates = 'MOVE POINTER TO INSPECT';
    });
    redrawIllustrativePaths();
    redrawMarkers();

    // Tauri's initial layout may settle after the map mounts.
      const resize: number = window.setTimeout((): void => {
          map?.invalidateSize();
      }, 200);
    return (): void => {
      window.clearTimeout(resize);
      markers = [];
      pathLayer?.clearLayers();
      pathLayer = null;
      map?.remove();
      map = null;
    };
  });

  function resetMap(): void {
    map?.flyToBounds([[36.2, -90.0], [42.0, -77.1]], { padding: [24, 24], duration: 0.85 });
  }
</script>

<div class="contagion-map" aria-label="Interactive regional map of Kentucky, Ohio and West Virginia">
  <div bind:this={mapHost} class="contagion-map-canvas"></div>
  <div class="map-screen-grid" aria-hidden="true"></div>
  <div class="map-sweep" class:map-sweep--active={scanEnabled} aria-hidden="true"></div>
  <div class="map-crosshair" aria-hidden="true"><i></i><i></i></div>
  <div class="map-corner top-left">
    <span class="map-mode-label">◈ REGIONAL SIGNAL NETWORK</span>
    <strong>CONTAGION <i>MAP</i></strong>
    <small>KY / OH / WV · DEMONSTRATION</small>
    <span class="map-hud-count">{demoSites.length} DEMO SITES <b>·</b> {demoDevices.length} DEMO ENDPOINTS</span>
  </div>
  <div class="map-corner top-right">
    <div class="map-hud-controls" role="group" aria-label="Map visualization controls">
      <button
        class:map-toggle-active={showIllustrativePaths}
        aria-pressed={showIllustrativePaths}
        onclick={() => showIllustrativePaths = !showIllustrativePaths}
        title="Visual guides only. These do not represent live links between office networks."
      >◇ DEMO PATHS {showIllustrativePaths ? 'ON' : 'OFF'}</button>
      <button
        class:map-toggle-active={scanEnabled}
        aria-pressed={scanEnabled}
        onclick={() => scanEnabled = !scanEnabled}
        title="Toggle decorative radar sweep"
      >◎ SWEEP {scanEnabled ? 'ON' : 'OFF'}</button>
      <button onclick={resetMap} class="map-reset">⤢ REGIONAL VIEW</button>
    </div>
    <span class="map-coordinate-label">POINTER // {pointerCoordinates}</span>
  </div>
  <div class="map-map-legend">
    <span><i class="map-legend-dot online"></i> ONLINE</span>
    <span><i class="map-legend-dot warning"></i> WARNING</span>
    <span><i class="map-legend-dot offline"></i> OFFLINE</span>
  </div>
  {#if tileUnavailable}
    <div class="map-tile-warning">Basemap tiles unavailable. Internet access may be required for the geographic layer.</div>
  {/if}
  <div class="map-watermark">
    ◇ SYNTHETIC SITES / HEALTH / COORDINATES · NO VERIFIED DEVICE LOCATIONS
    {#if showIllustrativePaths}<span> · DEMO PATHS ARE NOT REAL NETWORK LINKS</span>{/if}
  </div>
</div>
