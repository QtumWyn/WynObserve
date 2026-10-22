# Windows Agent

The Agent now has native x86-64 Windows implementations for the former platform
stubs. Linux remains supported through the existing modules. A Windows runtime
validation is still required: the implementation has been cross-built and linked
on Linux, including the Windows test executable, but not run on Windows hardware.

## Build and run

Install Rust, the Visual Studio C++ build tools (for the MSVC target), and NASM on
Windows. Make `nasm` available on PATH. From the repository root:

```powershell
cargo build -p wyncommand-agent
cargo test -p wyncommand-agent
.\target\debug\wyn-agent.exe --config .\Agent\config\windows-dev.toml
```

Edit the config to give each machine a unique ID/name and point `hub.endpoint` at
the Hub's **Agent** listener. The sample uses a local Hub on port 5768. Existing
Hub and protocol messages remain the transport; this does not install a service.

For a Linux cross-build with the Rust target, NASM and MinGW-w64 installed:

```sh
cargo build -p wyncommand-agent --target x86_64-pc-windows-gnu
cargo test -p wyncommand-agent --target x86_64-pc-windows-gnu --no-run
```

## Implemented collectors

| Area | Windows source and semantics |
| --- | --- |
| CPU/identity | Existing sysinfo CPU collector; Win64 NASM CPUID preserves RBX without changing the stack. RDTSCP returns the 64-bit counter and fences subsequent execution. Probe assembly is linked and tested. |
| Motherboard / firmware | `GetSystemFirmwareTable(RSMB)` collects baseboard manufacturer/model/revision, system model, chassis type and system-memory device positions/sizes/form factors. Cached at startup, no external process or monitoring app. Unknown/empty memory positions stay distinct; no serials or UUIDs are exported by this collector. |
| Memory | sysinfo physical/pagefile totals, available and used bytes; PDH cache bytes, modified page list, and total page-fault rate. |
| Scheduler | PDH context switches/sec and processor ready queue length. Ready queue excludes running threads; it is not identical to Linux `procs_running`. |
| Network | IP Helper route/interface lookup, 64-bit byte/packet/error/discard counters, and IPv4/IPv6 established TCP connections. The IPv4 route lookup sends no traffic. A changed interface or reset counter starts a new rate baseline. |
| Storage | PDH system-volume throughput, IOPS, utilization and queue depth; `GetDiskFreeSpaceExW` capacity/available space. These describe the logical system volume, not one inferred physical disk. |
| Processes | ToolHelp PID/parent/thread enumeration; process times, executable path, working set and I/O counters. CPU follows the Linux per-core convention and may exceed 100%. PID reuse invalidates the rate baseline. Protected process counters stay unavailable. |
| Process memory | `VirtualQueryEx` regions, protections and mapped paths; `ReadProcessMemory` for instruction decoding. Reserved memory has no access permissions. Image, mapped and private regions are classified without guessing heaps/stacks. |
| Instruction Vein | A private ETW sampled-profile session shared across thread queues, target filtering and existing iced-x86 decoding. Flushed buffers are consumed before draining; the last sampler closes its session. |
| NPU | Dynamically loaded DXCore, NPU hardware filtering, engine enumeration, busiest-engine utilization, frequency and resident memory queries where the driver supports them. No DXCore DLL/driver means unavailable NPU data, not Agent startup failure. |
| GPU | DXCore enumerates hardware graphics adapters, including Ryzen Radeon iGPUs, with driver-reported integrated/discrete status and dedicated/shared memory capacities. Inventory refreshes every five seconds independently of NVML. NVML measures NVIDIA; AMD ADL supplies supported Radeon sensors. Windows GPU Engine PDH counters provide a per-adapter utilization fallback. |

PDH queries use English counter names through `PdhAddEnglishCounterW`, so they do
not depend on the Windows display language. Every formatted counter checks its
status; rate counters need a baseline before they are available.

## Fields without a direct equivalent

CPU temperature is self-contained: on AMD CPUs the Agent reads the explicitly
identified `PMLOG_TEMPERATURE_CPU` sensor through the installed Radeon driver's
`atiadlxx.dll` (loaded from System32). GPU/SoC/ACPI temperatures are never relabelled
as CPU temperatures. This is conditional support, **not universal Windows CPU
thermal support**: a Ryzen model/driver without this sensor, an absent AMD driver,
and Intel CPUs currently report `null`. No external monitoring app or bundled
kernel driver is needed. Linux keeps its identified hwmon sensors.

GPU `adapters[].telemetry` carries independently optional utilization, temperature,
temperature source, memory usage, power and graphics clock. Missing or failed
queries are `null`, including missing Radeon memory usage; dedicated/shared
memory capacities are never substituted for usage. AMD PMLog readings use the
public ADL sensor IDs and units, with the older ADL GPU thermal-controller query
as a temperature fallback. The PMLog query is deprecated upstream but remains a
compatibility entry point; unsupported driver exports fail closed. A future ADL
shared-memory reader can replace it without changing the wire format.

