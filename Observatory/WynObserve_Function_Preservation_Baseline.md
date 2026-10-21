# WynObserve Observatory | Function-Preservation Baseline

**Scope:** Comparison of uploaded original Rust/egui `src(1).zip` with current Svelte/Tauri `src.zip`, plus uploaded Tauri `lib.rs` and package files. Static source review, not a running application or a complete Agent/Hub/Protocol audit.

## Source baseline and scope

- Original source ZIP SHA-256: `80390b0b4a04f69e8bdcc4b286935edd1ea6b26f4fa4dff913723b1cced0aee8` (24 entries, 22 Rust `.rs` files).
- New Svelte source ZIP SHA-256: `19b5f9b9f7140769669186ea56de7a4f519e1ce97807f34e759f0e72c509f467` (19 entries).
- Legacy entry is a Rust/egui application. New UI is Svelte/Tauri. These are different application trees: `Observatory/src` vs `Observatory/frontend/src`. Neither source tree should overwrite the other.
- The old app contains real TCP telemetry/Hub plumbing alongside many view implementations that use mock or synthetic analytical datasets. **Visual presence does not prove real instrumentation.**

## Findings that affect preservation decisions

1. **40 of 40 legacy view labels exist in Svelte navigation, but 38 are placeholders.** Only THE MACHINE and FLEET render distinct implemented preview screens in the new UI. The existence of a route is not equivalent to retaining its feature.
2. **Legacy live data pipeline missing from new Tauri shell:** `live.rs` consumes live local telemetry; `hub.rs` connects and reconnects to the Fleet Hub and processes requests/responses; `normalize.rs` converts wire telemetry. Uploaded new `lib.rs` only starts Tauri and logging.
3. **Legacy target selection is global.** Choosing a Fleet machine changes the selected `SystemSnapshot` read by most other views. Svelte `chosenDeviceId` currently only changes Fleet inspector presentation and demo map/list selection.
4. **Legacy process map is an actual routed request**, not just a graphic. Process selection records machine ID, PID and start-time, sends a Hub request, and displays result/error with refresh, search, filter, pagination and region inspector.
5. **Instruction Vein is materially different:** original samples selected-process host x86-64 activity over Hub, validates machine/PID responses and keeps up to **512** samples; Svelte feeds simulated instruction rows with a **256** record cap, even before the earlier polish patch. This is a useful new preview but *not* a drop-in functional replacement.
6. **User controls missing or reduced:** original DESCEND/ASCEND drilldown, full process views, runtime NPU gating, settings persistence, custom theme palette editor, familiar selection/interactions, updater check/install/restart/status are not restored in Svelte.
7. **New additions worth retaining:** Svelte chart hover/pin interactions, local Instruction Vein hold/follow/inspector, Fleet table search/filters, and the new Contagion Map. These are additive, not grounds to drop old behavior.

## 40-view inventory

| Original view | Original entry point | New frontend status |
|---|---|---|
| THE MACHINE | `ui::overview` | Svelte demo view, not feature parity |
| THE SERVER | `ui::server_view` | Svelte placeholder only |
| FLEET | `ui::fleet_view` | Svelte demo view, not feature parity |
| CPU | `ui::cpu_view` | Svelte placeholder only |
| MEMORY | `ui::memory_view` | Svelte placeholder only |
| GPU | `ui::gpu_view` | Svelte placeholder only |
| PROCESSES | `ui::processes_view` | Svelte placeholder only |
| NETWORK | `ui::network_view` | Svelte placeholder only |
| NPU OVERVIEW | `ui::npu_view` | Svelte placeholder only |
| NPU MEMORY | `ui::npu_memory_view` | Svelte placeholder only |
| NPU RUNTIME | `ui::npu_runtime_view` | Svelte placeholder only |
| TIME MACHINE | `ui::replay_view` | Svelte placeholder only |
| CAUSALITY | `ui::causality_view` | Svelte placeholder only |
| SYSCALLS | `ui::syscalls_view` | Svelte placeholder only |
| FLAMEGRAPH | `ui::flamegraph_view` | Svelte placeholder only |
| MEMORY MAP | `ui::memory_map_view` | Svelte placeholder only |
| SCHEDULER | `ui::scheduler_view` | Svelte placeholder only |
| CACHE / PMU | `ui::cache_view` | Svelte placeholder only |
| IRQ / SOFTIRQ | `ui::irq_view` | Svelte placeholder only |
| PACKET FLOW | `ui::packet_flow_view` | Svelte placeholder only |
| PCIe FABRIC | `ui::fabric_view` | Svelte placeholder only |
| NUMA | `ui::numa_view` | Svelte placeholder only |
| STORAGE I/O | `ui::storage_io_view` | Svelte placeholder only |
| POWER / THERMAL | `ui::power_view` | Svelte placeholder only |
| LOCKS | `ui::locks_view` | Svelte placeholder only |
| CGROUPS | `ui::containers_view` | Svelte placeholder only |
| DATABASE | `ui::database_view` | Svelte placeholder only |
| TRACES | `ui::traces_view` | Svelte placeholder only |
| COMPARE | `ui::compare_view` | Svelte placeholder only |
| SNAPSHOT DIFF | `ui::diff_view` | Svelte placeholder only |
| CROSS-MACHINE | `ui::cross_machine_view` | Svelte placeholder only |
| ANOMALIES | `ui::anomalies_view` | Svelte placeholder only |
| INCIDENTS | `ui::incidents_view` | Svelte placeholder only |
| SOURCE → SILICON | `ui::source_silicon_view` | Svelte placeholder only |
| FIRMWARE | `ui::firmware_view` | Svelte placeholder only |
| SECURITY | `ui::security_view` | Svelte placeholder only |
| BINARY | `ui::binary_view` | Svelte placeholder only |
| AUTOPSY | `ui::autopsy_view` | Svelte placeholder only |
| KERNEL | `ui::kernel_view` | Svelte placeholder only |
| LOGS | `ui::logs_view` | Svelte placeholder only |

