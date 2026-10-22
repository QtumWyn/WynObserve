use crate::windows::wide;
use std::{
    collections::HashMap,
    mem::size_of,
    ptr,
    time::{Duration, Instant},
};
use windows_sys::Win32::System::Performance::*;
struct Query {
    handle: PDH_HQUERY,
    counter: PDH_HCOUNTER,
}
impl Drop for Query {
    fn drop(&mut self) {
        unsafe {
            PdhCloseQuery(self.handle);
        }
    }
}
impl Query {
    fn open() -> Option<Self> {
        let mut handle = ptr::null_mut();
        unsafe {
            if PdhOpenQueryW(ptr::null(), 0, &mut handle) != 0 {
                return None;
            }
            let mut query = Self {
                handle,
                counter: ptr::null_mut(),
            };
            if PdhAddEnglishCounterW(
                handle,
                wide(r"\GPU Engine(*)\Utilization Percentage").as_ptr(),
                0,
                &mut query.counter,
            ) != 0
            {
                return None;
            }
            PdhCollectQueryData(handle); // first rate baseline
            Some(query)
        }
    }
    fn sample(&mut self) -> HashMap<String, f32> {
        let mut rows = vec![];
        unsafe {
            if PdhCollectQueryData(self.handle) != 0 {
                return HashMap::new();
            }
            for _ in 0..3 {
                // Re-query the required size on races; PDH does not guarantee the
                // size returned after a too-small non-null buffer is usable.
                let (mut bytes, mut count) = (0, 0);
                if PdhGetFormattedCounterArrayW(
                    self.counter,
                    PDH_FMT_DOUBLE,
                    &mut bytes,
                    &mut count,
                    ptr::null_mut(),
                ) != PDH_MORE_DATA
                {
                    break;
                }
                if bytes == 0 || bytes > 8 * 1024 * 1024 {
                    break;
                }
                // usize storage provides alignment for the C structures and strings.
                let mut buffer = vec![0usize; (bytes as usize).div_ceil(size_of::<usize>())];
                let capacity = buffer.len() * size_of::<usize>();
                let status = PdhGetFormattedCounterArrayW(
                    self.counter,
                    PDH_FMT_DOUBLE,
                    &mut bytes,
                    &mut count,
                    buffer.as_mut_ptr().cast(),
                );
                if status == PDH_MORE_DATA {
                    continue;
                }
                if status != 0
                    || count as usize > capacity / size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>()
                {
                    break;
                }
                let entries = std::slice::from_raw_parts(
                    buffer.as_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>(),
                    count as usize,
                );
                let start = buffer.as_ptr() as usize;
                for item in entries {
                    let name = item.szName as usize;
                    if name < start || name >= start + capacity || name % 2 != 0 {
                        continue;
                    }
                    let units =
                        std::slice::from_raw_parts(item.szName, (start + capacity - name) / 2);
                    let Some(end) = units.iter().position(|u| *u == 0) else {
                        continue;
                    };
                    let value = matches!(
                        item.FmtValue.CStatus,
                        PDH_CSTATUS_VALID_DATA | PDH_CSTATUS_NEW_DATA
                    )
                    .then(|| item.FmtValue.Anonymous.doubleValue);
                    rows.push((String::from_utf16_lossy(&units[..end]), value));
                }
                break;
            }
        }
        super::engine_load::aggregate(rows)
    }
}
#[derive(Default)]
pub(super) struct EngineLoad {
    query: Option<Query>,
    last_attempt: Option<Instant>,
}
impl EngineLoad {
    pub(super) fn sample(&mut self) -> HashMap<String, f32> {
        if self.query.is_none()
            && self
                .last_attempt
                .is_none_or(|t| t.elapsed() >= Duration::from_secs(30))
        {
            self.query = Query::open();
            self.last_attempt = Some(Instant::now());
            return HashMap::new(); // no zero-duration rate sample
        }
        self.query.as_mut().map(Query::sample).unwrap_or_default()
    }
}
