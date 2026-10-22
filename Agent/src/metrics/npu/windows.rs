//! DXCore NPU telemetry. Missing driver counters remain explicitly unavailable.
use super::NpuSnapshot;
use crate::windows::wide;
use std::{ffi::c_void, mem::size_of, ptr, time::Instant};
use windows::{
    Win32::Graphics::DXCore::*,
    core::{GUID, HRESULT, Interface},
};
use windows_sys::Win32::{
    Foundation::{FreeLibrary, HMODULE},
    System::LibraryLoader::*,
};

// New ABI entries missing from windows-rs metadata. Source (MIT):
// https://github.com/microsoft/DirectX-Headers/blob/main/include/directx/dxcore_interface.h
windows::core::imp::define_interface!(
    Adapter1,
    Adapter1Vtable,
    0xa0783366_cfa3_43be_9d79_55b2da97c63c
);
#[repr(C)]
pub struct Adapter1Vtable {
    base: IDXCoreAdapter_Vtbl,
    property_with_input: unsafe extern "system" fn(
        *mut c_void,
        DXCoreAdapterProperty,
        usize,
        *const c_void,
        usize,
        *mut c_void,
    ) -> HRESULT,
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct EngineIndex {
    physical: u32,
    engine: u32,
}
#[repr(C)]
#[derive(Default)]
struct Frequency {
    current: u64,
    maximum: u64,
    overclocked: u64,
}
#[repr(C)]
#[derive(Default)]
struct MemoryUsage {
    committed: u64,
    resident: u64,
}
#[derive(Debug)]
struct Module(HMODULE);
impl Drop for Module {
    fn drop(&mut self) {
        unsafe {
            FreeLibrary(self.0);
        }
    }
}
#[derive(Debug)]
struct Device {
    // Field order releases COM before unloading the implementing DLL.
    adapter: IDXCoreAdapter,
    _module: Module,
    engines: Vec<EngineIndex>,
    physical_count: u32,
    model: String,
    previous: Option<(Instant, Vec<u64>)>,
}
#[derive(Debug, Default)]
pub(super) struct PlatformNpuCollector {
    device: Option<Device>,
}
fn property<T: Default>(adapter: &IDXCoreAdapter, id: u32) -> Option<T> {
    let mut output = T::default();
    unsafe {
        adapter
            .GetProperty(
                DXCoreAdapterProperty(id),
                size_of::<T>(),
                (&mut output as *mut T).cast(),
            )
            .ok()?;
    }
    Some(output)
}
fn state<I, O: Default>(adapter: &IDXCoreAdapter, id: u32, input: &I) -> Option<O> {
    let mut output = O::default();
    unsafe {
        if !adapter.IsQueryStateSupported(DXCoreAdapterState(id)) {
            return None;
        }
        adapter
            .QueryState(
                DXCoreAdapterState(id),
                size_of::<I>(),
                Some((input as *const I).cast()),
                size_of::<O>(),
                (&mut output as *mut O).cast(),
            )
            .ok()?;
    }
    Some(output)
}
impl Device {
    fn discover() -> Option<Self> {
        let handle = unsafe {
            LoadLibraryExW(
                wide("dxcore.dll").as_ptr(),
                ptr::null_mut(),
                LOAD_LIBRARY_SEARCH_SYSTEM32,
            )
        };
        if handle.is_null() {
            return None;
        }
        let module = Module(handle);
        let entry =
            unsafe { GetProcAddress(handle, c"DXCoreCreateAdapterFactory".as_ptr().cast()) }?;
        let create: unsafe extern "system" fn(*const GUID, *mut *mut c_void) -> HRESULT =
            unsafe { std::mem::transmute(entry) };
        let mut raw = ptr::null_mut();
        unsafe {
            create(&IDXCoreAdapterFactory::IID, &mut raw).ok().ok()?;
        }
        if raw.is_null() {
            return None;
        }
        let factory = unsafe { IDXCoreAdapterFactory::from_raw(raw) };
        let list: IDXCoreAdapterList = unsafe {
            factory
                .CreateAdapterList(&[GUID::from_u128(0xd46140c4_add7_451b_9e56_06fe8c3b58ed)])
                .ok()?
        };
        // The wire format represents one NPU, matching the Linux collector.
        let adapter: IDXCoreAdapter = unsafe { list.GetAdapter(0).ok()? };
        let length = unsafe { adapter.GetPropertySize(DriverDescription).ok()? }.min(64 * 1024);
        let mut description = vec![0u8; length];
        unsafe {
            adapter
                .GetProperty(DriverDescription, length, description.as_mut_ptr().cast())
                .ok()?;
        }
        let model =
            String::from_utf8_lossy(description.split(|b| *b == 0).next().unwrap_or_default())
                .into_owned();
        let physical_count = property::<u32>(&adapter, 15)
            .filter(|n| *n > 0 && *n <= 64)
            .unwrap_or(0);
        let mut engines = Vec::new();
        if let Ok(newer) = adapter.cast::<Adapter1>() {
            for physical in 0..physical_count {
                let mut count = 0u32;
                let result = unsafe {
                    (newer.vtable().property_with_input)(
                        newer.as_raw(),
                        DXCoreAdapterProperty(16),
                        size_of::<u32>(),
                        (&physical as *const u32).cast(),
                        size_of::<u32>(),
                        (&mut count as *mut u32).cast(),
                    )
                };
                if result.is_err() || count > 1024 {
                    engines.clear();
                    break;
                }
                engines.extend((0..count).map(|engine| EngineIndex { physical, engine }));
            }
        }
        drop(list);
        drop(factory);
        Some(Self {
            adapter,
            _module: module,
            engines,
            physical_count,
            model,
            previous: None,
        })
    }
}
impl PlatformNpuCollector {
    pub(super) fn new() -> Self {
        Self {
            device: Device::discover(),
        }
    }
    pub(super) fn sample(&mut self) -> NpuSnapshot {
        let mut snapshot = NpuSnapshot {
            available: false,
            utilization_available: false,
            frequency_available: false,
            memory_available: false,
            model: "NPU unavailable".into(),
            pci_address: String::new(),
            utilization_percent: 0.0,
            current_frequency_mhz: 0,
            max_frequency_mhz: 0,
            memory_used_bytes: 0,
            power_state: "unknown".into(),
        };
        let Some(device) = self.device.as_mut() else {
            return snapshot;
        };
        if !unsafe { device.adapter.IsValid() } {
            self.device = Device::discover();
            return snapshot;
        }
        snapshot.available = true;
        snapshot.model = device.model.clone();
        let now = Instant::now();
        let running: Option<Vec<u64>> = device
            .engines
            .iter()
            .map(|e| state(&device.adapter, 4, e))
            .collect();
        if let Some(running) = running.filter(|v| !v.is_empty()) {
            if let Some((at, previous)) = &device.previous {
                let elapsed = now.duration_since(*at).as_secs_f64() * 1_000_000.0;
                if elapsed > 0.0
                    && previous.len() == running.len()
                    && running.iter().zip(previous).all(|(n, p)| n >= p)
                {
                    // Busiest engine, so idle engines do not hide saturation.
                    snapshot.utilization_percent = running
                        .iter()
                        .zip(previous)
                        .map(|(n, p)| ((*n - *p) as f64 / elapsed * 100.0).clamp(0.0, 100.0) as f32)
                        .fold(0.0, f32::max);
                    snapshot.utilization_available = true;
                }
            }
            device.previous = Some((now, running));
        } else {
            device.previous = None;
        }
        let frequencies: Option<Vec<Frequency>> = device
            .engines
            .iter()
            .map(|e| state(&device.adapter, 9, e))
            .collect();
        if let Some(values) = frequencies.filter(|v| !v.is_empty()) {
            snapshot.current_frequency_mhz = values
                .iter()
                .map(|f| f.current / 1_000_000)
                .max()
                .unwrap_or(0);
            snapshot.max_frequency_mhz = values
                .iter()
                .map(|f| f.maximum / 1_000_000)
                .max()
                .unwrap_or(0);
            snapshot.frequency_available = true;
        }
        let memory: Option<Vec<MemoryUsage>> = (0..device.physical_count)
            .flat_map(|physical| (0..2u32).map(move |kind| [physical, kind]))
            .map(|input| state(&device.adapter, 2, &input))
            .collect();
        if let Some(values) = memory.filter(|v| !v.is_empty()) {
            snapshot.memory_used_bytes = values.iter().map(|v| v.resident).sum();
            snapshot.memory_available = true;
        }
        snapshot
    }
}