Exact source path references and page-specific requirements are in `WynObserve_View_Parity_Matrix.csv`.

## Legacy capabilities that require explicit preservation tests

### Telemetry and targeting
- [ ] Live local M6 telemetry connection and reconnect (`src/live.rs`), with schema checking and disconnected state.
- [ ] Hub fleet feed and automatic reconnect (`src/hub.rs`), correct protocol version handling and error status.
- [ ] Selecting a Fleet machine retargets ALL relevant dashboards and inspectors (`src/app.rs::sync_selected_snapshot`).
- [ ] Offline target fallback, missing telemetry and unavailable capability states behave explicitly and safely.
- [ ] NPU navigation/selection is capability-gated; multi-socket CPU topology and full physical hardware fields are not flattened into one demo device.
- [ ] Real wire data and simulated analysis are visibly distinguished per data item; never silently relabel mock as observed.

### Process forensics and instruction sampling
- [ ] PROCESSES page preserves running/sleeping tabs, detailed columns, CPU and I/O availability markers, hover executable path, and MAP action.
- [ ] MAP uses machine ID + PID + expected start time to avoid stale/PID reuse, shows loading/error, and supports refresh/search/filters/pagination and region details.
- [ ] Instruction sampling targets the selected process and machine, rejects stale/mismatched responses and preserves 512-sample legacy history minimum (or explicitly increases it).
- [ ] Instruction inspection retains PID, TID, logical CPU, address, bytes, mnemonic, operands, architecture, provenance, and notes.
- [ ] Sampling is stopped while hidden/paused as in legacy. GPU-side context does not falsely claim NVIDIA SASS sampling.

### UI capabilities, settings and updates
- [ ] THE MACHINE restores its CPU socket/core topology, optional NPU, process activity on DESCEND, and component-specific inspection.
- [ ] Original 40 pages each have a corresponding functioning implementation; protect their real-vs-mock behavior instead of replacing them with empty routes.
- [ ] Playback pause/resume and 0.25×, 1×, 4× controls; DESCEND/ASCEND global detail mode.
- [ ] Configurable display title and familiar toggles; Gothic, Pastel Dusk, Cyberpunk, and Custom themes with per-color editing and persistent preferences.
- [ ] Animated cat, bat, bunny and puppy familiars retain click-to-feed, cursor-follow, sleeping, and telemetry/incident-responsive behaviors (`src/ui/critters.rs`).
- [ ] Updater checks real installed/latest/status through `wyn-update --json`; update/install flow and restart remain available under appropriately controlled Tauri commands.
- [ ] Version footer uses returned component versions; no static demo values presented as real version information.

### Additional view-level caveats
- [ ] SERVER topology is backed partly by `AnalysisSnapshot::mock`, so preserve the schematic while keeping synthetic data labeled.
- [ ] Extreme analysis (`src/extended_analysis.rs`) and many deep panels render synthetic mock datasets. Do not count them as currently live without verifying collectors.
- [ ] The incident Capture button currently increments a UI counter; displayed replay/export/compare buttons do not alone prove archive creation. Preserve existing visible behavior and separately implement promised backend functions.
- [ ] Preserve Fleet table/map SEARCH, SITE, STATUS filters and selection from new frontend as additive features.
- [ ] New Instruction Vein follows newest records and supports hold/filter/inspect; preserve those upgrades while restoring live sample source.

## Change policy / migration gates

1. **Source of truth:** Original Rust code is the functionality baseline; new Svelte code is the design and interaction baseline. Any future change must pass both.
2. **Never overwrite:** Do not replace `Observatory/src` with `Observatory/frontend/src`; do not delete old Rust code or tests until the new implementation passes feature parity.
3. **Feature flag or parallel running:** Keep original Observatory launchable while porting. New pages should be additive; mark incomplete pages as PREVIEW/UNAVAILABLE rather than implying restored functionality.
4. **No fake-live promotion:** An interaction counts as restored only when connected to the actual Hub/Agent/Updater as appropriate, with loading/error/disconnected behavior.
5. **Review checklist on every change:** enumerate files touched, previous behaviors preserved, newly enabled behaviors, missing behaviors, and test results. No unrelated file replacement.
6. **Testing gates:** static page/view coverage check; TypeScript `npm run check`; frontend build; Rust `cargo check` (when workspace/Cargo files supplied); manual regression through live Agent/Hub and all relevant UI controls.
7. **Trust boundary:** Tauri integration should expose narrowly scoped commands, validate request targets, and avoid turning raw frontend parameters into arbitrary network/process operations.

**Verification limit:** This is a source-based inventory. Cargo manifests, real Hub/Agent/Protocol sources, test fixtures, and a running instance were not supplied here. Therefore full runtime parity and backend correctness cannot yet be certified.
