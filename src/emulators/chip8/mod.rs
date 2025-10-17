mod configs;
mod rand;

use super::{EmuError, Emulator};
use configs::{FONTSET_START_ADDRESS, HEIGHT, PROGRAM_START_ADDRESS, WIDTH};
use rand::Lcg;
use raplay::{source::Sine, Sink};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::Duration;

/// The CHIP-8 font set.
const FONT_SET: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

/// CHIP-8 specific metadata
#[derive(Debug, Clone)]
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

/// Implementation of the CHIP-8 emulator
pub struct Chip8Emulator {
    registers: [u8; 16],
    memory: [u8; 4096],
    index_register: u16,
    program_counter: u16,
    stack: [u16; 16],
    stack_pointer: u8,
    delay_timer: u8,
    sound_timer: u8,
    input_keys: [bool; 16],
    buffer: [bool; WIDTH * HEIGHT],
    framebuffer: [u32; WIDTH * HEIGHT],
    current_opcode: u16,
    lcg: Lcg,
    audio: Sink,
    is_mute: bool,
    is_key_pressed: bool,
}

impl Chip8Emulator {
    pub fn new() -> Self {
        let mut memory = [0; 4096];
        memory[FONTSET_START_ADDRESS as usize
            ..(FONTSET_START_ADDRESS + FONT_SET.len() as u16) as usize]
            .copy_from_slice(&FONT_SET);

        // Try to initialize audio, but don't fail if it's not available
        let mut sink = Sink::default();
        let src = Sine::new(440.0);
        let _ = sink.load(Box::new(src), false); // Ignore errors

        Self {
            registers: [0; 16],
            memory,
            index_register: 0,
            program_counter: PROGRAM_START_ADDRESS,
            stack: [0; 16],
            stack_pointer: 0,
            delay_timer: 0,
            sound_timer: 0,
            input_keys: [false; 16],
            current_opcode: 0,
            buffer: [false; WIDTH * HEIGHT],
            framebuffer: [0; WIDTH * HEIGHT],
            lcg: Lcg::new(75, 1, 31),
            audio: sink,
            is_mute: false,
            is_key_pressed: false,
        }
    }

    pub fn set_mute(&mut self, mute: bool) {
        self.is_mute = mute;
    }

    fn update_framebuffer(&mut self) {
        for (i, &pixel) in self.buffer.iter().enumerate() {
            self.framebuffer[i] = if pixel {
                0xFFFFFFFF // white ARGB
            } else {
                0xFF000000 // black ARGB
            };
        }
    }

