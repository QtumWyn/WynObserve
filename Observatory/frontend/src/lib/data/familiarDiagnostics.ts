/**
 * Presentation-only reactions for Observatory familiars.
 * The active Svelte frontend supplies synthetic data. These conditions must
 * never be presented as live incidents until a trusted Hub data source is wired.
 * Thresholds for new reactions are illustrative UI choices, not alert policy.
 */
import type { FleetDevice } from './fleet';
import type { TelemetryFrame } from './telemetry';

export type FamiliarKind = 'cat' | 'bat' | 'bunny' | 'puppy';
export type FamiliarMood = 'idle' | 'curious' | 'worried' | 'sleeping' | 'playing' | 'happy';
export type FamiliarSignal = { active: boolean; message: string; detail: string };
export type FamiliarSignals = Record<FamiliarKind, FamiliarSignal>;

export function familiarSignals(
  frame: TelemetryFrame,
  devices: ReadonlyArray<Pick<FleetDevice, 'status'>>,
  incidentActive: boolean = false,
): FamiliarSignals {
  const traffic: number = frame.network.rxMiBs + frame.network.txMiBs;
  const thermal: boolean = frame.cpu.temperatureC >= 78 || frame.gpu.temperatureC >= 80;
  const memoryPressure: boolean = frame.memory.usedPercent >= 88;
  const offlineCount: number = devices.filter((device) => device.status === 'offline').length;

  return {
    cat: {
      active: traffic >= 18,
      message: 'Packets! ✧',
      detail: `Network traffic: ${traffic.toFixed(1)} MiB/s combined`,
    },
    bat: {
      active: thermal,
      message: 'Too warm! ♨',
      detail: `CPU ${frame.cpu.temperatureC.toFixed(1)}°C · GPU ${frame.gpu.temperatureC.toFixed(1)}°C`,
    },
    bunny: {
      active: incidentActive || memoryPressure,
      message: incidentActive ? 'An incident? ♡' : 'So much memory…',
      detail: incidentActive ? 'An incident has been flagged' : `Memory pressure: ${frame.memory.usedPercent.toFixed(1)}%`,
    },
    puppy: {
      active: offlineCount > 0,
      message: 'Where did they go? !',
      detail: `${offlineCount} endpoint${offlineCount === 1 ? '' : 's'} offline`,
    },
  };
}
