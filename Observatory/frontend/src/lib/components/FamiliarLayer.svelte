<script lang="ts">
  import { onMount } from 'svelte';
  import type { Component, TelemetryFrame } from '../data/telemetry';
  import type { FleetDevice } from '../data/fleet';
  import {
    familiarSignals,
    type FamiliarKind,
    type FamiliarMood,
  } from '../data/familiarDiagnostics';

  interface FamiliarVisibility {
    catgirl: boolean;
    batgirl: boolean;
    bunnygirl: boolean;
    puppygirl: boolean;
  }

  const visibilityKeys: Record<FamiliarKind, keyof FamiliarVisibility> = {
    cat: 'catgirl', bat: 'batgirl', bunny: 'bunnygirl', puppy: 'puppygirl',
  };
  const visibilityStorageKey: string = 'wynobserve.familiars.visibility.v1';

  interface Props {
    frame: TelemetryFrame;
    /** When available, use the original Observatory preferences without modifying them. */
    preferences?: FamiliarVisibility;
    devices: ReadonlyArray<FleetDevice>;
    /** Explicit provenance controls simulation-only interactions. */
    source?: 'demo' | 'observed';
    /** Leave false until connected to a real incident signal. */
    incidentActive?: boolean;
    selected?: Component;
  }

  let {
    frame,
    preferences: suppliedPreferences = undefined,
    devices,
    source = 'demo',
    incidentActive = false,
    selected = 'CPU',
  }: Props = $props();

  interface CreatureMotion {
    x: number;
    y: number;
    vx: number;
    vy: number;
    initialized: boolean;
    hovered: boolean;
    fedUntil: number;
    sleepAnchorX: number;
    sleepAnchorY: number;
    sleeping: boolean;
    chasing: boolean;
    mood: FamiliarMood;
  }

  const kinds: FamiliarKind[] = ['cat', 'bat', 'bunny', 'puppy'];
  const labels: Record<FamiliarKind, string> = {
    cat: 'Cat familiar',
    bat: 'Bat familiar',
    bunny: 'Bunny familiar',
    puppy: 'Puppy familiar',
  };
  const chaseCycles: Record<FamiliarKind, [number, number, number]> = {
    cat: [17, 5, 7],
    bat: [13, 4, 8],
    bunny: [19, 5, 10],
    puppy: [11, 6, 6],
  };
  const sleepCycles: Record<FamiliarKind, [number, number, number]> = {
    cat: [47, 6.5, 11],
    bat: [53, 7, 19],
    bunny: [43, 6, 8],
    puppy: [58, 8, 22],
  };

  function initialMotion(): CreatureMotion {
    return {
      x: 0, y: 0, vx: 0, vy: 0,
      initialized: false, hovered: false, fedUntil: 0,
      sleepAnchorX: 0, sleepAnchorY: 0,
      sleeping: false, chasing: false, mood: 'idle',
    };
  }

  let motions = $state<Record<FamiliarKind, CreatureMotion>>({
    cat: initialMotion(),
    bat: initialMotion(),
    bunny: initialMotion(),
    puppy: initialMotion(),
  });
  // Self-contained fallback until Settings Migration 01 is installed.
  // Only the four familiar toggles are saved, never telemetry or backend settings.
  let localVisibility = $state<FamiliarVisibility>({
    catgirl: true, batgirl: true, bunnygirl: false, puppygirl: false,
  });
  let preferences = $derived(suppliedPreferences ?? localVisibility);
  let refreshMotionLoop: (() => void) | null = null;

  function setEnabled(kind: FamiliarKind, enabledValue: boolean): void {
    if (suppliedPreferences) return;
    localVisibility = { ...localVisibility, [visibilityKeys[kind]]: enabledValue };
    try {
      window.localStorage.setItem(visibilityStorageKey, JSON.stringify(localVisibility));
    } catch {
      // Storage may be disabled; keep working with this session's settings.
    }
    // If the last familiar was disabled, cancel the pending animation frame.
    // Re-enabling one starts the loop immediately.
    refreshMotionLoop?.();
  }

  // Also honor changes from the optional shared settings migration.
  $effect((): void => {
    void preferences.catgirl; void preferences.batgirl;
    void preferences.bunnygirl; void preferences.puppygirl;
    refreshMotionLoop?.();
  });

  type DemoReaction = FamiliarKind | 'auto';
  const demoChoices: DemoReaction[] = ['auto', ...kinds];
  let demoReaction = $state<DemoReaction>('auto');
  let playroomOpen = $state<boolean>(false);
  let ready = $state<boolean>(false);
  let width = $state<number>(700);
  let height = $state<number>(530);
  let reducedMotion = $state<boolean>(false);
  let announcement = $state<string>('');
  let host: HTMLDivElement;
  let signals = $derived.by(() => {
    const actual = familiarSignals(frame, devices, incidentActive);
    if (source !== 'demo' || demoReaction === 'auto') return actual;
    return {
      ...actual,
      [demoReaction]: {
        active: true,
        message: demoReaction === 'cat' ? 'Packets! ✧'
          : demoReaction === 'bat' ? 'Too warm! ♨'
          : demoReaction === 'bunny' ? 'An incident? ♡' : 'Where did they go? !',
        detail: 'MANUAL DEMO · no telemetry or fleet state changed',
      },
    };
  });

  function enabled(kind: FamiliarKind): boolean {
    switch (kind) {
      case 'cat': return preferences.catgirl;
      case 'bat': return preferences.batgirl;
      case 'bunny': return preferences.bunnygirl;
      case 'puppy': return preferences.puppygirl;
    }
  }

  function clamp(value: number, low: number, high: number): number {
    return Math.min(Math.max(value, low), Math.max(low, high));
  }

  function smoothstep(value: number): number {
    const x: number = clamp(value, 0, 1);
    return x * x * (3 - 2 * x);
  }

  function intervalActivity(t: number, cycle: [number, number, number]): boolean {
    const [period, duration, phase]: [number, number, number] = cycle;
    const local: number = (t + phase) % period;
    return local > 0.55 && local < duration - 0.55;
  }

  function position(kind: FamiliarKind, t: number, w: number, h: number, alert: boolean): [number, number] {
    const s: (value: number) => number = (value: number): number => smoothstep(value);
    const lane: number = 0.05 * Math.sin(t * 0.071) + 0.025 * Math.sin(t * 0.137 + 1.4);
    if (kind === 'cat') {
      const wave: number = (t * (alert ? 0.16 : 0.095)) % 1;
      const triangle: number = wave < 0.5 ? wave * 2 : (1 - wave) * 2;
      return [w * (0.08 + lane + 0.26 * s(triangle)), h - (selected === 'NETWORK' ? 75 : 60) + 4 * Math.sin(t * 0.21)];
    }
    if (kind === 'bat') {
      return [
        w * (0.5 + 0.27 * Math.sin(t * 0.19) + 0.1 * Math.sin(t * 0.43 + 1.7)),
        h * (0.32 + 0.15 * Math.sin(t * (alert ? 0.42 : 0.23) + 2.2)
          + 0.07 * Math.sin(t * 0.57 + 0.3)),
      ];
    }
    if (kind === 'bunny') {
      if (incidentActive) return [w - 86, h - 70];
      const phase: number = (t * (alert ? 0.65 : 0.42)) % 1;
      const backwards: boolean = Math.floor(t * (alert ? 0.65 : 0.42)) % 2 === 1;
      const step: number = backwards ? 1 - phase : phase;
      const hop: number = Math.max(0, Math.sin((phase * 3 % 1) * Math.PI));
      return [w * (0.54 + 0.32 * s(step)), h - 65 - 16 * hop];
    }
    if (alert) return [Math.min(w * 0.18, 115), h - 60];
    const phase: number = (t * 0.14) % 1;
    const triangle: number = phase < 0.5 ? 2 * phase : 2 * (1 - phase);
    return [w * (0.28 + 0.44 * s(triangle)), h - 59 - Math.abs(Math.sin(t * 7.2)) * 3];
  }

  function spriteMood(kind: FamiliarKind): FamiliarMood {
    return motions[kind].mood;
  }

  function spriteLabel(kind: FamiliarKind): string {
    const reaction: string = signals[kind].active ? ` · ${signals[kind].detail} (${source})` : '';
    return `${labels[kind]} · click to feed${reaction}`;
  }

  function feed(kind: FamiliarKind): void {
    motions[kind].fedUntil = performance.now() / 1000 + 3.6;
    motions[kind].sleeping = false;
    motions[kind].vy = -95;
    motions[kind].mood = 'happy';
    announcement = `${labels[kind]} got a treat!`;
  }

  // The reduced-motion preference disables RAF but still updates emotions when
  // incoming metrics or the user's interactions change.
  $effect((): void => {
    if (!reducedMotion) return;
    for (const kind of kinds) {
      const motion: CreatureMotion = motions[kind];
      motion.mood = performance.now() / 1000 < motion.fedUntil ? 'happy'
        : motion.hovered ? 'curious'
        : signals[kind].active ? (kind === 'cat' ? 'curious' : 'worried') : 'idle';
    }
  });

  onMount((): () => void => {
    if (!suppliedPreferences) {
      try {
        const saved: string | null = window.localStorage.getItem(visibilityStorageKey);
        if (saved) {
          const parsed: unknown = JSON.parse(saved);
          if (parsed && typeof parsed === 'object') {
            const result = parsed as Partial<FamiliarVisibility>;
            localVisibility = {
              catgirl: typeof result.catgirl === 'boolean' ? result.catgirl : true,
              batgirl: typeof result.batgirl === 'boolean' ? result.batgirl : true,
              bunnygirl: typeof result.bunnygirl === 'boolean' ? result.bunnygirl : false,
              puppygirl: typeof result.puppygirl === 'boolean' ? result.puppygirl : false,
            };
          }
        }
      } catch {
        // Malformed or unavailable localStorage: default to cat and bat.
      }
    }
    const main: HTMLElement | null = host.closest('main.content');
    if (!main) return () => {};

    const mouse: { x: number; y: number; inside: boolean } = { x: 0, y: 0, inside: false };
    let raf: number = 0;
    let visible: boolean = document.visibilityState === 'visible';
    let last: number = performance.now();
    let lastRendered: number = 0;
    const hasVisibleFamiliar = (): boolean => kinds.some((kind: FamiliarKind) => enabled(kind));
    const start: number = performance.now();
    const media: MediaQueryList = window.matchMedia('(prefers-reduced-motion: reduce)');

    const updateSize = (): void => {
      const style: CSSStyleDeclaration = window.getComputedStyle(main);
      const horizontal: number = parseFloat(style.paddingLeft) + parseFloat(style.paddingRight);
      const vertical: number = parseFloat(style.paddingTop) + parseFloat(style.paddingBottom);
      width = Math.max(150, main.clientWidth - horizontal);
      height = Math.max(155, main.clientHeight - vertical);
    };
    const resize: ResizeObserver = new ResizeObserver(updateSize);
    resize.observe(main);
    updateSize();

    const updateMotionPreference = (): void => {
      reducedMotion = media.matches;
      if (reducedMotion) {
        // Freeze motion in a predictable location; feeding and focus remain available.
        for (const kind of kinds) {
          motions[kind].initialized = false;
          motions[kind].sleeping = false;
          motions[kind].chasing = false;
        }
      }
    };
    media.addEventListener('change', updateMotionPreference);
    updateMotionPreference();

    const trackPointer = (event: PointerEvent): void => {
      const rect: DOMRect = host.getBoundingClientRect();
      mouse.x = event.clientX - rect.left;
      mouse.y = event.clientY - rect.top;
      mouse.inside = true;
    };
    const leavePointer = (): void => { mouse.inside = false; };
    main.addEventListener('pointermove', trackPointer, { passive: true });
    main.addEventListener('pointerleave', leavePointer);

    const tick = (now: number): void => {
      // Cap reactive sprite movement at 40Hz even on a 200Hz display.
      // The visual movements still interpolate using elapsed time.
      if (!reducedMotion && now - lastRendered < 25) {
        if (visible && hasVisibleFamiliar()) raf = window.requestAnimationFrame(tick);
        return;
      }
      lastRendered = now;
      const dt: number = Math.min(Math.max((now - last) / 1000, 0), 0.05);
      last = now;
      const t: number = (now - start) / 1000;

      for (const kind of kinds) {
        if (!enabled(kind)) continue;
        const motion: CreatureMotion = motions[kind];
        const alert: boolean = signals[kind].active;
        const fed: boolean = now / 1000 < motion.fedUntil;
        const chasing: boolean = !reducedMotion && !alert && !fed && !motion.hovered
          && mouse.inside && intervalActivity(t, chaseCycles[kind]);
        const sleeping: boolean = !reducedMotion && !alert && !fed && !motion.hovered
          && !chasing && intervalActivity(t, sleepCycles[kind]);
        const [baseX, baseY]: [number, number] = position(kind, reducedMotion ? 0 : t, width, height, alert);

        if (!motion.initialized) {
          motion.x = clamp(baseX, 42, width - 42);
          motion.y = clamp(baseY, 45, height - 42);
          motion.initialized = true;
          motion.sleepAnchorX = motion.x;
          motion.sleepAnchorY = motion.y;
        }
        if (sleeping && !motion.sleeping) {
          motion.sleepAnchorX = kind === 'bat' ? width - 83 : motion.x;
          motion.sleepAnchorY = kind === 'bat' ? 67 : height - 59;
          motion.vx = 0;
          motion.vy = 0;
        }
        motion.sleeping = sleeping;
        motion.chasing = chasing;

        let x: number = sleeping ? motion.sleepAnchorX : baseX;
        let y: number = sleeping ? motion.sleepAnchorY : baseY;
        if (chasing) {
          if (kind === 'bat') {
            x = x * 0.25 + mouse.x * 0.75;
            y = y * 0.25 + (mouse.y - 16) * 0.75;
          } else {
            x = x * 0.4 + mouse.x * 0.6;
          }
        }
        x = clamp(x, 42, width - 42);
        y = clamp(y, 45, height - 42);

        if (reducedMotion) {
          motion.x = x;
          motion.y = y;
          motion.vx = 0;
          motion.vy = 0;
        } else {
          const spring: number = sleeping ? 10 : chasing ? 30 : kind === 'bat' ? 17 : 14;
          const damping: number = sleeping ? 11 : chasing ? 8 : 9.5;
          const damp: number = Math.exp(-damping * dt);
          motion.vx = (motion.vx + (x - motion.x) * spring * dt) * damp;
          motion.vy = (motion.vy + (y - motion.y) * spring * dt) * damp;
          const magnitude: number = Math.hypot(motion.vx, motion.vy);
          const max: number = chasing ? 540 : 350;
          if (magnitude > max) {
            motion.vx = motion.vx / magnitude * max;
            motion.vy = motion.vy / magnitude * max;
          }
          motion.x = clamp(motion.x + motion.vx * dt, 42, width - 42);
          motion.y = clamp(motion.y + motion.vy * dt, 45, height - 42);
        }

        motion.mood = fed ? 'happy' : motion.hovered ? 'curious'
          : alert ? (kind === 'cat' ? 'curious' : 'worried')
          : sleeping ? 'sleeping' : chasing ? 'playing' : 'idle';
      }
      ready = true;
      if (visible && !reducedMotion && hasVisibleFamiliar()) raf = window.requestAnimationFrame(tick);
    };

    const startTicking = (): void => {
      window.cancelAnimationFrame(raf);
      last = performance.now();
      ready = true;
      if (visible && !reducedMotion && hasVisibleFamiliar()) raf = window.requestAnimationFrame(tick);
      else if (reducedMotion) tick(last);
    };
    refreshMotionLoop = startTicking;
    const updateVisibility = (): void => {
      visible = document.visibilityState === 'visible';
      startTicking();
    };
    const onMediaChange = (): void => startTicking();
    media.addEventListener('change', onMediaChange);
    document.addEventListener('visibilitychange', updateVisibility);
    startTicking();

    return (): void => {
      refreshMotionLoop = null;
      window.cancelAnimationFrame(raf);
      resize.disconnect();
      media.removeEventListener('change', updateMotionPreference);
      media.removeEventListener('change', onMediaChange);
      document.removeEventListener('visibilitychange', updateVisibility);
      main.removeEventListener('pointermove', trackPointer);
      main.removeEventListener('pointerleave', leavePointer);
    };
  });
