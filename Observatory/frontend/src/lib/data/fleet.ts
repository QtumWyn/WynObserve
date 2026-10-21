/** DEMO ONLY. These sites and their coordinates are illustrative, NOT verified A Plus locations.
 * Replace them with authorized, intentionally configured site coordinates before live use.
 * Never derive endpoint locations from IP addresses or assume all machines have GPS coordinates.
 */
export type FleetStatus = 'online' | 'warning' | 'offline';
export type SiteId = 'river' | 'aa' | 'hurricane';

export interface FleetDevice {
  id: string;
  name: string;
  siteId: SiteId;
  role: string;
  os: string;
  status: FleetStatus;
  agentVersion: string;
  lastSeen: string;
  cpu: number;
  memory: number;
}

export interface FleetSite {
  id: SiteId;
  name: string;
  description: string;
  // DEMONSTRATION COORDINATES. Do not interpret them as physical office addresses.
  latitude: number;
  longitude: number;
}

export const demoSites: FleetSite[] = [
  { id: 'river', name: 'THE RIVER', description: 'Primary command site', latitude: 38.72, longitude: -82.97 },
  { id: 'aa', name: 'THE AA', description: 'Operations site', latitude: 38.50, longitude: -82.45 },
  { id: 'hurricane', name: 'HURRICANE', description: 'Service site', latitude: 38.49, longitude: -81.92 },
];

export const demoDevices: FleetDevice[] = [
  { id:'wyn-itpc', name:'WYN-ITPC', siteId:'river', role:'Workstation', os:'Kubuntu', status:'online', agentVersion:'0.2.0', lastSeen:'4s ago', cpu:42, memory:67 },
  { id:'river-r640', name:'RIVER-R640', siteId:'river', role:'Server', os:'Linux', status:'online', agentVersion:'0.2.0', lastSeen:'2s ago', cpu:19, memory:51 },
  { id:'eevee-02', name:'EEVEE-02', siteId:'river', role:'Workstation', os:'Kubuntu', status:'online', agentVersion:'0.2.0', lastSeen:'8s ago', cpu:37, memory:58 },
  { id:'river-test', name:'RIVER-LAB', siteId:'river', role:'Test endpoint', os:'Linux', status:'online', agentVersion:'0.2.0', lastSeen:'27s ago', cpu:83, memory:76 },
  { id:'aa-01', name:'AA-DESK-01', siteId:'aa', role:'Workstation', os:'Windows', status:'online', agentVersion:'0.2.0', lastSeen:'6s ago', cpu:22, memory:31 },
  { id:'aa-02', name:'AA-DESK-02', siteId:'aa', role:'Workstation', os:'Windows', status:'online', agentVersion:'0.2.0', lastSeen:'9s ago', cpu:14, memory:26 },
  { id:'aa-03', name:'AA-OPS-03', siteId:'aa', role:'Workstation', os:'Linux', status:'offline', agentVersion:'0.2.0', lastSeen:'1h ago', cpu:0, memory:0 },
  { id:'aa-04', name:'AA-SVC-04', siteId:'aa', role:'Workstation', os:'Windows', status:'online', agentVersion:'0.2.0', lastSeen:'3s ago', cpu:58, memory:48 },
  { id:'hurr-srv', name:'HURR-SRV-01', siteId:'hurricane', role:'Server', os:'Linux', status:'online', agentVersion:'0.2.0', lastSeen:'5s ago', cpu:18, memory:42 },
  { id:'hurr-01', name:'HURR-WS-01', siteId:'hurricane', role:'Workstation', os:'Windows', status:'warning', agentVersion:'0.2.0', lastSeen:'38s ago', cpu:71, memory:77 },
  { id:'hurr-02', name:'HURR-WS-02', siteId:'hurricane', role:'Workstation', os:'Windows', status:'online', agentVersion:'0.2.0', lastSeen:'11s ago', cpu:24, memory:33 },
];

export function siteStatus(siteId: SiteId): FleetStatus {
  const devices: FleetDevice[] = demoDevices.filter((device: FleetDevice) => device.siteId === siteId);
  if (devices.every((device: FleetDevice) => device.status === 'offline')) return 'offline';
  if (devices.some((device: FleetDevice) => device.status !== 'online')) return 'warning';
  return 'online';
}

export function siteDeviceCount(siteId: SiteId): number {
  return demoDevices.filter((device: FleetDevice) => device.siteId === siteId).length;
}
