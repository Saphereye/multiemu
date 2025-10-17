use super::{EmuError, Emulator};
use std::path::Path;
use std::time::Duration;

const WIDTH: usize = 160;
const HEIGHT: usize = 144;

// Game Boy CPU registers
const REG_A: usize = 0;
const REG_F: usize = 1;
const REG_B: usize = 2;
const REG_C: usize = 3;
const REG_D: usize = 4;
const REG_E: usize = 5;
const REG_H: usize = 6;
const REG_L: usize = 7;

// Flag register bits
const FLAG_Z: u8 = 0b10000000; // Zero flag
const FLAG_N: u8 = 0b01000000; // Subtract flag
const FLAG_H: u8 = 0b00100000; // Half-carry flag
const FLAG_C: u8 = 0b00010000; // Carry flag

/// Game Boy specific metadata
#[derive(Debug, Clone)]
pub struct GameBoyMetadata {
    pub registers: [u8; 8],  // A, F, B, C, D, E, H, L
    pub sp: u16,              // Stack pointer
    pub pc: u16,              // Program counter
    pub memory: Vec<u8>,      // Subset of memory for display (first 64KB or less)
    pub current_opcode: u8,   // Current opcode being executed
    pub ime: bool,            // Interrupt Master Enable
}

/// Game Boy emulator
pub struct GameBoyEmulator {
    registers: [u8; 8],       // A, F, B, C, D, E, H, L
    sp: u16,                  // Stack pointer
    pc: u16,                  // Program counter
    memory: [u8; 0x10000],    // 64KB memory
    framebuffer: [u32; WIDTH * HEIGHT],
    current_opcode: u8,
    ime: bool,                // Interrupt Master Enable
    input_keys: [bool; 8],    // 8 buttons
    rom_loaded: bool,
    cycles: u64,              // CPU cycle counter
    // Simple tile-based rendering for POC
    vram: [u8; 0x2000],       // 8KB Video RAM
    oam: [u8; 0xA0],          // Object Attribute Memory (sprites)
}

impl GameBoyEmulator {
    pub fn new() -> Self {
        Self {
            registers: [0; 8],
            sp: 0xFFFE,
            pc: 0x0100,
            memory: [0; 0x10000],
            framebuffer: [0xFF9BBC0F; WIDTH * HEIGHT], // Game Boy green
            current_opcode: 0,
            ime: true,
            input_keys: [false; 8],
            rom_loaded: false,
            cycles: 0,
            vram: [0; 0x2000],
            oam: [0; 0xA0],
        }
    }

    // Helper functions for 16-bit register pairs
    fn get_bc(&self) -> u16 {
        ((self.registers[REG_B] as u16) << 8) | (self.registers[REG_C] as u16)
    }

    fn set_bc(&mut self, value: u16) {
        self.registers[REG_B] = (value >> 8) as u8;
        self.registers[REG_C] = (value & 0xFF) as u8;
    }

    fn get_de(&self) -> u16 {
        ((self.registers[REG_D] as u16) << 8) | (self.registers[REG_E] as u16)
    }

    fn set_de(&mut self, value: u16) {
        self.registers[REG_D] = (value >> 8) as u8;
        self.registers[REG_E] = (value & 0xFF) as u8;
    }

    fn get_hl(&self) -> u16 {
        ((self.registers[REG_H] as u16) << 8) | (self.registers[REG_L] as u16)
    }

    fn set_hl(&mut self, value: u16) {
        self.registers[REG_H] = (value >> 8) as u8;
        self.registers[REG_L] = (value & 0xFF) as u8;
    }

    fn get_af(&self) -> u16 {
        ((self.registers[REG_A] as u16) << 8) | (self.registers[REG_F] as u16)
    }

    fn set_af(&mut self, value: u16) {
        self.registers[REG_A] = (value >> 8) as u8;
        self.registers[REG_F] = (value & 0xF0) as u8; // Lower 4 bits always 0
    }

    // Flag helpers
    fn get_flag(&self, flag: u8) -> bool {
        (self.registers[REG_F] & flag) != 0
    }

