use std::any::Any;
use std::path::Path;
use std::time::Duration;
use thiserror::Error;

pub mod chip8;

#[derive(Debug, Error)]
pub enum EmuError {
    /// The CPU encountered an opcode that is not implemented or unknown.
    #[error("Unrecognized opcode {opcode:#06X} at PC={pc:#010X}")]
    UnrecognizedOpcode { opcode: u64, pc: u64 },

    /// The opcode was recognized but used in an invalid or contextually incorrect way.
    #[error("Invalid opcode usage {opcode:#06X} at PC={pc:#010X}{hint}")]
    InvalidOpcodeUsage {
        opcode: u64,
        pc: u64,
        hint: &'static str,
    },

    /// Stack overflow, underflow, or out-of-bounds access.
    #[error("Invalid stack access SP={sp:#X} at PC={pc:#010X}")]
    InvalidStackAccess { sp: u64, pc: u64 },

    /// Memory access outside allowed address space.
    #[error("Invalid memory access at address {addr:#010X} (PC={pc:#010X})")]
    InvalidMemoryAccess { addr: u64, pc: u64 },

    /// Invalid register indexing (e.g., out of range, or unmapped).
    #[error("Invalid register index {index} at PC={pc:#010X}")]
    InvalidRegisterIndex { index: usize, pc: u64 },

    /// Arithmetic or logic operation error (overflow, division by zero, etc.).
    #[error("ALU error at PC={pc:#010X}: {details}")]
    AluError { pc: u64, details: &'static str },

    /// Catch-all for architecture-specific extensions.
    #[error("{message}")]
    Custom { message: &'static str },

    /// Invalid ROM is being loaded
    #[error("Loading invalid rom={rom:?}: {message}")]
    InvalidRom {
        rom: std::path::PathBuf,
        message: &'static str,
    },

    #[error("I/O error while loading ROM '{rom:?}': {source}")]
    RomIoError {
        rom: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
}

pub trait Emulator {
    /// System-specific metadata type
    type Metadata: Any + Send + Sync;

    /// Name of the system (e.g., "Chip-8", "NES")
    fn system_name(&self) -> &'static str;

    /// Load the ROM
    fn load_rom(&mut self, path: &Path) -> Result<(), EmuError>;

    /// Reset emulator
    fn reset(&mut self);

    /// Execute one CPU cycle
    fn step(&mut self) -> Result<(), EmuError>;

    /// Update timers/audio/etc.
    fn update_timers(&mut self, delta: Duration);

    /// Framebuffer as ARGB8888 pixels
    fn framebuffer(&self) -> &[u32];
    fn resolution(&self) -> (usize, usize);

    /// Input handling
    fn set_input_state(&mut self, inputs: &[bool]);

    /// Access typed metadata - returns a copy/clone
    fn metadata(&self) -> Self::Metadata;

    /// Dynamic metadata access (for UI code to downcast)
    fn metadata_any(&self) -> Box<dyn Any> {
        Box::new(self.metadata())
    }
}
