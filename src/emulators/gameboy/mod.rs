use super::{EmuError, Emulator};
use std::path::Path;
use std::time::Duration;

/// Game Boy specific metadata
#[derive(Debug, Clone)]
pub struct GameBoyMetadata {
    pub registers: [u8; 8],  // A, F, B, C, D, E, H, L
    pub sp: u16,              // Stack pointer
    pub pc: u16,              // Program counter
    pub memory: [u8; 0x10000], // 64KB memory
}

/// Game Boy emulator (skeleton)
pub struct GameBoyEmulator {
    registers: [u8; 8],
    sp: u16,
    pc: u16,
    memory: [u8; 0x10000],
    framebuffer: [u32; 160 * 144], // Game Boy screen: 160x144
}

impl GameBoyEmulator {
    pub fn new() -> Self {
        Self {
            registers: [0; 8],
            sp: 0xFFFE,
            pc: 0x0100,
            memory: [0; 0x10000],
            framebuffer: [0xFF000000; 160 * 144], // Black screen
        }
    }
}

impl Emulator for GameBoyEmulator {
    type Metadata = GameBoyMetadata;

    fn system_name(&self) -> &'static str {
        "Game Boy"
    }

    fn load_rom(&mut self, path: &Path) -> Result<(), EmuError> {
        use std::fs::File;
        use std::io::Read;
        
        let mut file = File::open(path).map_err(|e| EmuError::RomIoError {
            rom: path.to_path_buf(),
            source: e,
        })?;

        // Load ROM into memory starting at 0x0000
        let mut rom_data = Vec::new();
        file.read_to_end(&mut rom_data).map_err(|e| EmuError::RomIoError {
            rom: path.to_path_buf(),
            source: e,
        })?;

        if rom_data.len() > 0x8000 {
            return Err(EmuError::InvalidRom {
                rom: path.to_path_buf(),
                message: "ROM file is too large (max 32KB for now)",
            });
        }

        self.memory[0..rom_data.len()].copy_from_slice(&rom_data);
        Ok(())
    }

    fn reset(&mut self) {
        self.registers = [0; 8];
        self.sp = 0xFFFE;
        self.pc = 0x0100;
        self.framebuffer = [0xFF000000; 160 * 144];
    }

    fn step(&mut self) -> Result<(), EmuError> {
        // TODO: Implement Game Boy CPU instruction execution
        // For now, just a skeleton that does nothing
        Ok(())
    }

    fn update_timers(&mut self, _delta: Duration) {
        // TODO: Implement Game Boy timers
    }

    fn framebuffer(&self) -> &[u32] {
        &self.framebuffer
    }

    fn resolution(&self) -> (usize, usize) {
        (160, 144)
    }

    fn set_input_state(&mut self, _inputs: &[bool]) {
        // TODO: Implement Game Boy input handling
    }

    fn keymap(&self) -> Vec<(usize, String)> {
        vec![
            (0, "X".to_string()),      // A button
            (1, "Z".to_string()),      // B button
            (2, "Return".to_string()), // Start
            (3, "RShift".to_string()), // Select
            (4, "Up".to_string()),     // D-pad up
            (5, "Down".to_string()),   // D-pad down
            (6, "Left".to_string()),   // D-pad left
            (7, "Right".to_string()),  // D-pad right
        ]
    }

    fn metadata(&self) -> Self::Metadata {
        GameBoyMetadata {
            registers: self.registers,
            sp: self.sp,
            pc: self.pc,
            memory: self.memory,
        }
    }
}
