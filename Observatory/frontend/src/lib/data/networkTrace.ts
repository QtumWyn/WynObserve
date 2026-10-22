/**
 * WynObserve // network activity visualization contracts.
 *
 * This file is entirely illustrative. Nothing below is packet capture,
 * traffic telemetry, discovered topology, or a claim about A Plus networks.
 * The eventual live adapter must explicitly supply an observed data source.
 */
import { demoDevices, type FleetDevice, type SiteId } from './fleet';

export interface TraceNode {
  device: FleetDevice;
  x: number;
  y: number;
}

export interface TraceEvent {
  id: string;
  deviceId: string;
  deviceName: string;
  siteId: SiteId;
  protocol: string;
  direction: 'EGRESS' | 'INGRESS' | 'LOCAL';
  relativeSeconds: number;
  amountKiB: number;
  simulated: true;
}

const positions = [
  { x: 16, y: 23 },
  { x: 84, y: 23 },
  { x: 16, y: 77 },
  { x: 84, y: 77 },
  { x: 50, y: 91 },
  { x: 50, y: 11 },
];
const protocols: string[] = ['HTTPS', 'DNS', 'SMB', 'QUIC', 'SSH', 'RDP'];
const directions: TraceEvent['direction'][] = ['EGRESS', 'INGRESS', 'LOCAL'];

export function traceNodes(siteId: SiteId): TraceNode[] {
  return demoDevices.filter((device: FleetDevice) => device.siteId === siteId).map(
    (device: FleetDevice, index: number): TraceNode => ({
      device,
      x: positions[index % positions.length].x,
      y: positions[index % positions.length].y,
    }),
  );
}

export function simulatedActivity(deviceId: string, step: number): number {
  const device: FleetDevice | undefined = demoDevices.find((value: FleetDevice) => value.id === deviceId);
  if (!device || device.status === 'offline') return 0;
  const seed: number = [...deviceId].reduce((sum: number, letter: string) => sum + letter.charCodeAt(0), 0);
  return 4 + ((seed * 7 + step * 13 + Math.floor(step / 3) * 17) % 79);
}

export function traceEvents(siteId: SiteId, step: number): TraceEvent[] {
  const devices: FleetDevice[] = demoDevices.filter((device: FleetDevice) => device.siteId === siteId && device.status !== 'offline');
  const events: TraceEvent[] = [];
  for (let offset: number = 0; offset < 6; offset += 1) {
    if (devices.length === 0) break;
    const device: FleetDevice = devices[(step + offset) % devices.length];
    const protocol: string = protocols[(step + offset * 3 + device.name.length) % protocols.length];
    const direction: TraceEvent['direction'] = directions[(step + offset) % directions.length];
    events.push({
      id: `${siteId}-${device.id}-${step}-${offset}`,
      deviceId: device.id,
      deviceName: device.name,
      siteId,
      protocol,
      direction,
      relativeSeconds: offset * 2,
      amountKiB: 6 + (simulatedActivity(device.id, step - offset) * (offset + 3)) % 420,
      simulated: true,
    });
  }
  return events;
}