    fn set_flag(&mut self, flag: u8, value: bool) {
        if value {
            self.registers[REG_F] |= flag;
        } else {
            self.registers[REG_F] &= !flag;
        }
    }

    // Memory read/write with memory-mapped I/O handling
    fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7FFF => self.memory[addr as usize], // ROM
            0x8000..=0x9FFF => self.vram[(addr - 0x8000) as usize], // VRAM
            0xA000..=0xBFFF => self.memory[addr as usize], // External RAM
            0xC000..=0xDFFF => self.memory[addr as usize], // Work RAM
            0xE000..=0xFDFF => self.memory[(addr - 0x2000) as usize], // Echo RAM
            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize], // OAM
            0xFF00..=0xFFFF => self.memory[addr as usize], // I/O and High RAM
            _ => 0xFF,
        }
    }

    fn write_byte(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x7FFF => {}, // ROM, read-only (banking would go here)
            0x8000..=0x9FFF => self.vram[(addr - 0x8000) as usize] = value,
            0xA000..=0xBFFF => self.memory[addr as usize] = value,
            0xC000..=0xDFFF => self.memory[addr as usize] = value,
            0xE000..=0xFDFF => self.memory[(addr - 0x2000) as usize] = value,
            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize] = value,
            0xFF00..=0xFFFF => self.memory[addr as usize] = value,
            _ => {},
        }
    }

    fn read_word(&self, addr: u16) -> u16 {
        let lo = self.read_byte(addr) as u16;
        let hi = self.read_byte(addr.wrapping_add(1)) as u16;
        (hi << 8) | lo
    }

    fn write_word(&mut self, addr: u16, value: u16) {
        self.write_byte(addr, (value & 0xFF) as u8);
        self.write_byte(addr.wrapping_add(1), (value >> 8) as u8);
    }

    // Stack operations
    fn push(&mut self, value: u16) {
        self.sp = self.sp.wrapping_sub(2);
        self.write_word(self.sp, value);
    }

    fn pop(&mut self) -> u16 {
        let value = self.read_word(self.sp);
        self.sp = self.sp.wrapping_add(2);
        value
    }

    // Simple tile rendering for POC
    fn render_background(&mut self) {
        // For POC, just render a simple test pattern based on VRAM
        // Real implementation would decode tiles and tilemaps
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let tile_x = x / 8;
                let tile_y = y / 8;
                let tile_index = (tile_y * 20 + tile_x) % 0x2000;
                
                // Simple pattern based on VRAM content
                let pixel_on = (self.vram[tile_index] & (1 << (x % 8))) != 0;
                let color = if pixel_on {
                    0xFF0F380F // Dark green
                } else {
                    0xFF9BBC0F // Light green
                };
                
                self.framebuffer[y * WIDTH + x] = color;
            }
        }
    }

    // Execute one instruction
    fn execute_instruction(&mut self) -> Result<(), EmuError> {
        if !self.rom_loaded {
            return Ok(());
        }

        self.current_opcode = self.read_byte(self.pc);
        let opcode = self.current_opcode;
        self.pc = self.pc.wrapping_add(1);

        // Basic instruction set implementation
        match opcode {
            // NOP
            0x00 => {},
            
            // LD BC, d16
            0x01 => {
                let value = self.read_word(self.pc);
                self.pc = self.pc.wrapping_add(2);
                self.set_bc(value);
            },
            
            // LD (BC), A
            0x02 => {
                let addr = self.get_bc();
                self.write_byte(addr, self.registers[REG_A]);
            },
            
            // INC BC
            0x03 => {
                let value = self.get_bc().wrapping_add(1);
                self.set_bc(value);
            },
            
            // INC B
            0x04 => {
                let result = self.registers[REG_B].wrapping_add(1);
                self.set_flag(FLAG_Z, result == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, (self.registers[REG_B] & 0x0F) == 0x0F);
                self.registers[REG_B] = result;
            },
            
            // DEC B
            0x05 => {
                let result = self.registers[REG_B].wrapping_sub(1);
                self.set_flag(FLAG_Z, result == 0);
                self.set_flag(FLAG_N, true);
                self.set_flag(FLAG_H, (self.registers[REG_B] & 0x0F) == 0);
                self.registers[REG_B] = result;
            },
            
            // LD B, d8
            0x06 => {
                self.registers[REG_B] = self.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
            },
            
            // LD A, (BC)
            0x0A => {
                let addr = self.get_bc();
                self.registers[REG_A] = self.read_byte(addr);
            },
            
            // LD DE, d16
            0x11 => {
                let value = self.read_word(self.pc);
                self.pc = self.pc.wrapping_add(2);
                self.set_de(value);
            },
            
            // LD (DE), A
            0x12 => {
                let addr = self.get_de();
                self.write_byte(addr, self.registers[REG_A]);
            },
            
            // INC DE
            0x13 => {
                let value = self.get_de().wrapping_add(1);
                self.set_de(value);
            },
            
            // LD E, d8
            0x1E => {
                self.registers[REG_E] = self.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
            },
            
            // LD HL, d16
            0x21 => {
                let value = self.read_word(self.pc);
                self.pc = self.pc.wrapping_add(2);
                self.set_hl(value);
            },
            
            // LD (HL+), A
            0x22 => {
                let addr = self.get_hl();
                self.write_byte(addr, self.registers[REG_A]);
                self.set_hl(addr.wrapping_add(1));
            },
            
            // INC HL
            0x23 => {
                let value = self.get_hl().wrapping_add(1);
                self.set_hl(value);
            },
            
            // LD L, d8
            0x2E => {
                self.registers[REG_L] = self.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
            },
            
            // LD SP, d16
            0x31 => {
                self.sp = self.read_word(self.pc);
                self.pc = self.pc.wrapping_add(2);
            },
            
            // LD (HL-), A
            0x32 => {
                let addr = self.get_hl();
                self.write_byte(addr, self.registers[REG_A]);
                self.set_hl(addr.wrapping_sub(1));
            },
            
            // INC SP
            0x33 => {
                self.sp = self.sp.wrapping_add(1);
            },
            
            // LD A, d8
            0x3E => {
                self.registers[REG_A] = self.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
            },
            
            // LD B, B through LD A, A (most LD r, r instructions)
            0x40..=0x7F => {
                let to = ((opcode >> 3) & 0x07) as usize;
                let from = (opcode & 0x07) as usize;
                
                // HALT is 0x76
                if opcode == 0x76 {
                    // HALT - for now, just continue
                    return Ok(());
                }
                
                // (HL) addressing
                if from == 6 {
                    let addr = self.get_hl();
                    let value = self.read_byte(addr);
                    self.registers[to] = value;
                } else if to == 6 {
                    let addr = self.get_hl();
                    self.write_byte(addr, self.registers[from]);
                } else {
                    self.registers[to] = self.registers[from];
                }
            },
            
            // ADD A, r
            0x80..=0x87 => {
                let reg = (opcode & 0x07) as usize;
                let value = if reg == 6 {
                    let addr = self.get_hl();
                    self.read_byte(addr)
                } else {
                    self.registers[reg]
                };
                
                let a = self.registers[REG_A];
                let result = a.wrapping_add(value);
                
                self.set_flag(FLAG_Z, result == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, (a & 0x0F) + (value & 0x0F) > 0x0F);
                self.set_flag(FLAG_C, (a as u16) + (value as u16) > 0xFF);
                
                self.registers[REG_A] = result;
            },
            
            // SUB A, r
            0x90..=0x97 => {
                let reg = (opcode & 0x07) as usize;
                let value = if reg == 6 {
                    let addr = self.get_hl();
                    self.read_byte(addr)
                } else {
                    self.registers[reg]
                };
                
                let a = self.registers[REG_A];
                let result = a.wrapping_sub(value);
                
                self.set_flag(FLAG_Z, result == 0);
                self.set_flag(FLAG_N, true);
                self.set_flag(FLAG_H, (a & 0x0F) < (value & 0x0F));
                self.set_flag(FLAG_C, a < value);
                
                self.registers[REG_A] = result;
            },
            
            // XOR A, r
            0xA8..=0xAF => {
                let reg = (opcode & 0x07) as usize;
                let value = if reg == 6 {
                    let addr = self.get_hl();
                    self.read_byte(addr)
                } else {
                    self.registers[reg]
                };
                
                self.registers[REG_A] ^= value;
                self.set_flag(FLAG_Z, self.registers[REG_A] == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, false);
            },
            
            // CP A, r
            0xB8..=0xBF => {
                let reg = (opcode & 0x07) as usize;
                let value = if reg == 6 {
                    let addr = self.get_hl();
                    self.read_byte(addr)
                } else {
                    self.registers[reg]
                };
                
                let a = self.registers[REG_A];
                let result = a.wrapping_sub(value);
                
                self.set_flag(FLAG_Z, result == 0);
                self.set_flag(FLAG_N, true);
                self.set_flag(FLAG_H, (a & 0x0F) < (value & 0x0F));
                self.set_flag(FLAG_C, a < value);
            },
            
            // RET (conditional)
            0xC0 | 0xC8 | 0xD0 | 0xD8 => {
                let condition = match opcode {
                    0xC0 => !self.get_flag(FLAG_Z), // RET NZ
                    0xC8 => self.get_flag(FLAG_Z),  // RET Z
                    0xD0 => !self.get_flag(FLAG_C), // RET NC
                    0xD8 => self.get_flag(FLAG_C),  // RET C
                    _ => unreachable!(),
                };
                
                if condition {
                    self.pc = self.pop();
                }
            },
            
            // POP BC/DE/HL/AF
            0xC1 | 0xD1 | 0xE1 | 0xF1 => {
                let value = self.pop();
                match opcode {
                    0xC1 => self.set_bc(value),
                    0xD1 => self.set_de(value),
                    0xE1 => self.set_hl(value),
                    0xF1 => self.set_af(value),
                    _ => unreachable!(),
                }
            },
            
            // JP (conditional)
            0xC2 | 0xCA | 0xD2 | 0xDA => {
                let addr = self.read_word(self.pc);
                self.pc = self.pc.wrapping_add(2);
                
                let condition = match opcode {
                    0xC2 => !self.get_flag(FLAG_Z), // JP NZ
                    0xCA => self.get_flag(FLAG_Z),  // JP Z
                    0xD2 => !self.get_flag(FLAG_C), // JP NC
                    0xDA => self.get_flag(FLAG_C),  // JP C
                    _ => unreachable!(),
                };
                
                if condition {
                    self.pc = addr;
                }
            },
            
            // JP a16
            0xC3 => {
                self.pc = self.read_word(self.pc);
            },
            
            // CALL (conditional)
            0xC4 | 0xCC | 0xD4 | 0xDC => {
                let addr = self.read_word(self.pc);
                self.pc = self.pc.wrapping_add(2);
                
                let condition = match opcode {
                    0xC4 => !self.get_flag(FLAG_Z), // CALL NZ
                    0xCC => self.get_flag(FLAG_Z),  // CALL Z
                    0xD4 => !self.get_flag(FLAG_C), // CALL NC
                    0xDC => self.get_flag(FLAG_C),  // CALL C
                    _ => unreachable!(),
                };
                
                if condition {
                    self.push(self.pc);
                    self.pc = addr;
                }
            },
            
            // PUSH BC/DE/HL/AF
            0xC5 | 0xD5 | 0xE5 | 0xF5 => {
                let value = match opcode {
                    0xC5 => self.get_bc(),
                    0xD5 => self.get_de(),
                    0xE5 => self.get_hl(),
                    0xF5 => self.get_af(),
                    _ => unreachable!(),
                };
                self.push(value);
            },
            
            // ADD A, d8
            0xC6 => {
                let value = self.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
                
                let a = self.registers[REG_A];
                let result = a.wrapping_add(value);
                
                self.set_flag(FLAG_Z, result == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, (a & 0x0F) + (value & 0x0F) > 0x0F);
                self.set_flag(FLAG_C, (a as u16) + (value as u16) > 0xFF);
                
                self.registers[REG_A] = result;
            },
            
            // RET
            0xC9 => {
                self.pc = self.pop();
            },
            
            // CALL a16
            0xCD => {
                let addr = self.read_word(self.pc);
                self.pc = self.pc.wrapping_add(2);
                self.push(self.pc);
                self.pc = addr;
            },
            
            // LDH (a8), A
            0xE0 => {
                let offset = self.read_byte(self.pc) as u16;
                self.pc = self.pc.wrapping_add(1);
                self.write_byte(0xFF00 + offset, self.registers[REG_A]);
            },
            
            // LD (C), A
            0xE2 => {
                let addr = 0xFF00 + (self.registers[REG_C] as u16);
                self.write_byte(addr, self.registers[REG_A]);
            },
            
            // LDH A, (a8)
            0xF0 => {
                let offset = self.read_byte(self.pc) as u16;
                self.pc = self.pc.wrapping_add(1);
                self.registers[REG_A] = self.read_byte(0xFF00 + offset);
            },
            
            // DI
            0xF3 => {
                self.ime = false;
            },
            
            // CP d8
            0xFE => {
                let value = self.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
                
                let a = self.registers[REG_A];
                let result = a.wrapping_sub(value);
                
                self.set_flag(FLAG_Z, result == 0);
                self.set_flag(FLAG_N, true);
                self.set_flag(FLAG_H, (a & 0x0F) < (value & 0x0F));
                self.set_flag(FLAG_C, a < value);
            },
            
            // EI
            0xFB => {
                self.ime = true;
            },
            
            _ => {
                // For unimplemented instructions, just skip
                log::debug!("Unimplemented opcode: 0x{:02X} at PC=0x{:04X}", opcode, self.pc.wrapping_sub(1));
            }
        }

        self.cycles += 4; // Simplified cycle count
        Ok(())
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
        
        self.reset();
        
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
        self.rom_loaded = true;
        
        log::info!("Loaded Game Boy ROM: {} bytes", rom_data.len());
        Ok(())
    }

    fn reset(&mut self) {
        // Initialize registers to power-up state
        self.registers[REG_A] = 0x01;
        self.registers[REG_F] = 0xB0;
        self.registers[REG_B] = 0x00;
        self.registers[REG_C] = 0x13;
        self.registers[REG_D] = 0x00;
        self.registers[REG_E] = 0xD8;
        self.registers[REG_H] = 0x01;
        self.registers[REG_L] = 0x4D;
        
        self.sp = 0xFFFE;
        self.pc = 0x0100;
        self.ime = true;
        self.cycles = 0;
        
        // Clear VRAM and framebuffer
        self.vram = [0; 0x2000];
        self.framebuffer = [0xFF9BBC0F; WIDTH * HEIGHT];
        
        // Don't clear ROM area of memory
    }

    fn step(&mut self) -> Result<(), EmuError> {
        self.execute_instruction()?;
        
        // Update display every ~70224 cycles (one frame at 60Hz)
        if self.cycles % 70224 < 4 {
            self.render_background();
        }
        
        Ok(())
    }

    fn update_timers(&mut self, _delta: Duration) {
        // Timers would be updated here
        // For POC, we'll skip detailed timer implementation
    }

    fn framebuffer(&self) -> &[u32] {
        &self.framebuffer
    }

    fn resolution(&self) -> (usize, usize) {
        (WIDTH, HEIGHT)
    }

    fn set_input_state(&mut self, inputs: &[bool]) {
        if inputs.len() >= 8 {
            self.input_keys.copy_from_slice(&inputs[0..8]);
        }
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
        // Return a subset of memory for display purposes
        let mut memory_snapshot = vec![0u8; 0x1000]; // First 4KB for display
        memory_snapshot.copy_from_slice(&self.memory[0..0x1000]);
        
        GameBoyMetadata {
            registers: self.registers,
            sp: self.sp,
            pc: self.pc,
            memory: memory_snapshot,
            current_opcode: self.current_opcode,
            ime: self.ime,
        }
    }
}
