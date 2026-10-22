# Svelte Observatory telemetry

The desktop now reads the existing Agent → Hub fleet feed through
`wyn-observatory-core`. Tauri owns one reconnecting TCP client and exposes the
read-only `fleet_snapshot` command. Svelte polls that state every 250 ms; it never
opens a socket or decides which Hub to contact.

## Run

From the repository root, run the Hub and Agent in separate terminals:

```sh
cargo run -p wyncommand-hub -- --config Hub/config/dev.toml
cargo run -p wyncommand-agent -- --config Agent/config/wyn-itpc-dev.toml
```

Then, from `Observatory/frontend`:

```sh
npm install
WYNCOMMAND_OBSERVATORY_CONFIG="$(realpath ../config/dev.toml)" npx tauri dev
```

Existing installations can use their already-running Hub and Agents. The config
file's `[hub].endpoint` selects the Hub's **Observatory** port (5769 in local dev).
The desktop also accepts `--config <path>`. Invalid explicit config fails startup;
omitting config selects `127.0.0.1:5769`. The old `[local]` direct-Agent connection
is not used by this bridge.

`npm run dev` opens a browser-only Demo. Live telemetry requires the Tauri host;
choosing Live in a plain browser reports that requirement.

## Selection and data behavior

- Fleet selection is shared by Machine, CPU, Memory, GPU, Processes and all three NPU pages. The
  first online machine with telemetry is selected initially. A selected machine
  stays selected when it goes offline or disappears.
- A target/source change clears history, chart pins, core/process selections and
  instruction previews. Histories retain up to 80 distinct Agent timestamps.
- Pausing holds readings while connection and Fleet status continue updating.
- Disconnected or >5-second-old data is labelled stale. Last readings for the
  same target remain visible; they are not replaced with demo measurements.
- CPU load/core clocks and used/available/total memory come from the Agent.
  Available RAM is the reported value, not an estimate from `total - used`.
- Missing counters render `N/A` and gaps in charts. CPU temperature includes the
  measured Celsius value and sensor source when supported. GPU, basic process, storage and network fields use the existing
  availability flags. Zero reported CPU frequency is treated as unavailable.
- Live map coordinates, network tracing, memory-map requests and Instruction
  Vein requests are still separate migration work. Demo retains its previews.

## CPU temperature and GPU discovery

