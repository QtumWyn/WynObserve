//! Small ownership wrappers shared by the Windows collectors.
use std::{io, mem::size_of, ptr};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE},
    System::{Diagnostics::ToolHelp::*, Performance::*, Threading::*},
};

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
pub fn text(s: &[u16]) -> String {
    String::from_utf16_lossy(&s[..s.iter().position(|c| *c == 0).unwrap_or(s.len())])
}
#[derive(Debug)]
pub struct Handle(pub HANDLE);
impl Handle {
    pub fn new(handle: HANDLE) -> io::Result<Self> {
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self(handle))
        }
    }
}
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

/// PDH handles are only accessed by the collector thread that creates this query.
#[derive(Debug)]
pub struct Counters {
    query: PDH_HQUERY,
    counters: Vec<PDH_HCOUNTER>,
}
impl Counters {
    pub fn new(paths: &[&str]) -> Option<Self> {
        let mut query = ptr::null_mut();
        if unsafe { PdhOpenQueryW(ptr::null(), 0, &mut query) } != 0 {
            return None;
        }
        let mut this = Self {
            query,
            counters: Vec::new(),
        };
        for path in paths {
            let mut counter = ptr::null_mut();
            if unsafe { PdhAddEnglishCounterW(query, wide(path).as_ptr(), 0, &mut counter) } != 0 {
                counter = ptr::null_mut();
            }
            this.counters.push(counter);
        }
        unsafe {
            PdhCollectQueryData(query);
        } // rate baseline; formatted status still checked per counter
        Some(this)
    }
    pub fn sample(&mut self) -> Vec<Option<f64>> {
        if unsafe { PdhCollectQueryData(self.query) } != 0 {
            return vec![None; self.counters.len()];
        }
        self.counters
            .iter()
            .map(|counter| {
                if counter.is_null() {
                    return None;
                }
                let mut value: PDH_FMT_COUNTERVALUE = unsafe { std::mem::zeroed() };
                let status = unsafe {
                    PdhGetFormattedCounterValue(
                        *counter,
                        PDH_FMT_DOUBLE,
                        ptr::null_mut(),
                        &mut value,
                    )
                };
                if status != 0
                    || !matches!(value.CStatus, PDH_CSTATUS_VALID_DATA | PDH_CSTATUS_NEW_DATA)
                {
                    return None;
                }
                let number = unsafe { value.Anonymous.doubleValue };
                number.is_finite().then_some(number.max(0.0))
            })
            .collect()
    }
}
impl Drop for Counters {
    fn drop(&mut self) {
        unsafe {
            PdhCloseQuery(self.query);
        }
    }
}

pub fn threads(pid: u32) -> io::Result<std::collections::HashSet<u32>> {
    let snapshot = Handle::new(unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) })?;
    let mut entry = THREADENTRY32 {
        dwSize: size_of::<THREADENTRY32>() as u32,
        ..unsafe { std::mem::zeroed() }
    };
    let mut tids = std::collections::HashSet::new();
    let mut ok = unsafe { Thread32First(snapshot.0, &mut entry) };
    while ok != 0 {
        if entry.th32OwnerProcessID == pid {
            tids.insert(entry.th32ThreadID);
        }
        entry.dwSize = size_of::<THREADENTRY32>() as u32;
        ok = unsafe { Thread32Next(snapshot.0, &mut entry) };
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() != Some(18) {
        return Err(error);
    } // ERROR_NO_MORE_FILES
    Ok(tids)
}

pub fn executable(process: HANDLE) -> Option<String> {
    let mut name = vec![0u16; 32768];
    let mut len = name.len() as u32;
    (unsafe { QueryFullProcessImageNameW(process, 0, name.as_mut_ptr(), &mut len) } != 0)
        .then(|| String::from_utf16_lossy(&name[..len as usize]))
}
