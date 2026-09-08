# WynCommand // Observatory UI Prototype

This is a **standalone, mock-data UI prototype** for M6.

It is intentionally separated from the real telemetry backend so the interface can exist now without forcing the backend architecture to be complete.

## Run

```bash
cargo run --release
```

## What is implemented

- Gothic / technical WynCommand visual language
- The Machine / CPU / Memory / GPU / Processes / Kernel / Network / Logs views
- Animated machine schematic
- CPU logical-core activity animation
- RAM usage visualization
- GPU compute-grid animation
- NVMe and network activity
- Animated schematic data-flow particles
- Clickable hardware components
- Selected-component detail inspector
- Telemetry truth-level legend
- Live / pause / playback-speed controls
- `DESCEND` detail mode
- Mock event stream
- Backend integration seam through `TelemetrySource`

## Backend integration

The UI knows only about the normalized structures in `src/model.rs`.

The important boundary is:

```rust
pub trait TelemetrySource {
    fn poll(&mut self, elapsed_seconds: f64) -> SystemSnapshot;
}
```

`MockTelemetry` implements this trait today.

Later, the real Observatory backend can implement the same trait:

```rust
pub struct LinuxTelemetry {
    // sysinfo
    // /proc
    // /sys
    // perf
    // eBPF
    // NVML
    // ...
}

impl TelemetrySource for LinuxTelemetry {
    fn poll(&mut self, elapsed_seconds: f64) -> SystemSnapshot {
        // collect real data
        todo!()
    }
}
```

Then change the source constructed in `ObservatoryApp::new()`.

The rendering code should not need to know whether values came from mock telemetry, `sysinfo`, `/proc`, NVML, eBPF, or a tiny assembly gremlin.

## Suggested merge into M6 later

When ready, copy:

```text
src/app.rs
src/model.rs
src/theme.rs
src/ui/
```

into `Agent/src/`.

Keep `src/mock.rs` around as a development/demo backend.

The current M6 collectors can then be adapted behind `TelemetrySource` rather than rewritten around the UI.
