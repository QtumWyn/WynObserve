//! Read-only AMD sensors using the display driver's own ADL library.
//! No monitoring application, service, bundled DLL or kernel driver is required.
//! ABI reference: https://github.com/GPUOpen-LibrariesAndSDKs/display-library
use super::amd_sensors::PmLog;
use crate::windows::wide;
use std::{
    ffi::{CStr, c_void},
    mem::size_of,
    ptr,
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::{FreeLibrary, HMODULE},
    System::LibraryLoader::*,
};
use wyn_protocol::{CpuTemperatureSnapshot, GpuAdapterSnapshot, GpuAdapterTelemetry};

type Context = *mut c_void;
type Create =
    unsafe extern "C" fn(unsafe extern "C" fn(i32) -> *mut c_void, i32, *mut Context) -> i32;
type Destroy = unsafe extern "C" fn(Context) -> i32;
type Count = unsafe extern "C" fn(Context, *mut i32) -> i32;
type Info = unsafe extern "C" fn(Context, *mut AdapterInfo, i32) -> i32;
type Query = unsafe extern "C" fn(Context, i32, *mut PmLog) -> i32;
type TemperatureGet = unsafe extern "C" fn(Context, i32, i32, *mut Temperature) -> i32;
#[repr(C)]
#[derive(Clone)]
struct AdapterInfo {
    size: i32,
    index: i32,
    udid: [u8; 256],
    bus: i32,
    device: i32,
    function: i32,
    vendor: i32,
    name: [u8; 256],
    display: [u8; 256],
    present: i32,
    exists: i32,
    driver_path: [u8; 256],
    driver_path_ext: [u8; 256],
    pnp: [u8; 256],
    os_display_index: i32,
}
const _: () = assert!(size_of::<AdapterInfo>() == 1572);

