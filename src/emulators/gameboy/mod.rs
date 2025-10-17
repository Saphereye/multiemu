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
    halted: bool,             // CPU halted state
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
            halted: false,
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
                // Calculate VRAM byte index based on screen position
                let byte_index = (y * WIDTH + x) / 8;
                let bit_index = 7 - ((y * WIDTH + x) % 8);
                
                if byte_index < self.vram.len() {
                    // Check if the bit is set in VRAM
                    let pixel_on = (self.vram[byte_index] & (1 << bit_index)) != 0;
                    let color = if pixel_on {
                        0xFF0F380F // Dark green
                    } else {
                        0xFF9BBC0F // Light green
                    };
                    self.framebuffer[y * WIDTH + x] = color;
                } else {
                    // Beyond VRAM, show light green
                    self.framebuffer[y * WIDTH + x] = 0xFF9BBC0F;
                }
            }
        }
    }

    // Helper methods for common ALU operations
    fn inc_8bit(&mut self, value: u8) -> u8 {
        let result = value.wrapping_add(1);
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, (value & 0x0F) == 0x0F);
        result
    }

    fn dec_8bit(&mut self, value: u8) -> u8 {
        let result = value.wrapping_sub(1);
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N, true);
        self.set_flag(FLAG_H, (value & 0x0F) == 0);
        result
    }

    fn add_hl(&mut self, value: u16) {
        let hl = self.get_hl();
        let result = hl.wrapping_add(value);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, (hl & 0x0FFF) + (value & 0x0FFF) > 0x0FFF);
        self.set_flag(FLAG_C, hl > 0xFFFF - value);
        self.set_hl(result);
    }

    fn adc(&mut self, value: u8) {
        let a = self.registers[REG_A];
        let carry = if self.get_flag(FLAG_C) { 1 } else { 0 };
        let result = a.wrapping_add(value).wrapping_add(carry);
        
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, (a & 0x0F) + (value & 0x0F) + carry > 0x0F);
        self.set_flag(FLAG_C, (a as u16) + (value as u16) + (carry as u16) > 0xFF);
        
        self.registers[REG_A] = result;
    }

    fn sbc(&mut self, value: u8) {
        let a = self.registers[REG_A];
        let carry = if self.get_flag(FLAG_C) { 1 } else { 0 };
        let result = a.wrapping_sub(value).wrapping_sub(carry);
        
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N, true);
        self.set_flag(FLAG_H, (a & 0x0F) < (value & 0x0F) + carry);
        self.set_flag(FLAG_C, (a as u16) < (value as u16) + (carry as u16));
        
        self.registers[REG_A] = result;
    }

    fn and(&mut self, value: u8) {
        self.registers[REG_A] &= value;
        self.set_flag(FLAG_Z, self.registers[REG_A] == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, true);
        self.set_flag(FLAG_C, false);
    }

    fn or(&mut self, value: u8) {
        self.registers[REG_A] |= value;
        self.set_flag(FLAG_Z, self.registers[REG_A] == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, false);
    }

    fn rlca(&mut self) {
        let a = self.registers[REG_A];
        let carry = (a & 0x80) >> 7;
        self.registers[REG_A] = (a << 1) | carry;
        self.set_flag(FLAG_Z, false);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, carry != 0);
    }

    fn rrca(&mut self) {
        let a = self.registers[REG_A];
        let carry = a & 0x01;
        self.registers[REG_A] = (a >> 1) | (carry << 7);
        self.set_flag(FLAG_Z, false);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, carry != 0);
    }

    fn rla(&mut self) {
        let a = self.registers[REG_A];
        let old_carry = if self.get_flag(FLAG_C) { 1 } else { 0 };
        let new_carry = (a & 0x80) >> 7;
        self.registers[REG_A] = (a << 1) | old_carry;
        self.set_flag(FLAG_Z, false);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, new_carry != 0);
    }

    fn rra(&mut self) {
        let a = self.registers[REG_A];
        let old_carry = if self.get_flag(FLAG_C) { 1 } else { 0 };
        let new_carry = a & 0x01;
        self.registers[REG_A] = (a >> 1) | (old_carry << 7);
        self.set_flag(FLAG_Z, false);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, new_carry != 0);
    }

    fn daa(&mut self) {
        let mut a = self.registers[REG_A];
        let mut adjust = 0;
        
        if self.get_flag(FLAG_H) || (!self.get_flag(FLAG_N) && (a & 0x0F) > 9) {
            adjust |= 0x06;
        }
        
        if self.get_flag(FLAG_C) || (!self.get_flag(FLAG_N) && a > 0x99) {
            adjust |= 0x60;
            self.set_flag(FLAG_C, true);
        }
        
        a = if self.get_flag(FLAG_N) {
            a.wrapping_sub(adjust)
        } else {
            a.wrapping_add(adjust)
        };
        
        self.registers[REG_A] = a;
        self.set_flag(FLAG_Z, a == 0);
        self.set_flag(FLAG_H, false);
    }

    fn cpl(&mut self) {
        self.registers[REG_A] = !self.registers[REG_A];
        self.set_flag(FLAG_N, true);
        self.set_flag(FLAG_H, true);
    }

    fn scf(&mut self) {
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, true);
    }

    fn ccf(&mut self) {
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, !self.get_flag(FLAG_C));
    }

    // CB prefix instructions
    fn execute_cb_instruction(&mut self) -> u32 {
        let cb_opcode = self.read_byte(self.pc);
        self.pc = self.pc.wrapping_add(1);
        
        let reg_index = (cb_opcode & 0x07) as usize;
        let bit = ((cb_opcode >> 3) & 0x07) as u8;
        
        let (value, cycles) = if reg_index == 6 {
            // (HL) operations take longer
            (self.read_byte(self.get_hl()), 16)
        } else {
            (self.registers[reg_index], 8)
        };
        
        let result = match cb_opcode {
            // RLC r
            0x00..=0x07 => {
                let carry = (value & 0x80) >> 7;
                let result = (value << 1) | carry;
                self.set_flag(FLAG_Z, result == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, carry != 0);
                result
            },
            // RRC r
            0x08..=0x0F => {
                let carry = value & 0x01;
                let result = (value >> 1) | (carry << 7);
                self.set_flag(FLAG_Z, result == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, carry != 0);
                result
            },
            // RL r
            0x10..=0x17 => {
                let old_carry = if self.get_flag(FLAG_C) { 1 } else { 0 };
                let new_carry = (value & 0x80) >> 7;
                let result = (value << 1) | old_carry;
                self.set_flag(FLAG_Z, result == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, new_carry != 0);
                result
            },
            // RR r
            0x18..=0x1F => {
                let old_carry = if self.get_flag(FLAG_C) { 1 } else { 0 };
                let new_carry = value & 0x01;
                let result = (value >> 1) | (old_carry << 7);
                self.set_flag(FLAG_Z, result == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, new_carry != 0);
                result
            },
            // SLA r
            0x20..=0x27 => {
                let carry = (value & 0x80) >> 7;
                let result = value << 1;
                self.set_flag(FLAG_Z, result == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, carry != 0);
                result
            },
            // SRA r
            0x28..=0x2F => {
                let carry = value & 0x01;
                let result = (value >> 1) | (value & 0x80);
                self.set_flag(FLAG_Z, result == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, carry != 0);
                result
            },
            // SWAP r
            0x30..=0x37 => {
                let result = (value >> 4) | (value << 4);
                self.set_flag(FLAG_Z, result == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, false);
                result
            },
            // SRL r
            0x38..=0x3F => {
                let carry = value & 0x01;
                let result = value >> 1;
                self.set_flag(FLAG_Z, result == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, carry != 0);
                result
            },
            // BIT b, r
            0x40..=0x7F => {
                let bit_val = (value >> bit) & 0x01;
                self.set_flag(FLAG_Z, bit_val == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, true);
                return if reg_index == 6 { 12 } else { 8 };  // BIT doesn't write back
            },
            // RES b, r
            0x80..=0xBF => {
                value & !(1 << bit)
            },
            // SET b, r
            0xC0..=0xFF => {
                value | (1 << bit)
            },
        };
        
        // Write back result
        if reg_index == 6 {
            self.write_byte(self.get_hl(), result);
        } else {
            self.registers[reg_index] = result;
        }
        
        cycles
    }

    // Execute one instruction and return cycle count
    fn execute_instruction(&mut self) -> Result<u32, EmuError> {
        if !self.rom_loaded {
            return Ok(4);
        }

        // If halted, don't execute instructions until an interrupt occurs
        // Just return immediately without spinning to avoid freezing
        if self.halted {
            return Ok(4);
        }

        self.current_opcode = self.read_byte(self.pc);
        let opcode = self.current_opcode;
        self.pc = self.pc.wrapping_add(1);

        // Execute instruction and return cycles taken
        let mut cycles: u32 = 4; // Default
        
        match opcode {
            // NOP
            0x00 => { cycles = 4; },
            
            // LD BC, d16
            0x01 => {
                let value = self.read_word(self.pc);
                self.pc = self.pc.wrapping_add(2);
                self.set_bc(value);
                cycles = 12;
            },
            
            // LD (BC), A
            0x02 => {
                let addr = self.get_bc();
                self.write_byte(addr, self.registers[REG_A]);
                cycles = 8;
            },
            
            // INC BC
            0x03 => {
                let value = self.get_bc().wrapping_add(1);
                self.set_bc(value);
                cycles = 8;
            },
            
            // INC B
            0x04 => {
                self.registers[REG_B] = self.inc_8bit(self.registers[REG_B]);
                cycles = 4;
            },
            
            // DEC B
            0x05 => {
                self.registers[REG_B] = self.dec_8bit(self.registers[REG_B]);
                cycles = 4;
            },
            
            // LD B, d8
            0x06 => {
                self.registers[REG_B] = self.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
                cycles = 8;
            },
            
            // RLCA
            0x07 => {
                self.rlca();
                cycles = 4;
            },
            
            // LD (a16), SP
            0x08 => {
                let addr = self.read_word(self.pc);
                self.pc = self.pc.wrapping_add(2);
                self.write_word(addr, self.sp);
                cycles = 20;
            },
            
            // ADD HL, BC
            0x09 => {
                self.add_hl(self.get_bc());
                cycles = 8;
            },
            
            // LD A, (BC)
            0x0A => {
                let addr = self.get_bc();
                self.registers[REG_A] = self.read_byte(addr);
                cycles = 8;
            },
            
            // DEC BC
            0x0B => {
                let value = self.get_bc().wrapping_sub(1);
                self.set_bc(value);
                cycles = 8;
            },
            
            // INC C
            0x0C => {
                self.registers[REG_C] = self.inc_8bit(self.registers[REG_C]);
                cycles = 4;
            },
            
            // DEC C
            0x0D => {
                self.registers[REG_C] = self.dec_8bit(self.registers[REG_C]);
                cycles = 4;
            },
            
            // LD C, d8
            0x0E => {
                self.registers[REG_C] = self.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
                cycles = 8;
            },
            
            // RRCA
            0x0F => {
                self.rrca();
                cycles = 4;
            },
            
            //  STOP
            // STOP
            // STOP
            0x10 => {
                // STOP halts CPU and LCD until button press
                self.halted = true;
                cycles = 4;
            },
            
            // LD DE, d16
            0x11 => {
                let value = self.read_word(self.pc);
                self.pc = self.pc.wrapping_add(2);
                self.set_de(value);
                cycles = 12;
            },
            
            // LD (DE), A
            0x12 => {
                let addr = self.get_de();
                self.write_byte(addr, self.registers[REG_A]);
                cycles = 8;
            },
            
            // INC DE
            0x13 => {
                let value = self.get_de().wrapping_add(1);
                self.set_de(value);
                cycles = 8;
            },
            
            // INC D
            0x14 => {
                self.registers[REG_D] = self.inc_8bit(self.registers[REG_D]);
                cycles = 4;
            },
            
            // DEC D
            0x15 => {
                self.registers[REG_D] = self.dec_8bit(self.registers[REG_D]);
                cycles = 4;
            },
            
            // LD D, d8
            0x16 => {
                self.registers[REG_D] = self.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
                cycles = 8;
            },
            
            // RLA
            0x17 => {
                self.rla();
                cycles = 4;
            },
            
            // JR r8
            0x18 => {
                let offset = self.read_byte(self.pc) as i8;
                self.pc = self.pc.wrapping_add(1);
                self.pc = ((self.pc as i32) + (offset as i32)) as u16;
                cycles = 12;
            },
            
            // ADD HL, DE
            0x19 => {
                self.add_hl(self.get_de());
                cycles = 8;
            },
            
            // LD A, (DE)
            0x1A => {
                let addr = self.get_de();
                self.registers[REG_A] = self.read_byte(addr);
                cycles = 8;
            },
            
            // DEC DE
            0x1B => {
                let value = self.get_de().wrapping_sub(1);
                self.set_de(value);
                cycles = 8;
            },
            
            // INC E
            0x1C => {
                self.registers[REG_E] = self.inc_8bit(self.registers[REG_E]);
                cycles = 4;
            },
            
            // DEC E
            0x1D => {
                self.registers[REG_E] = self.dec_8bit(self.registers[REG_E]);
                cycles = 4;
            },
            
            // LD E, d8
            0x1E => {
                self.registers[REG_E] = self.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
                cycles = 8;
            },
            
            // RRA
            0x1F => {
                self.rra();
                cycles = 4;
            },
            
            // JR NZ, r8
            0x20 => {
                let offset = self.read_byte(self.pc) as i8;
                self.pc = self.pc.wrapping_add(1);
                if !self.get_flag(FLAG_Z) {
                    self.pc = ((self.pc as i32) + (offset as i32)) as u16;
                    cycles = 12;
                } else {
                    cycles = 8;
                }
            },
            
            // LD HL, d16
            0x21 => {
                let value = self.read_word(self.pc);
                self.pc = self.pc.wrapping_add(2);
                self.set_hl(value);
                cycles = 12;
            },
            
            // LD (HL+), A
            0x22 => {
                let addr = self.get_hl();
                self.write_byte(addr, self.registers[REG_A]);
                self.set_hl(addr.wrapping_add(1));
                cycles = 8;
            },
            
            // INC HL
            0x23 => {
                let value = self.get_hl().wrapping_add(1);
                self.set_hl(value);
                cycles = 8;
            },
            
            // INC H
            0x24 => {
                self.registers[REG_H] = self.inc_8bit(self.registers[REG_H]);
                cycles = 4;
            },
            
            // DEC H
            0x25 => {
                self.registers[REG_H] = self.dec_8bit(self.registers[REG_H]);
                cycles = 4;
            },
            
            // LD H, d8
            0x26 => {
                self.registers[REG_H] = self.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
                cycles = 8;
            },
            
            // DAA
            0x27 => {
                self.daa();
                cycles = 4;
            },
            
            // JR Z, r8
            0x28 => {
                let offset = self.read_byte(self.pc) as i8;
                self.pc = self.pc.wrapping_add(1);
                if self.get_flag(FLAG_Z) {
                    self.pc = ((self.pc as i32) + (offset as i32)) as u16;
                    cycles = 12;
                } else {
                    cycles = 8;
                }
            },
            
            // ADD HL, HL
            0x29 => {
                let hl = self.get_hl();
                self.add_hl(hl);
                cycles = 8;
            },
            
            // LD A, (HL+)
            0x2A => {
                let addr = self.get_hl();
                self.registers[REG_A] = self.read_byte(addr);
                self.set_hl(addr.wrapping_add(1));
                cycles = 8;
            },
            
            // DEC HL
            0x2B => {
                let value = self.get_hl().wrapping_sub(1);
                self.set_hl(value);
                cycles = 8;
            },
            
            // INC L
            0x2C => {
                self.registers[REG_L] = self.inc_8bit(self.registers[REG_L]);
                cycles = 4;
            },
            
            // DEC L
            0x2D => {
                self.registers[REG_L] = self.dec_8bit(self.registers[REG_L]);
                cycles = 4;
            },
            
            // LD L, d8
            0x2E => {
                self.registers[REG_L] = self.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
                cycles = 8;
            },
            
            // CPL
            0x2F => {
                self.cpl();
                cycles = 4;
            },
            
            // JR NC, r8
            0x30 => {
                let offset = self.read_byte(self.pc) as i8;
                self.pc = self.pc.wrapping_add(1);
                if !self.get_flag(FLAG_C) {
                    self.pc = ((self.pc as i32) + (offset as i32)) as u16;
                    cycles = 12;
                } else {
                    cycles = 8;
                }
            },
            
            // LD SP, d16
            0x31 => {
                self.sp = self.read_word(self.pc);
                self.pc = self.pc.wrapping_add(2);
                cycles = 12;
            },
            
            // LD (HL-), A
            0x32 => {
                let addr = self.get_hl();
                self.write_byte(addr, self.registers[REG_A]);
                self.set_hl(addr.wrapping_sub(1));
                cycles = 8;
            },
            
            // INC SP
            0x33 => {
                self.sp = self.sp.wrapping_add(1);
                cycles = 8;
            },
            
            // INC (HL)
            0x34 => {
                let addr = self.get_hl();
                let value = self.read_byte(addr);
                let result = self.inc_8bit(value);
                self.write_byte(addr, result);
                cycles = 12;
            },
            
            // DEC (HL)
            0x35 => {
                let addr = self.get_hl();
                let value = self.read_byte(addr);
                let result = self.dec_8bit(value);
                self.write_byte(addr, result);
                cycles = 12;
            },
            
            // LD (HL), d8
            0x36 => {
                let value = self.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
                let addr = self.get_hl();
                self.write_byte(addr, value);
                cycles = 12;
            },
            
            // SCF
            0x37 => {
                self.scf();
                cycles = 4;
            },
            
            // JR C, r8
            0x38 => {
                let offset = self.read_byte(self.pc) as i8;
                self.pc = self.pc.wrapping_add(1);
                if self.get_flag(FLAG_C) {
                    self.pc = ((self.pc as i32) + (offset as i32)) as u16;
                    cycles = 12;
                } else {
                    cycles = 8;
                }
            },
            
            // ADD HL, SP
            0x39 => {
                self.add_hl(self.sp);
                cycles = 8;
            },
            
            // LD A, (HL-)
            0x3A => {
                let addr = self.get_hl();
                self.registers[REG_A] = self.read_byte(addr);
                self.set_hl(addr.wrapping_sub(1));
                cycles = 8;
            },
            
            // DEC SP
            0x3B => {
                self.sp = self.sp.wrapping_sub(1);
                cycles = 8;
            },
            
            // INC A
            0x3C => {
                self.registers[REG_A] = self.inc_8bit(self.registers[REG_A]);
                cycles = 4;
            },
            
            // DEC A
            0x3D => {
                self.registers[REG_A] = self.dec_8bit(self.registers[REG_A]);
                cycles = 4;
            },
            
            // LD A, d8
            0x3E => {
                self.registers[REG_A] = self.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
                cycles = 8;
            },
            
            // CCF
            0x3F => {
                self.ccf();
                cycles = 4;
            },
            
            // LD B, B through LD A, A (most LD r, r instructions)
            0x40..=0x7F => {
                let to = ((opcode >> 3) & 0x07) as usize;
                let from = (opcode & 0x07) as usize;
                
                // HALT is 0x76
                cycles = if opcode == 0x76 {
                    // HALT - enter low power mode until interrupt
                    self.halted = true;
                    4
                } else if from == 6 {
                    // LD r, (HL)
                    let addr = self.get_hl();
                    let value = self.read_byte(addr);
                    self.registers[to] = value;
                    8
                } else if to == 6 {
                    // LD (HL), r
                    let addr = self.get_hl();
                    self.write_byte(addr, self.registers[from]);
                    8
                } else {
                    // LD r, r
                    self.registers[to] = self.registers[from];
                    4
                };
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
                cycles = if (opcode & 0x07) == 6 { 8 } else { 4 };
            },
            
            // ADC A, r
            0x88..=0x8F => {
                let reg = (opcode & 0x07) as usize;
                let value = if reg == 6 {
                    let addr = self.get_hl();
                    self.read_byte(addr)
                } else {
                    self.registers[reg]
                };
                self.adc(value);
                cycles = if (opcode & 0x07) == 6 { 8 } else { 4 };
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
                cycles = if (opcode & 0x07) == 6 { 8 } else { 4 };
            },
            
            // SBC A, r
            0x98..=0x9F => {
                let reg = (opcode & 0x07) as usize;
                let value = if reg == 6 {
                    let addr = self.get_hl();
                    self.read_byte(addr)
                } else {
                    self.registers[reg]
                };
                self.sbc(value);
                cycles = if (opcode & 0x07) == 6 { 8 } else { 4 };
            },
            
            // AND A, r
            0xA0..=0xA7 => {
                let reg = (opcode & 0x07) as usize;
                let value = if reg == 6 {
                    let addr = self.get_hl();
                    self.read_byte(addr)
                } else {
                    self.registers[reg]
                };
                self.and(value);
                cycles = if (opcode & 0x07) == 6 { 8 } else { 4 };
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
                cycles = if (opcode & 0x07) == 6 { 8 } else { 4 };
            },
            
            // OR A, r
            0xB0..=0xB7 => {
                let reg = (opcode & 0x07) as usize;
                let value = if reg == 6 {
                    let addr = self.get_hl();
                    self.read_byte(addr)
                } else {
                    self.registers[reg]
                };
                self.or(value);
                cycles = if (opcode & 0x07) == 6 { 8 } else { 4 };
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
                cycles = if (opcode & 0x07) == 6 { 8 } else { 4 };            },
            
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
                    cycles = 20;
                } else {
                    cycles = 8;
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
                cycles = 12;
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
                    cycles = 16;
                } else {
                    cycles = 12;
                }
            },
            
            // JP a16
            0xC3 => {
                self.pc = self.read_word(self.pc);
                cycles = 16;
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
                    cycles = 24;
                } else {
                    cycles = 12;
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
                cycles = 16;
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
                cycles = 8;
            },
            
            // RST 00H
            0xC7 => {
                self.push(self.pc);
                self.pc = 0x0000;
                cycles = 16;
            },
            
            // CB prefix
            0xCB => {
                cycles = self.execute_cb_instruction();
            },
            
            // RET
            0xC9 => {
                self.pc = self.pop();
                cycles = 16;
            },
            
            // CALL a16
            0xCD => {
                let addr = self.read_word(self.pc);
                self.pc = self.pc.wrapping_add(2);
                self.push(self.pc);
                self.pc = addr;
                cycles = 24;
            },
            
            // ADC A, d8
            0xCE => {
                let value = self.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
                self.adc(value);
                cycles = 8;
            },
            
            // RST 08H
            0xCF => {
                self.push(self.pc);
                self.pc = 0x0008;
                cycles = 16;
            },
            
            // RST 10H
            0xD7 => {
                self.push(self.pc);
                self.pc = 0x0010;
                cycles = 16;
            },
            
            // RETI
            0xD9 => {
                self.pc = self.pop();
                self.ime = true;
                cycles = 16;
            },
            
            // SUB d8
            0xD6 => {
                let value = self.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
                
                let a = self.registers[REG_A];
                let result = a.wrapping_sub(value);
                
                self.set_flag(FLAG_Z, result == 0);
                self.set_flag(FLAG_N, true);
                self.set_flag(FLAG_H, (a & 0x0F) < (value & 0x0F));
                self.set_flag(FLAG_C, a < value);
                
                self.registers[REG_A] = result;
                cycles = 8;
            },
            
            // SBC A, d8
            0xDE => {
                let value = self.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
                self.sbc(value);
                cycles = 8;
            },
            
            // RST 18H
            0xDF => {
                self.push(self.pc);
                self.pc = 0x0018;
                cycles = 16;
            },
            
            // LDH (a8), A
            0xE0 => {
                let offset = self.read_byte(self.pc) as u16;
                self.pc = self.pc.wrapping_add(1);
                self.write_byte(0xFF00 + offset, self.registers[REG_A]);
                cycles = 12;
            },
            
            // LD (C), A
            0xE2 => {
                let addr = 0xFF00 + (self.registers[REG_C] as u16);
                self.write_byte(addr, self.registers[REG_A]);
                cycles = 8;
            },
            
            // AND d8
            0xE6 => {
                let value = self.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
                self.and(value);
                cycles = 8;
            },
            
            // RST 20H
            0xE7 => {
                self.push(self.pc);
                self.pc = 0x0020;
                cycles = 16;
            },
            
            // ADD SP, r8
            0xE8 => {
                let offset = self.read_byte(self.pc) as i8 as i16 as u16;
                self.pc = self.pc.wrapping_add(1);
                let sp = self.sp;
                let result = sp.wrapping_add(offset);
                
                self.set_flag(FLAG_Z, false);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, (sp & 0x0F) + (offset & 0x0F) > 0x0F);
                self.set_flag(FLAG_C, (sp & 0xFF) + (offset & 0xFF) > 0xFF);
                
                self.sp = result;
                cycles = 16;
            },
            
            // JP (HL)
            0xE9 => {
                self.pc = self.get_hl();
                cycles = 4;
            },
            
            // LD (a16), A
            0xEA => {
                let addr = self.read_word(self.pc);
                self.pc = self.pc.wrapping_add(2);
                self.write_byte(addr, self.registers[REG_A]);
                cycles = 16;
            },
            
            // XOR d8
            0xEE => {
                let value = self.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
                self.registers[REG_A] ^= value;
                self.set_flag(FLAG_Z, self.registers[REG_A] == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, false);
                cycles = 8;
            },
            
            // RST 28H
            0xEF => {
                self.push(self.pc);
                self.pc = 0x0028;
                cycles = 16;
            },
            
            // LDH A, (a8)
            0xF0 => {
                let offset = self.read_byte(self.pc) as u16;
                self.pc = self.pc.wrapping_add(1);
                self.registers[REG_A] = self.read_byte(0xFF00 + offset);
                cycles = 12;
            },
            
            // LD A, (C)
            0xF2 => {
                let addr = 0xFF00 + (self.registers[REG_C] as u16);
                self.registers[REG_A] = self.read_byte(addr);
                cycles = 8;
            },
            
            // DI
            0xF3 => {
                self.ime = false;
                cycles = 4;
            },
            
            // OR d8
            0xF6 => {
                let value = self.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
                self.or(value);
                cycles = 8;
            },
            
            // RST 30H
            0xF7 => {
                self.push(self.pc);
                self.pc = 0x0030;
                cycles = 16;
            },
            
            // LD HL, SP+r8
            0xF8 => {
                let offset = self.read_byte(self.pc) as i8 as i16 as u16;
                self.pc = self.pc.wrapping_add(1);
                let sp = self.sp;
                let result = sp.wrapping_add(offset);
                
                self.set_flag(FLAG_Z, false);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, (sp & 0x0F) + (offset & 0x0F) > 0x0F);
                self.set_flag(FLAG_C, (sp & 0xFF) + (offset & 0xFF) > 0xFF);
                
                self.set_hl(result);
                cycles = 12;
            },
            
            // LD SP, HL
            0xF9 => {
                self.sp = self.get_hl();
                cycles = 8;
            },
            
            // LD A, (a16)
            0xFA => {
                let addr = self.read_word(self.pc);
                self.pc = self.pc.wrapping_add(2);
                self.registers[REG_A] = self.read_byte(addr);
                cycles = 16;
            },
            
            // EI
            0xFB => {
                self.ime = true;
                // Wake from HALT when interrupts are enabled
                self.halted = false;
                cycles = 4;
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
                cycles = 8;
            },
            
            // RST 38H
            0xFF => {
                self.push(self.pc);
                self.pc = 0x0038;
                cycles = 16;
            },
            
            _ => {
                // For unimplemented instructions, just skip
                log::warn!("Unimplemented opcode: 0x{:02X} at PC=0x{:04X}", opcode, self.pc.wrapping_sub(1));
                cycles = 4;
            }
        };
        
        self.cycles += cycles as u64;
        Ok(cycles)
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
        log::info!("PC initialized to: 0x{:04X}", self.pc);
        log::info!("First 16 bytes at PC:");
        for i in 0..16 {
            log::info!("  0x{:04X}: 0x{:02X}", self.pc as usize + i, self.memory[self.pc as usize + i]);
        }
        log::info!("Halted state: {}", self.halted);
        log::info!("IME: {}", self.ime);
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
        self.halted = false;
        
        // Clear VRAM and framebuffer
        self.vram = [0; 0x2000];
        self.framebuffer = [0xFF9BBC0F; WIDTH * HEIGHT];
        
        // Don't clear ROM area of memory
    }

    fn step(&mut self) -> Result<(), EmuError> {
        let cycles_before = self.cycles;
        self.execute_instruction()?;
        
        // Update display every ~70224 cycles (one frame at 60Hz)
        // Check if we crossed a frame boundary
        let frame_before = cycles_before / 70224;
        let frame_after = self.cycles / 70224;
        if frame_after > frame_before {
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
            // Wake from HALT/STOP on any button press
            if inputs.iter().any(|&pressed| pressed) {
                self.halted = false;
            }
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
