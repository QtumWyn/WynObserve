pub mod messages;
pub mod process_memory;
pub mod telemetry;

pub use messages::*;
pub use telemetry::*;
pub mod instruction_vein;
pub use instruction_vein::{InstructionVeinBatch, InstructionVeinSample};

pub const PROTOCOL_VERSION: u16 = 2;

pub const TELEMETRY_SCHEMA_VERSION: u16 = 1;