    fn parse_opcode(&mut self, opcode: u16) -> Result<(), EmuError> {
        log::debug!("Parsing opcode: {:#06X}", opcode);
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let y = ((opcode & 0x00F0) >> 4) as usize;
        match opcode {
            0x00E0 => {
                // CLS, clear display
                self.buffer = [false; WIDTH * HEIGHT];
            }
            0x00EE => {
                // RET, return from subroutine
                let subtracted_stack_pointer = self.stack_pointer.wrapping_sub(1);

                if (0..16).contains(&subtracted_stack_pointer) {
                    self.stack_pointer = self.stack_pointer.wrapping_sub(1);
                    self.program_counter = self.stack[self.stack_pointer as usize];
                } else {
                    return Err(EmuError::InvalidStackAccess {
                        sp: subtracted_stack_pointer as u64,
                        pc: self.program_counter as u64,
                    });
                }
            }
            0x1000..=0x1FFF => {
                // JP addr, target address = opcode & 0x0FFF
                self.program_counter = opcode & 0x0FFF;
            }
            0x2000..=0x2FFF => {
                // CALL addr, target address = opcode & 0x0FFF
                if (0..16).contains(&self.stack_pointer) {
                    self.stack[self.stack_pointer as usize] = self.program_counter;
                    self.stack_pointer += 1;
                    self.program_counter = opcode & 0x0FFF;
                } else {
                    return Err(EmuError::InvalidStackAccess {
                        sp: self.stack_pointer as u64,
                        pc: self.program_counter as u64,
                    });
                }
            }
            0x3000..=0x3FFF => {
                // SE Vx, byte, skip next instruction if Vx == byte
                let byte = (opcode & 0x00FF) as u8;
                if self.registers[x] == byte {
                    self.program_counter = self.program_counter.wrapping_add(2);
                }
            }
            0x4000..=0x4FFF => {
                // SNE Vx, byte, skip next instruction if Vx != byte
                let byte = (opcode & 0x00FF) as u8;
                if self.registers[x] != byte {
                    self.program_counter = self.program_counter.wrapping_add(2);
                }
            }
            0x5000..=0x5FFF => {
                // SE Vx, Vy, skip next instruction if Vx == Vy

                // Check if last nibble is 0
                if opcode & 0x1 != 0 {
                    return Err(EmuError::InvalidOpcodeUsage {
                        opcode: opcode as u64,
                        pc: self.program_counter as u64,
                        hint: " (Set last nibble to 0)",
                    });
                } else if self.registers[x] == self.registers[y] {
                    self.program_counter = self.program_counter.wrapping_add(2);
                }
            }
            0x6000..=0x6FFF => {
                // LD Vx, byte, Vx = byte
                let byte = (opcode & 0x00FF) as u8;
                self.registers[x] = byte;
            }
            0x7000..=0x7FFF => {
                // ADD Vx, byte, Vx += byte
                let byte = (opcode & 0x00FF) as u8;
                self.registers[x] = self.registers[x].wrapping_add(byte);
            }
            0x8000..=0x8FFF => {
                match opcode & 0x000F {
                    0x0 => {
                        // LD Vx, Vy, Vx = Vy
                        self.registers[x] = self.registers[y];
                    }
                    0x1 => {
                        // OR Vx, Vy, Vx |= Vy
                        self.registers[x] |= self.registers[y];
                        self.registers[0xF] = 0;
                    }
                    0x2 => {
                        // AND Vx, Vy, Vx &= Vy
                        self.registers[x] &= self.registers[y];
                        self.registers[0xF] = 0;
                    }
                    0x3 => {
                        // XOR Vx, Vy, Vx ^= Vy
                        self.registers[x] ^= self.registers[y];
                        self.registers[0xF] = 0;
                    }
                    0x4 => {
                        // ADD Vx, Vy, Vx += Vy, VF = carry
                        let (result, overflow) =
                            self.registers[x].overflowing_add(self.registers[y]);
                        self.registers[x] = result;
                        self.registers[0xF] = overflow as u8;
                    }
                    0x5 => {
                        // SUB Vx, Vy, Vx -= Vy, VF = !borrow
                        let (result, overflow) =
                            self.registers[x].overflowing_sub(self.registers[y]);
                        self.registers[x] = result;
                        self.registers[0xF] = !overflow as u8;
                    }
                    0x6 => {
                        // SHR Vx {, Vy}, Vx = Vy >> 1, VF = carry
                        let carry = self.registers[y] & 0x1;
                        self.registers[x] = self.registers[y] >> 1;
                        self.registers[0xF] = carry;
                    }
                    0x7 => {
                        // SUBN Vx, Vy, Vx = Vy - Vx, VF = !borrow
                        let (result, overflow) =
                            self.registers[y].overflowing_sub(self.registers[x]);
                        self.registers[x] = result;
                        self.registers[0xF] = !overflow as u8;
                    }
                    0xE => {
                        // SHL Vx {, Vy}, Vx = Vy << 1, VF = carry
                        let carry = (self.registers[y] & 0x80) >> 7;
                        self.registers[x] = self.registers[y] << 1;
                        self.registers[0xF] = carry;
                    }
                    _ => {
                        return Err(EmuError::InvalidOpcodeUsage {
                            opcode: opcode as u64,
                            pc: self.program_counter as u64,
                            hint: " (Set last nibble to 0, 1, 2, 3, 4, 5, 6, 7 or E)",
                        });
                    }
                }
            }
            0x9000..=0x9FFF => {
                // SNE Vx, Vy, skip next instruction if Vx != Vy

                // Check if last nibble is 0
                if opcode & 0x1 != 0 {
                    return Err(EmuError::InvalidOpcodeUsage {
                        opcode: opcode as u64,
                        pc: self.program_counter as u64,
                        hint: " (Set last nibble to 0)",
                    });
                } else if self.registers[x] != self.registers[y] {
                    self.program_counter = self.program_counter.wrapping_add(2);
                }
            }
            0xA000..=0xAFFF => {
                // LD I, addr, I = addr
                self.index_register = opcode & 0x0FFF;
            }
            0xB000..=0xBFFF => {
                // JP V0, addr, PC = V0 + addr
                self.program_counter = self.registers[0] as u16 + (opcode & 0x0FFF);
            }
            0xC000..=0xCFFF => {
                // RND Vx, byte, Vx = rand() & byte
                let byte = (opcode & 0x00FF) as u8;
                self.registers[x] = self.lcg.next() & byte;
            }
            0xD000..=0xDFFF => {
                // DRW Vx, Vy, nibble, draw sprite at (Vx, Vy) with height nibble
                // VF = collision
                let height = (opcode & 0x000F) as usize;
                let x_pos = self.registers[x] as usize % WIDTH;
                let y_pos = self.registers[y] as usize % HEIGHT;
                self.registers[0xF] = 0;

                for row in 0..height {
                    let screen_y = (y_pos + row) % HEIGHT;

                    let index = self.index_register as usize + row;
                    let sprite_byte = if index < self.memory.len() {
                        self.memory[index]
                    } else {
                        return Err(EmuError::InvalidRegisterIndex {
                            index,
                            pc: self.program_counter as u64,
                        });
                    };

                    for col in 0..8 {
                        let screen_x = (x_pos + col) % WIDTH;

                        let sprite_pixel = sprite_byte & (0x80 >> col);
                        if sprite_pixel != 0 {
                            let screen_index = screen_y * WIDTH + screen_x;
                            let screen_pixel = &mut self.buffer[screen_index];
                            if *screen_pixel {
                                self.registers[0xF] = 1;
                            }
                            *screen_pixel = !*screen_pixel;
                        }
                    }
                }
            }
            0xE000..=0xEFFF => {
                match opcode & 0x00FF {
                    0x9E => {
                        // SKP Vx, skip next instruction if key Vx is pressed
                        if self.registers[x] > 15 {
                            log::warn!("Vx value not in 0..=15 range: {:#06X}", self.registers[x]);
                        }

                        let x_val = self.registers[x] % 16;

                        if self.input_keys[x_val as usize] {
                            self.program_counter = self.program_counter.wrapping_add(2);
                        }
                    }
                    0xA1 => {
                        // SKP Vx, skip next instruction if key Vx is NOT pressed
                        if self.registers[x] > 15 {
                            log::warn!("Vx value not in 0..=15 range: {:#06X}", self.registers[x]);
                        }

                        let x_val = self.registers[x] % 16;

                        if !self.input_keys[x_val as usize] {
                            self.program_counter = self.program_counter.wrapping_add(2);
                        }
                    }
                    _ => {
                        return Err(EmuError::InvalidOpcodeUsage {
                            opcode: opcode as u64,
                            pc: self.program_counter as u64,
                            hint: " (For Ex prefix, only 9E and A1 suffix are supported)",
                        });
                    }
                }
            }
            0xF000..=0xFFFF => {
                match opcode & 0x00FF {
                    0x07 => {
                        // LD Vx, DT, Set Vx = delay timer value.
                        self.registers[x] = self.delay_timer
                    }
                    0x0A => {
                        // Fx0A - LD Vx, K, Wait for a key press, store the value of the key in Vx.
                        for index in 0..16 {
                            if self.input_keys[index] {
                                self.registers[x] = index as u8;
                                self.is_key_pressed = true;
                                break;
                            }
                        }

                        // Stay on this instruction until a key is released
                        if self.is_key_pressed && self.input_keys.iter().all(|x| !x) {
                            self.is_key_pressed = false
                        } else {
                            self.program_counter = self.program_counter.wrapping_sub(2);
                        }
                    }
                    0x15 => {
                        // LD DT, Vx, Set delay timer = Vx.
                        self.delay_timer = self.registers[x];
                    }
                    0x18 => {
                        // LD ST, Vx, Set sound timer = Vx.
                        self.sound_timer = self.registers[x];
                    }
                    0x1E => {
                        // ADD I, Vx, Set I = I + Vx.
                        self.index_register =
                            self.index_register.wrapping_add(self.registers[x] as u16);
                    }
                    0x29 => {
                        // LD F, Vx, Set I = location of sprite for digit Vx.
                        if self.registers[x] > 15 {
                            log::warn!("Vx value not in 0..=15 range: {:#06X}", self.registers[x]);
                        }

                        let x_val = self.registers[x] % 16;
                        let index = FONTSET_START_ADDRESS
                            .wrapping_add((x_val as u16).wrapping_mul(5));

                        if index > 4096 {
                            log::warn!("Index value crossing 4KiB boundary: {:#06X}", index);
                        }

                        self.index_register = index % 4096;
                    }
                    0x33 => {
                        // LD B, Vx, Store BCD representation of Vx in memory locations I, I+1, and I+2.
                        let mut value = self.registers[x];

                        self.memory[self.index_register as usize + 2] = value % 10; // Ones
                        value /= 10;

                        self.memory[self.index_register as usize + 1] = value % 10; // Tens
                        value /= 10;

                        self.memory[self.index_register as usize] = value % 10;
                        // Hundreds
                    }
                    0x55 => {
                        // LD [I], Vx, Store registers V0 through Vx in memory starting at location I.
                        for index in 0..=x {
                            self.memory[self.index_register as usize + index] =
                                self.registers[index];
                        }
                        self.index_register += 1 + x as u16;
                    }
                    0x65 => {
                        // LD Vx, [I], Read registers V0 through Vx from memory starting at location I.
                        for index in 0..=x {
                            self.registers[index] =
                                self.memory[self.index_register as usize + index];
                        }
                        self.index_register += 1 + x as u16;
                    }
                    _ => {
                        return Err(EmuError::InvalidOpcodeUsage {
                            opcode: opcode as u64,
                            pc: self.program_counter as u64,
                            hint: " (For Fx prefix, only 07, 0A, 15, 18, 1E, 29, 33, 55 and 65 suffix are supported)",
                        });
                    }
                }
            }
            _ => {
                return Err(EmuError::UnrecognizedOpcode {
                    opcode: opcode as u64,
                    pc: self.program_counter as u64,
                });
            }
        }

        Ok(())
    }
}

