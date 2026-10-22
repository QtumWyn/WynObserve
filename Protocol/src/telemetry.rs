use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySnapshot {
    pub schema_version: u16,
    pub captured_at_unix_ms: u64,

    /// Firmware identity and inventory; absent on older Agents. No serials or UUIDs.
    #[serde(default)]
    pub motherboard: Option<MotherboardSnapshot>,

    pub cpu: CpuSnapshot,
    pub memory: MemorySnapshot,
    pub npu: NpuSnapshot,
    pub scheduler: SchedulerSnapshot,
    pub gpu: GpuSnapshot,
    pub storage: StorageSnapshot,
    pub network: NetworkSnapshot,

    pub processes_available: bool,
    pub process_count: usize,
    pub thread_count: usize,

    pub processes: Vec<ProcessSnapshot>,
}

/// Static firmware inventory, refreshed when the Agent starts.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MotherboardSnapshot {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub version: Option<String>,
    pub system_manufacturer: Option<String>,
    pub system_model: Option<String>,
    /// SMBIOS chassis type (lock bit removed); not a motherboard form factor.
    pub chassis_type: Option<u8>,
    pub source: String,
    /// System-memory device positions reported by SMBIOS Type 16.
    pub memory_slot_count: Option<u16>,
    pub memory_devices: Vec<MemoryDeviceSnapshot>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoryDeviceSnapshot {
    pub locator: Option<String>,
    pub bank: Option<String>,
    /// None means unknown, Some(0) means an explicitly empty socket.
    pub size_bytes: Option<u64>,
    /// Raw SMBIOS enums: DIMM=9, SODIMM=13; other values can be soldered memory.
    pub form_factor: Option<u8>,
    pub memory_type: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuSnapshot {
    pub vendor: String,
    pub brand: String,

    pub global_usage_percent: f32,

    pub logical_cpu_count: usize,

    #[serde(default)]
    pub physical_core_count: Option<usize>,

    /// Current CPU sensor reading; absent on older Agents or unavailable hardware.
    #[serde(default)]
    pub temperature: Option<CpuTemperatureSnapshot>,

    pub logical_cpus: Vec<LogicalCpuSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuTemperatureSnapshot {
    pub celsius: f32,
    /// Driver and sensor label. Multiple CPU devices use the hottest reading.
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicalCpuSnapshot {
    pub logical_id: usize,
    pub usage_percent: f32,
    pub frequency_mhz: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,

    pub cached_bytes: u64,
    pub active_bytes: u64,
    pub dirty_bytes: u64,

    pub page_faults_per_second: u64,
    pub major_page_faults_per_second: u64,

    pub total_swap_bytes: u64,
    pub used_swap_bytes: u64,

    pub memory_detail_available: bool,
    pub fault_rates_available: bool,
    #[serde(default)]
    pub major_fault_rate_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpuSnapshot {
    pub available: bool,
    #[serde(default)]
    pub utilization_available: bool,
    #[serde(default)]
    pub frequency_available: bool,
    #[serde(default)]
    pub memory_available: bool,

    pub model: String,
    pub pci_address: String,

    pub utilization_percent: f32,

    pub current_frequency_mhz: u64,
    pub max_frequency_mhz: u64,

    pub memory_used_bytes: u64,

    pub power_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerSnapshot {
    pub context_switches_per_second: u64,

    #[serde(default)]
    pub runnable_tasks_available: bool,
    #[serde(default)]
    pub blocked_tasks_available: bool,
    pub runnable_tasks: u64,
    pub blocked_tasks: u64,

    pub context_switch_rate_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuSnapshot {
    pub available: bool,

    /// Device inventory with independently available per-adapter measurements.
    #[serde(default)]
    pub adapters: Vec<GpuAdapterSnapshot>,
    #[serde(default)]
    pub primary_adapter_id: Option<String>,

    pub model: String,
    pub uuid: String,
    pub pci_address: String,

    pub utilization_percent: f32,
    pub memory_activity_percent: f32,

    pub vram_total_bytes: u64,
    pub vram_used_bytes: u64,

    pub temperature_c: f32,
    pub power_watts: f32,

    pub graphics_clock_mhz: Option<u32>,
    pub sm_clock_mhz: Option<u32>,
    pub memory_clock_mhz: Option<u32>,
    pub video_clock_mhz: Option<u32>,

    pub performance_state: Option<String>,
    pub fan_percent: Option<u32>,
    pub power_limit_watts: Option<f32>,

    pub pcie_rx_mib_s: Option<f32>,
    pub pcie_tx_mib_s: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuAdapterSnapshot {
    #[serde(default)]
    pub pci_address: Option<String>,
    #[serde(default)]
    pub telemetry: Option<GpuAdapterTelemetry>,
    pub id: String,
    pub model: String,
    pub vendor: String,
    pub integrated: Option<bool>,
    pub dedicated_memory_bytes: Option<u64>,
    /// Maximum shared system memory available to this adapter, NOT current use.
    pub shared_memory_limit_bytes: Option<u64>,
    pub source: String,
}

/// Missing fields mean unsupported or failed this sample; zero is a measured value.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GpuAdapterTelemetry {
    pub source: String,
    pub utilization_percent: Option<f32>,
    pub temperature_c: Option<f32>,
    pub temperature_source: Option<String>,
    pub memory_used_bytes: Option<u64>,
    pub power_watts: Option<f32>,
    pub graphics_clock_mhz: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSnapshot {
    pub available: bool,

    pub device: String,
    pub model: String,

    pub read_mib_s: f32,
    pub write_mib_s: f32,

    pub read_iops: f32,
    pub write_iops: f32,

    pub utilization_percent: f32,

    pub io_in_progress: u64,
    pub average_queue_depth: f32,

    pub rates_available: bool,

    pub space_available: bool,

    pub mount_point: String,

    pub capacity_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSnapshot {
    pub available: bool,

    pub interface: String,

    pub rx_mib_s: f32,
    pub tx_mib_s: f32,

    pub rx_packets_per_second: u64,
    pub tx_packets_per_second: u64,

    pub rx_errors_per_second: u64,
    pub tx_errors_per_second: u64,

    pub rx_drops_per_second: u64,
    pub tx_drops_per_second: u64,

    pub connections: usize,

    pub rates_available: bool,
    pub connections_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessSnapshot {
    pub pid: u32,
    pub parent_pid: u32,

    pub name: String,
    pub executable: Option<String>,
    pub state: String,

    pub cpu_usage_percent: f32,
    pub cpu_rate_available: bool,

    pub memory_bytes: u64,
    pub memory_available: bool,

    pub last_cpu: usize,
    pub threads: usize,

    pub read_mib_s: f32,
    pub write_mib_s: f32,

    pub io_rates_available: bool,

    pub started_at_unix_ms: Option<u64>,
}

#[cfg(test)]
mod compatibility_tests {
    #[test]
    fn motherboard_is_optional_and_survives_full_snapshot_relays() {
        // Minimal old Agent packet, including all original required fields.
        let mut json: serde_json::Value = serde_json::from_str(
            r#"{
  "schema_version": 0,
  "captured_at_unix_ms": 0,
  "cpu": {
    "vendor": "",
    "brand": "",
    "global_usage_percent": 0,
    "logical_cpu_count": 0,
    "physical_core_count": null,
    "temperature": null,
    "logical_cpus": []
  },
  "memory": {
    "total_bytes": 0,
    "used_bytes": 0,
    "available_bytes": 0,
    "cached_bytes": 0,
    "active_bytes": 0,
    "dirty_bytes": 0,
    "page_faults_per_second": 0,
    "major_page_faults_per_second": 0,
    "total_swap_bytes": 0,
    "used_swap_bytes": 0,
    "memory_detail_available": false,
    "fault_rates_available": false,
    "major_fault_rate_available": false
  },
  "npu": {
    "available": false,
    "utilization_available": false,
    "frequency_available": false,
    "memory_available": false,
    "model": "",
    "pci_address": "",
    "utilization_percent": 0,
    "current_frequency_mhz": 0,
    "max_frequency_mhz": 0,
    "memory_used_bytes": 0,
    "power_state": ""
  },
  "scheduler": {
    "context_switches_per_second": 0,
    "runnable_tasks_available": false,
    "blocked_tasks_available": false,
    "runnable_tasks": 0,
    "blocked_tasks": 0,
    "context_switch_rate_available": false
  },
  "gpu": {
    "available": false,
    "adapters": [],
    "primary_adapter_id": null,
    "model": "",
    "uuid": "",
    "pci_address": "",
    "utilization_percent": 0,
    "memory_activity_percent": 0,
    "vram_total_bytes": 0,
    "vram_used_bytes": 0,
    "temperature_c": 0,
    "power_watts": 0,
    "graphics_clock_mhz": null,
    "sm_clock_mhz": null,
    "memory_clock_mhz": null,
    "video_clock_mhz": null,
    "performance_state": null,
    "fan_percent": null,
    "power_limit_watts": null,
    "pcie_rx_mib_s": null,
    "pcie_tx_mib_s": null
  },
  "storage": {
    "available": false,
    "device": "",
    "model": "",
    "read_mib_s": 0,
    "write_mib_s": 0,
    "read_iops": 0,
    "write_iops": 0,
    "utilization_percent": 0,
    "io_in_progress": 0,
    "average_queue_depth": 0,
    "rates_available": false,
    "space_available": false,
    "mount_point": "",
    "capacity_bytes": 0,
    "used_bytes": 0,
    "available_bytes": 0
  },
  "network": {
    "available": false,
    "interface": "",
    "rx_mib_s": 0,
    "tx_mib_s": 0,
    "rx_packets_per_second": 0,
    "tx_packets_per_second": 0,
    "rx_errors_per_second": 0,
    "tx_errors_per_second": 0,
    "rx_drops_per_second": 0,
    "tx_drops_per_second": 0,
    "connections": 0,
    "rates_available": false,
    "connections_available": false
  },
  "processes_available": false,
  "process_count": 0,
  "thread_count": 0,
  "processes": []
}"#,
        )
        .unwrap();
        let old: super::TelemetrySnapshot = serde_json::from_value(json.clone()).unwrap();
        assert!(old.motherboard.is_none());
        json["motherboard"] = serde_json::json!({"manufacturer":"Gigabyte", "model":"B860M DS3H WIFI6E",
            "source":"Windows SMBIOS", "memory_slot_count":4, "memory_devices":[
                {"locator":"DIMM_A1", "size_bytes":0}, {"locator":"DIMM_A2", "size_bytes":17179869184u64},
                {"locator":"DIMM_B1", "size_bytes":null}]});
        let snapshot: super::TelemetrySnapshot = serde_json::from_value(json).unwrap();
        let relayed: super::TelemetrySnapshot =
            serde_json::from_slice(&serde_json::to_vec(&snapshot).unwrap()).unwrap();
        let board = relayed.motherboard.unwrap();
        assert_eq!(board.model.as_deref(), Some("B860M DS3H WIFI6E"));
        assert_eq!(board.memory_slot_count, Some(4));
        assert_eq!(
            board
                .memory_devices
                .iter()
                .map(|m| m.size_bytes)
                .collect::<Vec<_>>(),
            vec![Some(0), Some(17179869184), None]
        );
    }

    #[test]
    fn cpu_temperature_is_optional_for_older_agents_and_round_trips_when_present() {
        let mut old = serde_json::json!({"vendor":"GenuineIntel","brand":"CPU","global_usage_percent":12.0,
            "logical_cpu_count":2,"logical_cpus":[]});
        let cpu: super::CpuSnapshot = serde_json::from_value(old.clone()).unwrap();
        assert!(cpu.temperature.is_none());
        old["temperature"] =
            serde_json::json!({"celsius":45.125,"source":"coretemp · Package id 0"});
        let cpu: super::CpuSnapshot = serde_json::from_value(old).unwrap();
        let restored: super::CpuSnapshot =
            serde_json::from_str(&serde_json::to_string(&cpu).unwrap()).unwrap();
        let temperature = restored.temperature.unwrap();
        assert_eq!(temperature.celsius, 45.125);
        assert_eq!(temperature.source, "coretemp · Package id 0");
    }

    #[test]
    fn gpu_inventory_is_optional_and_independent_of_nvml_availability() {
        let mut old = serde_json::json!({"available":false,"model":"GPU unavailable","uuid":"","pci_address":"",
            "utilization_percent":0.0,"memory_activity_percent":0.0,"vram_total_bytes":0,"vram_used_bytes":0,
            "temperature_c":0.0,"power_watts":0.0});
        let gpu: super::GpuSnapshot = serde_json::from_value(old.clone()).unwrap();
        assert!(gpu.adapters.is_empty());
        old["adapters"] = serde_json::json!([{"id":"dxcore:123","model":"AMD Radeon Graphics","vendor":"AMD",
            "integrated":true,"dedicated_memory_bytes":536870912,"shared_memory_limit_bytes":17179869184u64,"source":"DXCore"}]);
        let gpu: super::GpuSnapshot = serde_json::from_value(old).unwrap();
        let restored: super::GpuSnapshot =
            serde_json::from_str(&serde_json::to_string(&gpu).unwrap()).unwrap();
        assert!(!restored.available);
        assert_eq!(restored.adapters.len(), 1);
        assert_eq!(restored.adapters[0].integrated, Some(true));
    }

    #[test]
    fn old_npu_snapshots_default_new_availability_flags() {
        let old = serde_json::json!({"available":true,"model":"NPU","pci_address":"", "utilization_percent":0.0,
            "current_frequency_mhz":0,"max_frequency_mhz":0,"memory_used_bytes":0,"power_state":"unknown"});
        let snapshot: super::NpuSnapshot = serde_json::from_value(old).unwrap();
        assert!(snapshot.available);
        assert!(
            !snapshot.utilization_available
                && !snapshot.frequency_available
                && !snapshot.memory_available
        );
    }
    #[test]
    fn per_adapter_sensor_options_survive_a_typed_relay() {
        let json = serde_json::json!({"id":"radeon", "model":"Radeon Graphics", "vendor":"AMD", "integrated":true,
            "dedicated_memory_bytes":null,"shared_memory_limit_bytes":17179869184u64,"source":"DXCore",
            "pci_address":"0000:0e:00.0", "telemetry":{"source":"AMD ADL + Windows GPU Engine",
                "utilization_percent":0.0,"temperature_c":47.0,"temperature_source":"GPU edge",
                "memory_used_bytes":null,"power_watts":null,"graphics_clock_mhz":400}});
        let adapter: super::GpuAdapterSnapshot = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(serde_json::to_value(adapter).unwrap(), json);
    }
}
