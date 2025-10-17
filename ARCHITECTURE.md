# Emulator Architecture

This document describes the architecture of the multi-emulator framework.

## Overview

The emulator framework is designed to support multiple emulation cores through a trait-based architecture. Currently, it supports CHIP-8, with the ability to easily add more emulators in the future.

## Core Components

### Emulator Trait

Located in `src/emulators/mod.rs`, the `Emulator` trait defines the interface that all emulators must implement:

```rust
pub trait Emulator {
    type Metadata: Any + Send + Sync;
    
    fn system_name(&self) -> &'static str;
    fn load_rom(&mut self, path: &Path) -> Result<(), EmuError>;
    fn reset(&mut self);
    fn step(&mut self) -> Result<(), EmuError>;
    fn update_timers(&mut self, delta: Duration);
    fn framebuffer(&self) -> &[u32];
    fn resolution(&self) -> (usize, usize);
    fn set_input_state(&mut self, inputs: &[bool]);
    fn metadata(&self) -> Self::Metadata;
}
```

### Error Handling

The `EmuError` enum provides comprehensive error handling for emulation errors:

- `UnrecognizedOpcode` - Unknown/unimplemented opcodes
- `InvalidOpcodeUsage` - Incorrect opcode usage
- `InvalidStackAccess` - Stack overflow/underflow
- `InvalidMemoryAccess` - Out-of-bounds memory access
- `InvalidRegisterIndex` - Invalid register access
- `AluError` - Arithmetic/logic errors
- `Custom` - Architecture-specific errors
- `InvalidRom` - Invalid ROM file
- `RomIoError` - I/O errors during ROM loading

## CHIP-8 Implementation

Located in `src/emulators/chip8.rs`, the CHIP-8 emulator implements the `Emulator` trait.

### Features

- Full CHIP-8 instruction set support
- 64x32 monochrome display
- 16-key hexadecimal keypad
- Delay and sound timers
- Audio beep functionality (with mute option)
- Stack support (16 levels)
- 4KB memory

### Metadata

The `Chip8Metadata` struct provides access to internal state for debugging:

```rust
pub struct Chip8Metadata {
    pub registers: [u8; 16],
    pub index_register: u16,
    pub program_counter: u16,
    pub stack: [u16; 16],
    pub stack_pointer: u8,
    pub delay_timer: u8,
    pub sound_timer: u8,
    pub current_opcode: u16,
    pub memory: [u8; 4096],
}
```

## UI Integration

The main UI (`src/main.rs`) displays:

1. **Emulator Selector** - Dropdown to choose between different emulators (currently only CHIP-8)
2. **Control Panel** - Play/Pause/Reset buttons and speed control
3. **Registers View** - All CPU registers and system state
4. **Keypad Display** - Visual representation of key states
5. **Stack Viewer** - Current stack contents
6. **Memory Viewer** - Hexdump of all memory with PC/I highlighting
7. **Disassembler** - Real-time instruction disassembly
8. **Display** - Emulator screen output

## Adding New Emulators

To add a new emulator:

1. Create a new file in `src/emulators/` (e.g., `nes.rs`)
2. Implement the `Emulator` trait
3. Define system-specific metadata struct
4. Add the module to `src/emulators/mod.rs`
5. Update the UI dropdown in `src/main.rs`

Example:

```rust
// src/emulators/nes.rs
use super::{EmuError, Emulator};

pub struct NesMetadata {
    // NES-specific state
}

pub struct NesEmulator {
    // NES-specific fields
}

impl Emulator for NesEmulator {
    type Metadata = NesMetadata;
    
    fn system_name(&self) -> &'static str {
        "NES"
    }
    
    // ... implement other methods
}
```

## Design Principles

1. **Modularity** - Each emulator is self-contained
2. **Type Safety** - Strong typing with associated metadata types
3. **Error Handling** - Comprehensive error types for debugging
4. **Performance** - Direct framebuffer access (ARGB8888)
5. **Extensibility** - Easy to add new emulators
