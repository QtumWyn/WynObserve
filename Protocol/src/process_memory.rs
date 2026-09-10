use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMemoryMap {
    pub pid: u32,

    pub process_name: String,

    pub executable: Option<String>,

    pub captured_at_unix_ms: u64,

    pub total_virtual_bytes: u64,

    pub region_count: usize,

    pub writable_executable_regions: usize,

    pub overview: ProcessMemoryOverview,

    pub regions: Vec<ProcessMemoryRegion>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProcessMemoryOverview {
    pub heap_bytes: u64,

    pub anonymous_bytes: u64,

    pub shared_library_bytes: u64,

    pub mapped_file_bytes: u64,

    pub stack_bytes: u64,

    pub executable_image_bytes: u64,

    pub special_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessMemoryRegionKind {
    Heap,

    Anonymous,

    SharedLibrary,

    MappedFile,

    Stack,

    ExecutableImage,

    Special,

    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMemoryPermissions {
    pub readable: bool,

    pub writable: bool,

    pub executable: bool,

    pub private: bool,

    pub shared: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMemoryRegion {
    pub start_address: u64,

    pub end_address: u64,

    pub size_bytes: u64,

    pub permissions: ProcessMemoryPermissions,

    pub offset: u64,

    pub device_major: u32,

    pub device_minor: u32,

    pub inode: u64,

    pub pathname: Option<String>,

    pub kind: ProcessMemoryRegionKind,
}