Windows GPU Engine counters sum process activity on each physical engine and use
the busiest engine for each adapter, matching Task Manager's aggregation model.
DXCore LUIDs identify those counters. D3DKMT PCI locations join ADL/NVML sensors to
DXCore identities; ambiguous matches are not attributed by name. Inventory
refreshes every five seconds. Driver failures clear readings on the next sample.
The original top-level GPU fields retain their primary NVML semantics for older
clients; new clients use `primary_adapter_id` and per-adapter telemetry, or select
an available adapter when NVML is absent. The Agent installs no drivers.

`memory_detail_available` remains false on Windows because Linux active/cache/
dirty semantics are not equivalent. The modified-page count is exposed in
`dirty_bytes`, but consumers must honor availability. Total page faults are real;
`major_fault_rate_available` is false (Pages Input/sec is not a fault count).
Scheduler `runnable_tasks_available` and `blocked_tasks_available` distinguish a
missing count from zero; the Linux blocked-task equivalent is unavailable.

NPU `available` means a device was found. Separate `utilization_available`,
`frequency_available`, and `memory_available` flags describe successful queries.
The wire format represents the first NPU. PCI address and power state are not
inferred. New availability flags are additive and deserialize as false when absent.

Windows process I/O includes file/device/network transfers, unlike physical-disk
traffic. `last_cpu` is `usize::MAX` and state is `unknown`, since these APIs do not
report them. Memory-map file offsets, Unix device numbers and inodes are unknown
zero placeholders; paths can use Windows device names. Reserved regions count
toward virtual size. Never interpret these as Linux inode or physical-memory data.

## Windows runtime validation

Ordinary telemetry should work without elevation where Windows allows access.
Instruction sampling needs permission to start a system profiling trace and
`SeSystemProfilePrivilege`; elevation is typically needed. The Agent enables an
already-granted privilege only while starting its private session, then restores
the prior token state. It does not change security policy or take over an existing
logger. Permission errors are returned through the existing request error path.
WOW64 targets are explicitly rejected because the current decoder uses 64-bit x86.

On the Windows host:

1. Run normal tests, then compare CPU/memory/network/disk values with Task Manager
   over several sampling intervals. Check unavailable flags on the first interval.
2. Verify Fleet identity and CPU/memory in the Svelte desktop through the Hub.
   Check the Machine motherboard model against firmware/System Information and the
   RAM inventory against the installed modules. Restart Agent, Hub and desktop
   after rebuilding so every typed relay preserves the new optional field.
   On a Ryzen machine, verify that the GPU page lists its Radeon adapter as
   integrated and lists any discrete GPU separately. Compare memory capacities
   with Task Manager. Select each adapter and confirm its graphs change independently.
   Exercise a workload on the Radeon adapter and compare utilization/temperature
   with AMD's driver UI. Verify the CPU sensor source says AMD ADL when present;
   if CPU temperature remains N/A, inspect `cpu.temperature` in the feed rather
   than assuming a GPU temperature is a CPU reading. Test without NVIDIA/NVML too.
3. Exercise a memory-map request for a process owned by the user; confirm protected
   process access errors remain errors and the Agent continues collecting.
4. In an elevated terminal, run the ignored process sampler test:
   `cargo test -p wyncommand-agent samples_whole_process -- --ignored --nocapture`.
   This must yield decoded samples from multiple worker threads.
5. Validate DXCore counters on an NPU-equipped machine and unavailable flags on
   machines/drivers without the relevant interfaces.

ETW sampling is statistical, not a complete instruction trace. Up to 4096 samples
are buffered per target thread per drain. The legacy egui display does not yet
consume all the new field-level availability flags.

API references: [PDH](https://learn.microsoft.com/en-us/windows/win32/api/pdh/nf-pdh-pdhaddenglishcounterw),
[private system tracing](https://learn.microsoft.com/en-us/windows/win32/etw/configuring-and-starting-a-systemtraceprovider-session),
[sampled profile events](https://learn.microsoft.com/en-us/windows/win32/etw/sampledprofile),
[DXCore state queries](https://learn.microsoft.com/en-us/windows/win32/api/dxcore_interface/ne-dxcore_interface-dxcoreadapterstate),
[DXCore ABI header](https://github.com/microsoft/DirectX-Headers/blob/main/include/directx/dxcore_interface.h).

Sensor API references: [AMD ADL PMLog](https://gpuopen-librariesandsdks.github.io/adl/group__OVERDRIVE8API.html),
[AMD sensor definitions and units](https://github.com/GPUOpen-LibrariesAndSDKs/display-library/blob/master/include/adl_defines.h),
[GPU performance counters](https://devblogs.microsoft.com/directx/gpus-in-the-task-manager/),
[D3DKMT adapter information](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/d3dkmthk/nf-d3dkmthk-d3dkmtqueryadapterinfo).