#[repr(C)]
struct Temperature {
    size: i32,
    millidegrees: i32,
}
struct Module(HMODULE);
impl Drop for Module {
    fn drop(&mut self) {
        unsafe {
            FreeLibrary(self.0);
        }
    }
}
impl Module {
    unsafe fn symbol<T: Copy>(&self, name: &CStr) -> Option<T> {
        // All callers use a function pointer with the exact public ADL C ABI.
        let pointer = unsafe { GetProcAddress(self.0, name.as_ptr().cast()) }?;
        Some(unsafe { std::mem::transmute_copy(&pointer) })
    }
}
unsafe extern "C" fn allocate(bytes: i32) -> *mut c_void {
    if bytes <= 0 {
        return ptr::null_mut();
    }
    unsafe { libc::malloc(bytes as usize) }
}
struct Session {
    context: Context,
    destroy: Destroy,
    count: Count,
    info: Info,
    query: Option<Query>,
    temperature: Option<TemperatureGet>,
    _module: Module,
}
impl Drop for Session {
    fn drop(&mut self) {
        unsafe {
            (self.destroy)(self.context);
        }
    }
}
impl Session {
    fn open() -> Option<Self> {
        let handle = unsafe {
            LoadLibraryExW(
                wide("atiadlxx.dll").as_ptr(),
                ptr::null_mut(),
                LOAD_LIBRARY_SEARCH_SYSTEM32,
            )
        };
        if handle.is_null() {
            return None;
        }
        let module = Module(handle);
        unsafe {
            let create: Create = module.symbol(c"ADL2_Main_Control_Create")?;
            let destroy: Destroy = module.symbol(c"ADL2_Main_Control_Destroy")?;
            let count = module.symbol(c"ADL2_Adapter_NumberOfAdapters_Get")?;
            let info = module.symbol(c"ADL2_Adapter_AdapterInfo_Get")?;
            let query = module.symbol(c"ADL2_New_QueryPMLogData_Get");
            let temperature = module.symbol(c"ADL2_Overdrive5_Temperature_Get");
            let mut context = ptr::null_mut();
            // Enumerate all physical adapters, including a headless iGPU.
            if create(allocate, 0, &mut context) != 0 || context.is_null() {
                if !context.is_null() {
                    destroy(context);
                }
                return None;
            }
            Some(Self {
                context,
                destroy,
                count,
                info,
                query,
                temperature,
                _module: module,
            })
        }
    }
    fn adapters(&self) -> Vec<AdapterInfo> {
        let mut count = 0;
        unsafe {
            if (self.count)(self.context, &mut count) != 0 || !(1..=250).contains(&count) {
                return vec![];
            }
            let mut blank: AdapterInfo = std::mem::zeroed();
            blank.size = size_of::<AdapterInfo>() as i32;
            let mut adapters = vec![blank; count as usize];
            if (self.info)(
                self.context,
                adapters.as_mut_ptr(),
                (adapters.len() * size_of::<AdapterInfo>()) as i32,
            ) != 0
            {
                return vec![];
            }
            adapters.retain(|a| {
                a.exists != 0
                    && matches!(a.vendor, 1002 | 0x1002)
                    && (0..=255).contains(&a.bus)
                    && (0..=31).contains(&a.device)
                    && (0..=7).contains(&a.function)
            });
            adapters
        }
    }
    fn sample(&self, index: i32) -> (GpuAdapterTelemetry, Option<CpuTemperatureSnapshot>) {
        // A fresh zeroed response on every call prevents stale values on driver errors.
        let mut data = PmLog::default();
        let mut cpu = None;
        let mut gpu = GpuAdapterTelemetry {
            source: "AMD ADL".into(),
            ..Default::default()
        };
        if self
            .query
            .is_some_and(|query| unsafe { query(self.context, index, &mut data) == 0 })
        {
            gpu = data.gpu();
            cpu = data.cpu_temperature();
        }
        if gpu.temperature_c.is_none() {
            let mut reading = Temperature {
                size: size_of::<Temperature>() as i32,
                millidegrees: 0,
            };
            if self
                .temperature
                .is_some_and(|get| unsafe { get(self.context, index, 0, &mut reading) == 0 })
                && (1000..=150_000).contains(&reading.millidegrees)
            {
                gpu.temperature_c = Some(reading.millidegrees as f32 / 1000.0);
                gpu.temperature_source = Some("AMD ADL · GPU thermal controller".into());
            }
        }
        (gpu, cpu)
    }
}
#[derive(Default)]
pub(super) struct AmdCollector {
    session: Option<Session>,
    last_attempt: Option<Instant>,
}
impl AmdCollector {
    pub(super) fn sample(
        &mut self,
        adapters: &mut Vec<GpuAdapterSnapshot>,
    ) -> Option<CpuTemperatureSnapshot> {
        if self.session.is_none()
            && self
                .last_attempt
                .is_none_or(|t| t.elapsed() >= Duration::from_secs(30))
        {
            self.session = Session::open();
            self.last_attempt = Some(Instant::now());
        }
        let session = self.session.as_ref()?;
        let mut cpu: Option<CpuTemperatureSnapshot> = None;
        for info in session.adapters() {
            let address = format!(
                "0000:{:02x}:{:02x}.{}",
                info.bus, info.device, info.function
            );
            let (measurement, temperature) = session.sample(info.index);
            if let Some(temperature) = temperature {
                if cpu.as_ref().is_none_or(|c| temperature.celsius > c.celsius) {
                    cpu = Some(temperature);
                }
            }
            let matching: Vec<_> = adapters
                .iter_mut()
                .filter(|a| a.vendor == "AMD" && a.pci_address.as_deref() == Some(&address))
                .collect();
            if matching.len() == 1 {
                let adapter = matching.into_iter().next().unwrap();
                // Some ADL versions enumerate two handles per physical GPU. Retain
                // whichever handle successfully supplied each counter in THIS sample.
                let target = adapter
                    .telemetry
                    .get_or_insert_with(GpuAdapterTelemetry::default);
                target.source = measurement.source;
                target.utilization_percent = measurement
                    .utilization_percent
                    .or(target.utilization_percent);
                if measurement.temperature_c.is_some() {
                    target.temperature_c = measurement.temperature_c;
                    target.temperature_source = measurement.temperature_source;
                }
                target.power_watts = measurement.power_watts.or(target.power_watts);
                target.graphics_clock_mhz =
                    measurement.graphics_clock_mhz.or(target.graphics_clock_mhz);
            } else if matching.is_empty() {
                let name = String::from_utf8_lossy(
                    info.name.split(|b| *b == 0).next().unwrap_or_default(),
                )
                .into_owned();
                adapters.push(GpuAdapterSnapshot {
                    id: format!("adl:{address}"),
                    model: name,
                    vendor: "AMD".into(),
                    integrated: None,
                    pci_address: Some(address),
                    dedicated_memory_bytes: None,
                    shared_memory_limit_bytes: None,
                    source: "AMD ADL".into(),
                    telemetry: Some(measurement),
                });
            }
        }
        cpu
    }
}