</script>

<div class="familiar-layer" bind:this={host} style:height={`${height}px`}>
  {#if source === 'demo'}
      <div class="familiar-lab">
        {#if playroomOpen}
          <div class="familiar-lab-menu">
            <span>REACTION PREVIEW · DEMO ONLY</span>
            <small>Overrides a familiar's expression, not device telemetry.</small>
            <div class="familiar-lab-actions">
              {#each demoChoices as item (item)}
                <button type="button" class:chosen={demoReaction === item}
                  onclick={() => demoReaction = item}>
                  {item.toUpperCase()}
                </button>
              {/each}
            </div>
            {#if !suppliedPreferences}
              <span class="familiar-visibility-heading">VISIBLE FAMILIARS</span>
              <div class="familiar-visibility-controls">
                {#each kinds as kind (kind)}
                  <label>
                    <input type="checkbox" checked={enabled(kind)}
                      onchange={(event: Event) => setEnabled(kind, (event.currentTarget as HTMLInputElement).checked)} />
                    {kind.toUpperCase()}
                  </label>
                {/each}
              </div>
            {:else}
              <small>Familiar visibility is controlled in Observatory Settings.</small>
            {/if}
          </div>
        {/if}
        <button class="familiar-lab-toggle" type="button"
          aria-expanded={playroomOpen}
          onclick={() => playroomOpen = !playroomOpen}>
          ◇ FAMILIARS / DEMO {playroomOpen ? '▴' : '▾'}
        </button>
      </div>
  {:else if preferences.catgirl || preferences.batgirl || preferences.bunnygirl || preferences.puppygirl}
      <span class="familiar-layer-source" aria-hidden="true">FAMILIAR SIGNALS / OBSERVED</span>
  {/if}
  <span class="familiar-screenreader" aria-live="polite">{announcement}</span>
  {#each kinds as kind (kind)}
    {#if ready && enabled(kind)}
      <button
        type="button"
        class="familiar-sprite"
        class:fam-cat={kind === 'cat'}
        class:fam-bat={kind === 'bat'}
        class:fam-bunny={kind === 'bunny'}
        class:fam-puppy={kind === 'puppy'}
        class:is-sleeping={spriteMood(kind) === 'sleeping'}
        class:is-happy={spriteMood(kind) === 'happy'}
        class:is-alert={signals[kind].active && spriteMood(kind) !== 'happy'}
        class:is-playing={spriteMood(kind) === 'playing'}
        class:is-hovered={motions[kind].hovered}
        style:left="0px"
        style:top="0px"
        style:transform={`translate3d(${motions[kind].x}px, ${motions[kind].y}px, 0) translate(-50%, -50%)`}
        aria-label={spriteLabel(kind)}
        title={spriteLabel(kind)}
        onclick={() => feed(kind)}
        onpointerenter={() => motions[kind].hovered = true}
        onpointerleave={() => motions[kind].hovered = false}
        onfocus={() => motions[kind].hovered = true}
        onblur={() => motions[kind].hovered = false}
      >
        {#if kind === 'cat'}
          <svg viewBox="0 0 112 100" aria-hidden="true" class="familiar-figure">
            <ellipse class="fam-shadow" cx="53" cy="89" rx="34" ry="5" />
            <path class="fam-line fam-tail" d="M29 64 Q 7 49 18 27 Q 26 17 27 26" />
            <ellipse class="fam-body" cx="51" cy="66" rx="26" ry="19" />
            <path class="fam-paw" d="M42 81 Q36 90 29 86 M61 81 Q68 90 76 86" />
            <circle class="fam-face" cx="65" cy="45" r="21" />
            <path class="fam-ear" d="M47 30 L45 9 L61 25 M72 24 L86 9 L86 36" />
            <path class="fam-inner-ear" d="M50 21 L53 28 M80 21 L77 28" />
            {#if spriteMood(kind) === 'sleeping'}
              <path class="fam-eye-line" d="M54 44 Q59 49 64 44 M72 44 Q77 49 82 44" />
            {:else if spriteMood(kind) === 'worried'}
              <path class="fam-eye-line" d="M55 39 L62 41 M76 40 L82 38" />
              <circle class="fam-eye" cx="59" cy="45" r="2.6" /><circle class="fam-eye" cx="77" cy="45" r="2.6" />
            {:else}
              <ellipse class="fam-eye" cx="58" cy="45" rx="2.6" ry="3.2" /><ellipse class="fam-eye" cx="78" cy="45" rx="2.6" ry="3.2" />
            {/if}
            <path class="fam-nose" d="M66 52 L70 52 L68 55 Z" />
            <path class="fam-mouth" d={spriteMood(kind) === 'happy' ? 'M68 55 Q62 67 57 58 M68 55 Q75 67 80 58' : 'M68 55 Q64 60 61 57 M68 55 Q72 60 75 57'} />
            <path class="fam-whisker" d="M52 55 L42 52 M52 59 L41 61 M83 55 L95 52 M83 59 L96 61" />
          </svg>
        {:else if kind === 'bat'}
          <svg viewBox="0 0 112 100" aria-hidden="true" class="familiar-figure">
            <g class="fam-wing-left"><path class="fam-wing" d="M50 52 L35 31 L13 24 L22 44 L7 52 L30 58 L29 66 L48 62 Z" /></g>
            <g class="fam-wing-right"><path class="fam-wing" d="M62 52 L77 31 L99 24 L90 44 L105 52 L82 58 L83 66 L64 62 Z" /></g>
            <ellipse class="fam-body" cx="56" cy="62" rx="17" ry="24" />
            <path class="fam-ear" d="M43 46 L41 16 L54 35 M58 35 L71 16 L69 46" />
            <circle class="fam-face" cx="56" cy="45" r="19" />
            {#if spriteMood(kind) === 'sleeping'}
              <path class="fam-eye-line" d="M43 44 Q48 48 52 44 M61 44 Q65 48 70 44" />
            {:else if spriteMood(kind) === 'worried'}
              <path class="fam-eye-line" d="M43 37 L51 40 M61 40 L69 37" />
              <circle class="fam-eye" cx="48" cy="45" r="3.2" /><circle class="fam-eye" cx="64" cy="45" r="3.2" />
            {:else}
              <circle class="fam-eye" cx="48" cy="45" r="3" /><circle class="fam-eye" cx="64" cy="45" r="3" />
            {/if}
            <path class="fam-fang" d="M52 55 L55 62 L58 55 M60 55 L63 62 L66 55" />
            <path class="fam-mouth" d={spriteMood(kind) === 'happy' ? 'M50 53 Q56 59 64 53' : 'M53 55 L60 55'} />
          </svg>
        {:else if kind === 'bunny'}
          <svg viewBox="0 0 112 100" aria-hidden="true" class="familiar-figure">
            <ellipse class="fam-shadow" cx="53" cy="91" rx="32" ry="5" />
            <circle class="fam-fluff" cx="27" cy="71" r="10" />
            <ellipse class="fam-body" cx="50" cy="70" rx="27" ry="18" />
            <ellipse class="fam-foot" cx="65" cy="86" rx="12" ry="6" />
            <g class="fam-ears"><ellipse class="fam-ear" cx="56" cy="22" rx="8" ry="26" transform="rotate(-12 56 22)" /><ellipse class="fam-ear" cx="77" cy="22" rx="8" ry="26" transform="rotate(12 77 22)" />
              <path class="fam-inner-ear" d="M53 9 L60 31 M79 9 L73 30" /></g>
            <circle class="fam-face" cx="68" cy="53" r="21" />
            {#if spriteMood(kind) === 'sleeping'}
              <path class="fam-eye-line" d="M58 52 Q63 56 67 52 M75 52 Q80 56 84 52" />
            {:else if spriteMood(kind) === 'worried'}
              <circle class="fam-eye" cx="63" cy="52" r="3.5" /><circle class="fam-eye" cx="79" cy="52" r="3.5" /><path class="fam-eye-line" d="M59 45 L65 47 M77 47 L83 45" />
            {:else}
              <circle class="fam-eye" cx="63" cy="52" r="2.7" /><circle class="fam-eye" cx="79" cy="52" r="2.7" />
            {/if}
            <path class="fam-nose" d="M69 59 L73 59 L71 63 Z" />
            <path class="fam-mouth" d={spriteMood(kind) === 'happy' ? 'M71 64 Q67 70 63 65 M71 64 Q75 70 79 65' : 'M71 64 L71 68'} />
          </svg>
        {:else}
          <svg viewBox="0 0 112 100" aria-hidden="true" class="familiar-figure">
            <ellipse class="fam-shadow" cx="53" cy="91" rx="31" ry="5" />
            <path class="fam-line fam-tail" d="M28 70 Q 9 52 17 39" />
            <ellipse class="fam-body" cx="49" cy="71" rx="26" ry="18" />
            <path class="fam-paw" d="M41 83 L37 90 M60 82 L65 90" />
            <circle class="fam-face" cx="69" cy="49" r="21" />
            <path class="fam-ear" d="M52 36 Q40 36 44 62 Q52 68 58 52 M82 36 Q97 32 93 59 Q86 65 79 53" />
            <ellipse class="fam-muzzle" cx="75" cy="56" rx="11" ry="8" />
            {#if spriteMood(kind) === 'sleeping'}
              <path class="fam-eye-line" d="M60 46 Q64 50 68 46 M78 46 Q83 50 87 46" />
            {:else if spriteMood(kind) === 'worried'}
              <circle class="fam-eye" cx="65" cy="47" r="3.2" /><circle class="fam-eye" cx="82" cy="47" r="3.2" />
              <path class="fam-eye-line" d="M60 41 L67 43 M80 43 L87 41" />
            {:else}
              <circle class="fam-eye" cx="65" cy="47" r="2.8" /><circle class="fam-eye" cx="82" cy="47" r="2.8" />
            {/if}
            <ellipse class="fam-nose" cx="81" cy="55" rx="3" ry="2.6" />
            <path class="fam-mouth" d={spriteMood(kind) === 'happy' ? 'M81 61 Q75 72 68 63' : 'M81 61 Q77 64 73 62'} />
            <path class="fam-collar" d="M55 64 Q69 71 82 66" />
          </svg>
        {/if}

        {#if spriteMood(kind) === 'happy'}
          <span class="familiar-particle" aria-hidden="true">♥</span>
        {:else if spriteMood(kind) === 'sleeping'}
          <span class="familiar-particle familiar-zzz" aria-hidden="true">zZ</span>
        {:else if signals[kind].active}
          <span class="familiar-alert-dot" aria-hidden="true">!</span>
        {/if}

        {#if motions[kind].hovered || spriteMood(kind) === 'happy' || (signals[kind].active && (spriteMood(kind) === 'worried' || spriteMood(kind) === 'curious'))}
          <span class="familiar-bubble" aria-hidden="true">
            {#if spriteMood(kind) === 'happy'}
              Nom nom! ♡
            {:else if motions[kind].hovered}
              {signals[kind].active ? signals[kind].message : 'Treat? :3'}
            {:else}
              {signals[kind].message}
            {/if}
            {#if signals[kind].active}<small>{source === 'demo' ? 'DEMO SIGNAL' : 'OBSERVED'}</small>{/if}
          </span>
        {/if}
      </button>
    {/if}
  {/each}
</div>
