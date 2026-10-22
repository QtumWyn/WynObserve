//! Graphics identity from DXCore, including integrated AMD/Intel adapters.
//! PCI addresses join driver measurements to DXCore identity without name matching.
use crate::windows::wide;
use std::{
    ffi::c_void,
    mem::size_of,
    ptr,
    time::{Duration, Instant},
};
use windows::{
    Win32::{Foundation::LUID, Graphics::DXCore::*},
    core::{GUID, HRESULT, Interface},
};
use windows_sys::Win32::{
    Foundation::{FreeLibrary, HMODULE},
    System::LibraryLoader::*,
};
use wyn_protocol::GpuAdapterSnapshot;

#[derive(Default)]
pub(super) struct GpuInventory {
    last_refresh: Option<Instant>,
    adapters: Vec<GpuAdapterSnapshot>,
}

impl GpuInventory {
    pub(super) fn sample(&mut self) -> Vec<GpuAdapterSnapshot> {
        if self
            .last_refresh
            .is_none_or(|at| at.elapsed() >= Duration::from_secs(5))
        {
            self.adapters = discover().unwrap_or_default();
            self.last_refresh = Some(Instant::now());
        }
        self.adapters.clone()
    }
}

struct Module(HMODULE);
impl Drop for Module {
    fn drop(&mut self) {
        unsafe {
            FreeLibrary(self.0);
        }
    }
}

// Only called for the POD types specified by the DXCore property ABI.
fn property<T: Default>(adapter: &IDXCoreAdapter, id: DXCoreAdapterProperty) -> Option<T> {
    let mut result = T::default();
    unsafe {
        if !adapter.IsPropertySupported(id) || adapter.GetPropertySize(id).ok()? != size_of::<T>() {
            return None;
        }
        adapter
            .GetProperty(id, size_of::<T>(), (&mut result as *mut T).cast())
            .ok()?;
    }
    Some(result)
}

fn boolean(adapter: &IDXCoreAdapter, id: DXCoreAdapterProperty) -> Option<bool> {
    match property::<u8>(adapter, id)? {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}

fn description(adapter: &IDXCoreAdapter) -> Option<String> {
    let len = unsafe { adapter.GetPropertySize(DriverDescription).ok()? };
    if len == 0 || len > 64 * 1024 {
        return None;
    }
    let mut bytes = vec![0u8; len];
    unsafe {
        adapter
            .GetProperty(DriverDescription, len, bytes.as_mut_ptr().cast())
            .ok()?;
    }
    let text = String::from_utf8_lossy(bytes.split(|b| *b == 0).next()?)
        .trim()
        .to_owned();
    (!text.is_empty()).then_some(text)
}

fn discover() -> Option<Vec<GpuAdapterSnapshot>> {
    // Dynamic loading preserves startup on Windows versions without DXCore.
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
    let _module = Module(handle); // Created first, dropped after all COM objects.
    let entry = unsafe { GetProcAddress(handle, c"DXCoreCreateAdapterFactory".as_ptr().cast()) }?;
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
            .CreateAdapterList(&[DXCORE_ADAPTER_ATTRIBUTE_D3D11_GRAPHICS])
            .ok()?
    };
    let mut adapters = Vec::new();
    for index in 0..unsafe { list.GetAdapterCount() }.min(256) {
        let Ok(adapter): Result<IDXCoreAdapter, _> = (unsafe { list.GetAdapter(index) }) else {
            continue;
        };
        if !unsafe { adapter.IsValid() } || boolean(&adapter, IsHardware) != Some(true) {
            continue;
        }
        let Some(luid) = property::<LUID>(&adapter, InstanceLuid) else {
            continue;
        };
        let vendor_id = property::<DXCoreHardwareIDParts>(&adapter, HardwareIDParts)
            .map(|id| id.vendorID)
            .or_else(|| property::<DXCoreHardwareID>(&adapter, HardwareID).map(|id| id.vendorID));
        let vendor = match vendor_id {
            Some(0x1002) => "AMD".into(),
            Some(0x8086) => "Intel".into(),
            Some(0x10de) => "NVIDIA".into(),
            Some(id) => format!("Vendor {id:04X}"),
            None => "Not reported".into(),
        };
        adapters.push(GpuAdapterSnapshot {
            id: format!("dxcore:{:08x}{:08x}", luid.HighPart as u32, luid.LowPart),
            model: description(&adapter).unwrap_or_else(|| "Graphics adapter".into()),
            vendor,
            integrated: boolean(&adapter, IsIntegrated),
            dedicated_memory_bytes: property::<u64>(&adapter, DedicatedAdapterMemory),
            shared_memory_limit_bytes: property::<u64>(&adapter, SharedSystemMemory),
            source: "DXCore".into(),
            pci_address: pci_address(luid),
            telemetry: None,
        });
    }
    adapters.sort_by(|a, b| a.id.cmp(&b.id));
    Some(adapters)
}

// Translate the exact DXCore LUID to PCI location, including headless adapters.
fn pci_address(luid: LUID) -> Option<String> {
    use windows_sys::Wdk::Graphics::Direct3D::*;
    let mut open = D3DKMT_OPENADAPTERFROMLUID {
        AdapterLuid: windows_sys::Win32::Foundation::LUID {
            LowPart: luid.LowPart,
            HighPart: luid.HighPart,
        },
        hAdapter: 0,
    };
    unsafe {
        if D3DKMTOpenAdapterFromLuid(&mut open) < 0 {
            return None;
        }
        let mut address = D3DKMT_ADAPTERADDRESS::default();
        let mut query = D3DKMT_QUERYADAPTERINFO {
            hAdapter: open.hAdapter,
            Type: KMTQAITYPE_ADAPTERADDRESS,
            pPrivateDriverData: (&mut address as *mut D3DKMT_ADAPTERADDRESS).cast(),
            PrivateDriverDataSize: size_of::<D3DKMT_ADAPTERADDRESS>() as u32,
        };
        let status = D3DKMTQueryAdapterInfo(&mut query);
        D3DKMTCloseAdapter(&D3DKMT_CLOSEADAPTER {
            hAdapter: open.hAdapter,
        });
        (status >= 0).then(|| {
            format!(
                "0000:{:02x}:{:02x}.{}",
                address.BusNumber, address.DeviceNumber, address.FunctionNumber
            )
        })
    }
}
