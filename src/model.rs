use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeviceRecord {
    pub vendor: String,
    pub device: String,
    pub device_kind: String,
    pub parent_device: Option<String>,
    pub processor: Option<String>,
    pub processor_units: Option<u32>,
    pub core: Option<String>,
    pub core_version: Option<String>,
    pub fpu: Option<String>,
    pub endian: Option<String>,
    pub dsp: Option<String>,
    pub mve: Option<String>,
    pub trustzone: Option<String>,
    pub mpu: Option<String>,
    pub clock_hz: Option<u64>,
    pub rust_target: Option<String>,
    pub source_pack_vendor: String,
    pub source_pack_name: String,
    pub source_pack_version: Option<String>,
    pub source_url: Option<String>,
    pub source_pdsc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub schema_version: u32,
    pub record_count: usize,
    pub pdsc_file_count: usize,
    pub rust_target_resolved_count: usize,
    pub rust_target_unresolved_count: usize,
    pub source_index_sha256: Option<String>,
}