impl Emulator for Chip8Emulator {
    type Metadata = Chip8Metadata;

    fn system_name(&self) -> &'static str {
        "CHIP-8"
    }

    fn load_rom(&mut self, path: &Path) -> Result<(), EmuError> {
        self.reset();
        let mut file = File::open(path).map_err(|e| EmuError::RomIoError {
            rom: path.to_path_buf(),
            source: e,
        })?;

        let rom_space = &mut self.memory[PROGRAM_START_ADDRESS as usize..];
        let n = file.read(rom_space).map_err(|e| EmuError::RomIoError {
            rom: path.to_path_buf(),
            source: e,
        })?;

        if n > rom_space.len() {
            return Err(EmuError::InvalidRom {
                rom: path.to_path_buf(),
                message: "ROM file is too large",
            });
        }

        Ok(())
    }

    fn reset(&mut self) {
        self.program_counter = PROGRAM_START_ADDRESS;
        self.index_register = 0;
        self.stack_pointer = 0;
        self.delay_timer = 0;
        self.sound_timer = 0;
        self.registers = [0; 16];
        self.buffer = [false; WIDTH * HEIGHT];
        self.framebuffer = [0; WIDTH * HEIGHT];
    }

    fn step(&mut self) -> Result<(), EmuError> {
        self.current_opcode = (self.memory[self.program_counter as usize] as u16) << 8
            | self.memory[self.program_counter as usize + 1] as u16;
        self.program_counter = self.program_counter.wrapping_add(2);
        self.parse_opcode(self.current_opcode)?;
        self.update_framebuffer();
        Ok(())
    }

    fn update_timers(&mut self, _delta: Duration) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        if self.sound_timer > 0 {
            if !self.is_mute {
                let _ = self.audio.play(true); // Ignore errors
            }
            self.sound_timer -= 1;
        } else if !self.is_mute {
            let _ = self.audio.pause(); // Ignore errors
        }
    }

    fn framebuffer(&self) -> &[u32] {
        &self.framebuffer
    }

    fn resolution(&self) -> (usize, usize) {
        (WIDTH, HEIGHT)
    }

    fn set_input_state(&mut self, inputs: &[bool]) {
        self.input_keys.copy_from_slice(&inputs[0..16.min(inputs.len())]);
    }

    fn keymap(&self) -> Vec<(usize, String)> {
        vec![
            (0x0, "X".to_string()),
            (0x1, "1".to_string()),
            (0x2, "2".to_string()),
            (0x3, "3".to_string()),
            (0x4, "Q".to_string()),
            (0x5, "W".to_string()),
            (0x6, "E".to_string()),
            (0x7, "A".to_string()),
            (0x8, "S".to_string()),
            (0x9, "D".to_string()),
            (0xA, "Z".to_string()),
            (0xB, "C".to_string()),
            (0xC, "4".to_string()),
            (0xD, "R".to_string()),
            (0xE, "F".to_string()),
            (0xF, "V".to_string()),
        ]
    }

    fn metadata(&self) -> Self::Metadata {
        Chip8Metadata {
            registers: self.registers,
            index_register: self.index_register,
            program_counter: self.program_counter,
            stack: self.stack,
            stack_pointer: self.stack_pointer,
            delay_timer: self.delay_timer,
            sound_timer: self.sound_timer,
            current_opcode: self.current_opcode,
            memory: self.memory,
        }
    }
}