Linux CPU temperature comes from identified `coretemp` or `k10temp` hwmon
channels. Intel package and AMD Tdie readings are preferred per device; the
hottest identified core/CCD is a fallback when that device lacks a valid
package/die reading. Multiple CPU devices use the hottest reading. The driver
and sensor label are shown on the CPU page. Unrelated ACPI, motherboard, NVMe
and GPU sensors are excluded, as are AMD Tctl control values that need not be
physical temperatures. See the kernel's [coretemp](https://docs.kernel.org/hwmon/coretemp.html)
and [k10temp](https://docs.kernel.org/hwmon/k10temp.html) documentation.

Discovery and reads happen each sample. Missing, disabled, faulty or unreadable
sensors clear the reading, so charts show a gap instead of a stale temperature.
No sensor configuration or driver loading is performed. Windows AMD CPU temperature uses the installed Radeon driver's explicit CPU
sensor where supported; no separate monitoring app is required. A driver/model
without that sensor, and Windows Intel CPUs, still report N/A. See
[Windows collector details](../../Agent/WINDOWS.md).

On Windows, the GPU page lists DXCore graphics adapters, including AMD/Intel
integrated GPUs even when NVML is unavailable. `IsIntegrated` comes from the
driver; it is not inferred from a CPU name or memory size. Dedicated memory
capacity and the shared-system-memory limit are labelled separately from usage.
Inventory refreshes every five seconds. Missing DXCore support does not prevent
Agent startup; the known NVML GPU remains a fallback identity when available.
Select an adapter to view its own utilization and temperature history. AMD ADL
reads supported Radeon temperatures; Windows GPU Engine supplies utilization when
AMD's sensor API omits it. NVML continues to supply NVIDIA counters. Missing
sensors create gaps. Explicit GPU selection remains on that identity if the
adapter disappears. Machine lists every discovered adapter with its own readings.

Rebuild/restart the Agent, Hub and Tauri desktop to use these additive protocol
fields. Older Agents remain readable but omit the new fields; an older Hub or
desktop can discard them while deserializing, even if its connection succeeds.

## Verify

```sh
npm run check
npm test
npm run build
```

Tests use Node 24's built-in TypeScript stripping and test runner. The adapter
checks measured values, unavailable fields, explicit machine selection, offline
Fleet entries, snapshot deduplication and bounded history. Core transport tests
exercise fragmented packets, reconnects and stale/disconnected status:

```sh
cargo test -p wyn-observatory-core
cargo run -p wyn-observatory-core --example inspect_feed -- 127.0.0.1:5769
```

The example prints a real fleet packet as JSON and exits; it is also useful for
separating transport failures from frontend failures. Rust code in
`ObservatoryCore` has no egui or Tauri dependency. The legacy egui application
still uses its existing client during this migration.

## NPU pages

Overview, Memory and Runtime consume the selected machine's existing NPU fields.
Each counter honors its own availability flag. Memory is resident MiB, not a
capacity percentage. Runtime shows device activity, current/maximum clocks and
power state where reported. Model-level attribution, inference latency and queue
depth are not collected. Older Agents without capability flags show N/A.

## Window placement

The desktop uses Tauri's window-state plugin to save window size, position and
maximized state on normal application exit. State is stored in
`wynobserve-window-state.json` under Tauri's app config directory. On startup,
WynObserve bounds the saved placement to one monitor's usable work area before
showing the window, leaving room for its titlebar and borders. Oversized saved
windows recover to the configured startup size, bounded to that display; valid
sizes and positions remain unchanged, and maximized windows still reopen
maximized. Removed displays fall back to an
available monitor. Saved dimensions and monitor bounds use physical pixels,
including on scaled displays. Linux applies the restore in GTK logical units
using the monitor scale: a hidden Wayland window can report scale 1 before
switching to scale 2 when shown. This avoids doubling the saved dimensions on
every launch. Missing or invalid state uses the configured startup size. Position restoration is subject to the window manager (notably
Wayland); a forced process termination may not save the latest placement. No
frontend storage or additional IPC permissions are required. Close and reopen
the desktop after resizing/moving it to verify behavior on the target OS.

## Machine hardware and motherboard

Machine's summary, Logical View and Motherboard use the selected target's GPU
inventory and NPU availability flag. Integrated GPUs are labelled explicitly;
devices remain visible when individual counters are unavailable. No GPU/NPU is
inferred from a CPU model. Older measured primary GPUs remain supported.

Selecting a component in either view updates the same inspector and two focused
charts. GPU histories follow the adapter ID, never another GPU's counters. NPU
charts show device utilization and resident MiB (not a capacity percentage).
Switching views retains focus; changing target/source resets it and the history.

The Motherboard tab prioritizes an illustrated PCB with hardware-shaped controls:
a CPU socket/heat spreader, DIMMs, graphics cards, an M.2-style storage device and
a LAN port/controller. NPU and integrated GPU blocks are enlarged details. Summary
cards stay in Logical View; the board itself carries no numeric telemetry.

Clicking a part opens a non-modal in-app popup beside it with its identity and
current readings. Click the part again, click elsewhere, use the close button or
press Escape to dismiss. Escape/close restores focus to the part; Tab continues
through the board. The popup follows scrolling/resizing, stays within the visible
panel, and closes when its part scrolls out of view or disappears. Switching views
or machines also dismisses it. Chart/inspector selection is still shared.

Motherboard identity is collected once when the Agent starts and travels through
Agent → Hub → ObservatoryCore → Tauri in optional `motherboard` telemetry. Linux
uses readable DMI sysfs identity files, with SMBIOS memory inventory when the raw
DMI table is readable. Windows uses the built-in read-only
[GetSystemFirmwareTable](https://learn.microsoft.com/windows/win32/api/sysinfoapi/nf-sysinfoapi-getsystemfirmwaretable)
RSMB API; no separate app, shell command, WMI service or bundled driver is needed.
The shared bounds-checked parser follows [DMTF SMBIOS 3.9.0](https://www.dmtf.org/sites/default/files/standards/documents/DSP0134_3.9.0.pdf)
Types 1/2/3/16/17. It retains manufacturer/model/revision, system model, chassis and
system-memory positions/sizes/form factors; serial numbers and UUIDs are excluded.
OEM placeholders become unavailable. A restricted Linux DMI table leaves RAM
population unknown even when board identity is readable. The Agent does not change
permissions. Restart it to refresh static firmware inventory.

The Machine identity strip shows the selected machine's actual board name and
revision when supplied. `motherboard.ts` holds a conservative profile registry:
Gigabyte **B860M DS3H WIFI6E** matches by vendor and exact model and uses the shared
Micro-ATX dimensions, LGA1851 socket, four DDR5 sockets, four physical PCIe slots
(top ×16, three electrically ×1), two M.2 connectors and four SATA ports from
[GIGABYTE's specifications](https://www.gigabyte.com/us/Motherboard/B860M-DS3H-WIFI6E-rev-1x/sp).
A model profile is an approximate illustration, not a revision-specific CAD map.
Other boards adapt to reported portable/server/desktop chassis and RAM inventory;
no form factor or connector count is guessed from CPU brand or a board-name prefix.

Memory artwork uses reported occupied/empty/unknown positions and SO-DIMM or chip
forms where reported. A known profile maps physical DIMMs only by unambiguous
locator labels (e.g. DIMM_A2 → DDR5_A2); firmware ordering is never treated as
physical ordering. Unmapped sockets remain dashed/unknown. The memory popup lists
firmware inventory, including empty sockets. At most 16 positions are drawn; the
popup retains the reported list (bounded to 256 in the bridge). Unknown inventory
uses a neutral memory bank. Demo has explicitly synthetic inventory.

The PCB keeps its profile shape as additional adapters arrive. Enlarged package
accelerators and extra GPUs occupy a separate detail area. GPU and storage overlays
represent the reported subsystem; they do not assert occupied PCIe or M.2 sockets.
Component shapes, trace placement and signal animation remain illustrative.
Glow and simulated pulse intensity follow available activity counters; memory
uses pressure, and storage/network use throughput scales. Unknown activity stays
unlit; reported zero is idle. The canvas expands for additional accelerators without stretching the PCB.
The board initially fits the available canvas, including narrow panes. Zoom buttons,
Ctrl/Command + wheel and touch pinch zoom from 1× to 4× relative to that fit.
Drag the background to pan when zoomed. Focus the canvas to use +/−, arrow keys
(Shift for larger steps), or 0 to fit. Keyboard focus pans hardware into view.
The Fit button restores the entire board; resizing keeps the camera bounded.

Activity mode pulses the CPU with load and lights storage/LAN activity LEDs.
Read/receive signals travel toward the CPU in cyan; write/transmit signals travel
outward in the secondary theme color, each driven by its own measured rate.
Memory pulses represent pressure, not measured memory-bus bandwidth. Hover or
keyboard focus reveals a name label and highlights associated traces.
Pause, stale telemetry and reduced-motion preferences stop pulses and blinking.
GPU fans are static because fan-speed telemetry is not collected.

Thermal mode uses only measured CPU/GPU Celsius readings on a fixed 30–95°C
color scale, clamped at each end. This is a visualization scale, not a device's
thermal limit or an overheating diagnosis. Devices without sensors stay neutral;
CPU temperature is never copied onto an iGPU or NPU.

Component popups include two compact histories drawn from the shared, bounded
80-sample timeline. Timestamps determine horizontal position; unavailable values
and gaps over five seconds break the line. Single samples remain visible as dots.
GPU histories remain tied to adapter IDs, including when sensors disappear.

The CPU popup's **Explore CPU package** opens an interactive enlarged lens.
Only reported hardware is eligible: GPUs need an explicit integrated flag and a
matching CPU vendor; NPU grouping recognizes Intel NPU/AI Boost and AMD Ryzen
AI/XDNA identities. Unknown accelerators remain separate on the board. Grouping
is illustrative and inferred from device identity, not detected die topology.
Selecting a block updates its readings, histories and shared Machine focus.
Tab traverses the package controls before returning to the board; Escape closes
the popup and returns focus to its original hardware trigger.

Motherboard changes require rebuilding/restarting Agent, Hub and the Tauri desktop.
Older Agents remain compatible but omit identity; older typed relays can discard
the added field. Windows firmware collection still needs a run on Windows hardware.
