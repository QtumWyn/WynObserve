//! Windows equivalent of the Linux perf buffer: a private ETW sampled-profile session.
//! No target threads are suspended. Access errors are returned to the requester.
use crate::windows::{Handle, wide};
use std::{
    collections::HashMap,
    io,
    mem::{size_of, zeroed},
    ptr,
    sync::{
        Arc, Condvar, Mutex, OnceLock, Weak,
        atomic::{AtomicU64, Ordering},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::*,
    Security::*,
    System::{Diagnostics::Etw::*, Threading::*},
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct PerfSample {
    pub ip: u64,
    pub pid: u32,
    pub tid: u32,
    pub time: u64,
    pub cpu: u32,
}
#[derive(Default)]
struct Queue {
    pid: u32,
    active: bool,
    samples: Vec<PerfSample>,
}
#[derive(Default)]
struct Context {
    queues: Mutex<HashMap<u32, Queue>>,
    buffers: Mutex<u32>,
    delivered: Condvar,
}
struct Session {
    handle: CONTROLTRACE_HANDLE,
    trace: PROCESSTRACE_HANDLE,
    context: Arc<Context>,
    worker: Option<JoinHandle<()>>,
    name: Vec<u16>,
    last_flush: Mutex<Option<Instant>>,
}
// ETW handles are opaque integers. Callback state is mutex protected and the
// consumer is joined before its context is released.
static SESSION: OnceLock<Mutex<Weak<Session>>> = OnceLock::new();
static NEXT_NAME: AtomicU64 = AtomicU64::new(1);

fn properties(name: &[u16]) -> Vec<u64> {
    let bytes = size_of::<EVENT_TRACE_PROPERTIES>() + name.len() * 2;
    let mut storage = vec![0u64; bytes.div_ceil(8)];
    unsafe {
        let p = &mut *storage.as_mut_ptr().cast::<EVENT_TRACE_PROPERTIES>();
        p.Wnode.BufferSize = (storage.len() * 8) as u32;
        p.Wnode.Flags = WNODE_FLAG_TRACED_GUID; /* Wnode.Guid is assigned only when starting a private session. */
        p.Wnode.ClientContext = 1; // QPC clock; ProcessTrace converts to 100ns timestamps
        p.LogFileMode = EVENT_TRACE_REAL_TIME_MODE | EVENT_TRACE_SYSTEM_LOGGER_MODE;
        p.EnableFlags = EVENT_TRACE_FLAG_PROFILE | EVENT_TRACE_FLAG_NO_SYSCONFIG;
        p.BufferSize = 16;
        p.MinimumBuffers = 2;
        p.MaximumBuffers = 64;
        p.FlushTimer = 1;
        p.LoggerNameOffset = size_of::<EVENT_TRACE_PROPERTIES>() as u32;
        ptr::copy_nonoverlapping(
            name.as_ptr(),
            storage
                .as_mut_ptr()
                .cast::<u8>()
                .add(size_of::<EVENT_TRACE_PROPERTIES>())
                .cast(),
            name.len(),
        );
    }
    storage
}

struct ProfilePrivilege {
    token: Handle,
    previous: TOKEN_PRIVILEGES,
}
impl Drop for ProfilePrivilege {
    fn drop(&mut self) {
        unsafe {
            AdjustTokenPrivileges(
                self.token.0,
                0,
                &self.previous,
                0,
                ptr::null_mut(),
                ptr::null_mut(),
            );
        }
    }
}
fn profile_privilege() -> io::Result<ProfilePrivilege> {
    let mut token = ptr::null_mut();
    if unsafe {
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            &mut token,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let token = Handle::new(token)?;
    let mut luid = unsafe { zeroed() };
    if unsafe {
        LookupPrivilegeValueW(
            ptr::null(),
            wide("SeSystemProfilePrivilege").as_ptr(),
            &mut luid,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let privileges = TOKEN_PRIVILEGES {
        PrivilegeCount: 1,
        Privileges: [LUID_AND_ATTRIBUTES {
            Luid: luid,
            Attributes: SE_PRIVILEGE_ENABLED,
        }],
    };
    let mut previous: TOKEN_PRIVILEGES = unsafe { zeroed() };
    let mut returned = 0;
    unsafe {
        SetLastError(0);
    }
    if unsafe {
        AdjustTokenPrivileges(
            token.0,
            0,
            &privileges,
            size_of::<TOKEN_PRIVILEGES>() as u32,
            &mut previous,
            &mut returned,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let error = unsafe { GetLastError() };
    if error != 0 {
        return Err(io::Error::from_raw_os_error(error as i32));
    }
    Ok(ProfilePrivilege { token, previous })
}

impl Session {
    fn start() -> io::Result<Arc<Self>> {
        let _privilege = profile_privilege()?;
        let name = wide(&format!(
            "WynObserve-Profile-{}-{}",
            std::process::id(),
            NEXT_NAME.fetch_add(1, Ordering::Relaxed)
        ));
        let mut storage = properties(&name);
        let mut handle = CONTROLTRACE_HANDLE { Value: 0 };
        let props = unsafe { &mut *storage.as_mut_ptr().cast::<EVENT_TRACE_PROPERTIES>() };
        let guid_status =
            unsafe { windows_sys::Win32::System::Com::CoCreateGuid(&mut props.Wnode.Guid) };
        if guid_status < 0 {
            return Err(io::Error::other(format!(
                "CoCreateGuid failed: {guid_status:#x}"
            )));
        }
        let error = unsafe { StartTraceW(&mut handle, name.as_ptr(), storage.as_mut_ptr().cast()) };
        if error != 0 {
            return Err(io::Error::from_raw_os_error(error as i32));
        }
        let context = Arc::new(Context::default());
        let mut logfile: EVENT_TRACE_LOGFILEW = unsafe { zeroed() };
        logfile.LoggerName = name.as_ptr() as *mut _;
        logfile.Anonymous1.ProcessTraceMode =
            PROCESS_TRACE_MODE_REAL_TIME | PROCESS_TRACE_MODE_EVENT_RECORD;
        logfile.Anonymous2.EventRecordCallback = Some(on_event);
        logfile.BufferCallback = Some(on_buffer);
        logfile.Context = Arc::as_ptr(&context) as *mut _;
        let trace = unsafe { OpenTraceW(&mut logfile) };
        if trace.Value == u64::MAX {
            let error = io::Error::last_os_error();
            unsafe {
                ControlTraceW(
                    handle,
                    ptr::null(),
                    storage.as_mut_ptr().cast(),
                    EVENT_TRACE_CONTROL_STOP,
                );
            }
            return Err(error);
        }
        let worker = std::thread::spawn(move || unsafe {
            ProcessTrace(&trace, 1, ptr::null(), ptr::null());
        });
        Ok(Arc::new(Self {
            handle,
            trace,
            context,
            worker: Some(worker),
            name,
            last_flush: Mutex::new(None),
        }))
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        let mut props = properties(&self.name);
        unsafe {
            ControlTraceW(
                self.handle,
                ptr::null(),
                props.as_mut_ptr().cast(),
                EVENT_TRACE_CONTROL_STOP,
            );
            CloseTrace(self.trace);
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
unsafe extern "system" fn on_event(record: *mut EVENT_RECORD) {
    if record.is_null() {
        return;
    }
    let r = unsafe { &*record };
    if r.UserContext.is_null()
        || r.UserData.is_null()
        || !same_guid(r.EventHeader.ProviderId, PerfInfoGuid)
        || r.EventHeader.EventDescriptor.Opcode != 46
    {
        return;
    }
    let pointer_size = if r.EventHeader.Flags as u32 & EVENT_HEADER_FLAG_32_BIT_HEADER != 0 {
        4
    } else {
        8
    };
    if (r.UserDataLength as usize) < pointer_size + 4 {
        return;
    }
    let bytes =
        unsafe { std::slice::from_raw_parts(r.UserData.cast::<u8>(), r.UserDataLength as usize) };
    let ip = if pointer_size == 8 {
        u64::from_le_bytes(bytes[..8].try_into().unwrap())
    } else {
        u32::from_le_bytes(bytes[..4].try_into().unwrap()) as u64
    };
    let tid = u32::from_le_bytes(bytes[pointer_size..pointer_size + 4].try_into().unwrap());
    let context = unsafe { &*r.UserContext.cast::<Context>() };
    let Ok(mut queues) = context.queues.lock() else {
        return;
    };
    if let Some(queue) = queues.get_mut(&tid).filter(|q| q.active) {
        if queue.samples.len() < 4096 {
            // Perf samples expose nanosecond-relative differences. Avoid overflow of epoch nanoseconds.
            let timestamp = r
                .EventHeader
                .TimeStamp
                .saturating_sub(116_444_736_000_000_000)
                .max(0) as u64;
            queue.samples.push(PerfSample {
                ip,
                pid: queue.pid,
                tid,
                time: timestamp.saturating_mul(100),
                cpu: unsafe { r.BufferContext.Anonymous.ProcessorIndex } as u32,
            });
        }
    }
}

fn same_guid(a: windows_sys::core::GUID, b: windows_sys::core::GUID) -> bool {
    a.data1 == b.data1 && a.data2 == b.data2 && a.data3 == b.data3 && a.data4 == b.data4
}
unsafe extern "system" fn on_buffer(log: *mut EVENT_TRACE_LOGFILEW) -> u32 {
    if log.is_null() {
        return 1;
    }
    let log = unsafe { &*log };
    if !log.Context.is_null() {
        let context = unsafe { &*log.Context.cast::<Context>() };
        *context.buffers.lock().unwrap_or_else(|e| e.into_inner()) = log.BuffersRead;
        context.delivered.notify_all();
    }
    1
}
impl Session {
    fn flush(&self) -> io::Result<()> {
        let mut last = self.last_flush.lock().unwrap_or_else(|e| e.into_inner());
        // A process poll drains many thread queues from the same ETW session.
        if last.is_some_and(|at| at.elapsed() < Duration::from_millis(50)) {
            return Ok(());
        }
        let mut props = properties(&self.name);
        let error = unsafe {
            ControlTraceW(
                self.handle,
                ptr::null(),
                props.as_mut_ptr().cast(),
                EVENT_TRACE_CONTROL_FLUSH,
            )
        };
        if error != 0 {
            return Err(io::Error::from_raw_os_error(error as i32));
        }
        let written = unsafe { &*props.as_ptr().cast::<EVENT_TRACE_PROPERTIES>() }.BuffersWritten;
        let read = self
            .context
            .buffers
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        // FLUSH queues delivery; it does not synchronously run the callbacks.
        // Wait for the consumer's buffer callback before draining any target queue.
        let (read, _) = self
            .context
            .delivered
            .wait_timeout_while(read, Duration::from_secs(2), |read| *read < written)
            .unwrap_or_else(|e| e.into_inner());
        if *read < written {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "ETW consumer did not finish the flushed buffers",
            ));
        }
        *last = Some(Instant::now());
        Ok(())
    }
}

pub(crate) struct PerfBuffer {
    session: Arc<Session>,
    tid: u32,
}
impl PerfBuffer {
    pub fn reset(&self) -> io::Result<()> {
        if let Some(q) = self
            .session
            .context
            .queues
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_mut(&self.tid)
        {
            q.samples.clear();
        }
        Ok(())
    }
    pub fn enable(&self) -> io::Result<()> {
        if let Some(q) = self
            .session
            .context
            .queues
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_mut(&self.tid)
        {
            q.active = true;
        }
        Ok(())
    }
    pub fn disable(&self) -> io::Result<()> {
        if let Some(q) = self
            .session
            .context
            .queues
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_mut(&self.tid)
        {
            q.active = false;
        }
        Ok(())
    }
    pub fn drain_samples(&mut self) -> io::Result<Vec<PerfSample>> {
        self.session.flush()?;
        Ok(self
            .session
            .context
            .queues
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_mut(&self.tid)
            .map(|q| std::mem::take(&mut q.samples))
            .unwrap_or_default())
    }
}
impl Drop for PerfBuffer {
    fn drop(&mut self) {
        self.session
            .context
            .queues
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&self.tid);
    }
}
pub(crate) fn map_probe_for_tid(tid: u32) -> io::Result<PerfBuffer> {
    let thread = Handle::new(unsafe { OpenThread(THREAD_QUERY_LIMITED_INFORMATION, 0, tid) })?;
    let pid = unsafe { GetProcessIdOfThread(thread.0) };
    if pid == 0 {
        return Err(io::Error::last_os_error());
    }
    let process = Handle::new(unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) })?;
    let mut wow64 = 0;
    if unsafe { IsWow64Process(process.0, &mut wow64) } == 0 {
        return Err(io::Error::last_os_error());
    }
    if wow64 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Instruction Vein currently decodes x86-64 targets only",
        ));
    }
    let mut shared = SESSION
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let session = match shared.upgrade() {
        Some(session) => session,
        None => {
            let session = Session::start()?;
            *shared = Arc::downgrade(&session);
            session
        }
    };
    session
        .context
        .queues
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(
            tid,
            Queue {
                pid,
                ..Default::default()
            },
        );
    Ok(PerfBuffer { session, tid })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profile_events_are_filtered_and_decoded() {
        let context = Context::default();
        context.queues.lock().unwrap().insert(
            42,
            Queue {
                pid: 7,
                active: true,
                samples: vec![],
            },
        );
        let mut payload = [0u8; 16];
        payload[..8].copy_from_slice(&0x12345678u64.to_le_bytes());
        payload[8..12].copy_from_slice(&42u32.to_le_bytes());
        let mut record: EVENT_RECORD = unsafe { zeroed() };
        record.UserContext = (&context as *const Context).cast_mut().cast();
        record.UserData = payload.as_mut_ptr().cast();
        record.UserDataLength = 16;
        record.EventHeader.ProviderId = PerfInfoGuid;
        record.EventHeader.EventDescriptor.Opcode = 46;
        record.EventHeader.TimeStamp = 116_444_736_000_000_000 + 100;
        unsafe {
            on_event(&mut record);
        }
        let samples = &context.queues.lock().unwrap()[&42].samples;
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].pid, 7);
        assert_eq!(samples[0].tid, 42);
        assert_eq!(samples[0].ip, 0x12345678);
        assert_eq!(samples[0].time, 10000);
    }
}
