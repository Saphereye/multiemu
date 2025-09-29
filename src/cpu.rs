use crate::configs::{FONTSET_START_ADDRESS, HEIGHT, PROGRAM_START_ADDRESS, WIDTH};
use crate::rand::Lcg;
use raplay::{source::Sine, Sink};

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

/// Implementation of the Chip-8 CPU
pub struct Cpu {
    pub registers: [u8; 16],
    pub memory: [u8; 4096],
    pub index_register: u16,
    pub program_counter: u16,
    pub stack: [u16; 16], // 16 levels of stack
    pub stack_pointer: u8,
    pub delay_timer: u8,
    pub sound_timer: u8,
    pub input_keys: [bool; 16],         // true if nth key is pressed
    pub buffer: [bool; WIDTH * HEIGHT], // true if pixel is on
    pub current_opcode: u16,

    // These fields aren't cpu specific,
    // but I am using them as helper fields
    pub lcg: Lcg,
    pub audio: Sink,
    pub to_draw: bool,
    pub is_mute: bool,
    is_key_pressed: bool,
}

impl Cpu {
    /// Initialize the CPU with default values.
    ///
    /// The program counter is set to 0x200, the start address of the program.
    /// All registers are set to 0 and the monochrome display is cleared.
    pub fn new() -> Self {
        let mut memory = [0; 4096];
        memory[FONTSET_START_ADDRESS as usize
            ..(FONTSET_START_ADDRESS + FONT_SET.len() as u16) as usize]
            .copy_from_slice(&FONT_SET);

        let mut sink = Sink::default();
        let src = Sine::new(440.0);
        sink.load(Box::new(src), false).unwrap();
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
            buffer: [false; 64 * 32],
            // For the first 200 values of this LCG:
            // Arithmetic mean: 129.02 Expected value: 128.00
            // Monte Carlo PI Test: 3.120, where PI should be 3.142
            // Serial Coefficient: 0.090
            // Entropy: 7.366bits, where 8 bits is optimal
            lcg: Lcg::new(75, 1, 31),
            audio: sink,
            to_draw: false,
            is_mute: false,
            is_key_pressed: false,
        }
    }

    /// Parses the opcode and executes the instruction.
    ///
    /// Uses the CHIP-8 instruction set.
    fn parse_opcode(&mut self, opcode: u16) {
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let y = ((opcode & 0x00F0) >> 4) as usize;
        match opcode {
            0x00E0 => {
                // CLS, clear display
                self.buffer = [false; 64 * 32];
            }
            0x00EE => {
                // RET, return from subroutine
                self.stack_pointer -= 1;
                self.program_counter = self.stack[self.stack_pointer as usize];
            }
            0x1000..=0x1FFF => {
                // JP addr, target address = opcode & 0x0FFF
                self.program_counter = opcode & 0x0FFF;
            }
            0x2000..=0x2FFF => {
                // CALL addr, target address = opcode & 0x0FFF
                self.stack[self.stack_pointer as usize] = self.program_counter;
                self.stack_pointer += 1;
                self.program_counter = opcode & 0x0FFF;
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
                    eprintln!("Unrecognized opcode {:X}", opcode);
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
                    _ => eprintln!("Unrecognized opcode {:X}", opcode),
                }
            }
            0x9000..=0x9FFF => {
                // SNE Vx, Vy, skip next instruction if Vx != Vy

                // Check if last nibble is 0
                if opcode & 0x1 != 0 {
                    eprintln!("Unrecognized opcode {:X}", opcode);
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
                let x_pos = self.registers[x] % WIDTH as u8;
                let y_pos = self.registers[y] % HEIGHT as u8;
                self.registers[0xF] = 0;

                for row in 0..height {
                    if y_pos as usize + row >= HEIGHT {
                        break;
                    }
                    let sprite_byte = self.memory[self.index_register as usize + row];
                    for col in 0..8 {
                        if x_pos as usize + col >= WIDTH {
                            break;
                        }
                        let sprite_pixel = sprite_byte & (0x80 >> col);
                        let screen_index = (y_pos as usize + row) * WIDTH + (x_pos as usize + col);
                        let screen_pixel = &mut self.buffer[screen_index];
                        if sprite_pixel != 0 {
                            if *screen_pixel {
                                self.registers[0xF] = 1;
                            }
                            *screen_pixel = !*screen_pixel;
                        }
                    }
                }

                self.to_draw = true;
            }
            0xE000..=0xEFFF => {
                match opcode & 0x00FF {
                    0x9E => {
                        // SKP Vx, skip next instruction if key Vx is pressed
                        if self.input_keys[self.registers[x] as usize] {
                            self.program_counter = self.program_counter.wrapping_add(2);
                        }
                    }
                    0xA1 => {
                        // SKP Vx, skip next instruction if key Vx is NOT pressed
                        if !self.input_keys[self.registers[x] as usize] {
                            self.program_counter = self.program_counter.wrapping_add(2);
                        }
                    }
                    _ => eprintln!("Unrecognized opcode {:X}", opcode),
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
                        self.index_register = FONTSET_START_ADDRESS + self.registers[x] as u16 * 5;
                    }
                    0x33 => {
                        // LD B, Vx, Store BCD representation of Vx in memory locations I, I+1, and I+2.
                        // The interpreter takes the decimal value of Vx, and places the hundreds digit in memory at location in I, the tens digit at location I+1, and the ones digit at location I+2.
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
                    _ => eprintln!("Unrecognized opcode {:X}", opcode),
                }
            }
            _ => eprintln!("Unrecognized opcode {:X}", opcode),
        }
    }

    /// Simulates one execution cycle of the cpu.
    pub fn execute_instruction(&mut self) {
        self.current_opcode = (self.memory[self.program_counter as usize] as u16) << 8
            | self.memory[self.program_counter as usize + 1] as u16;
        self.program_counter = self.program_counter.wrapping_add(2);
        self.parse_opcode(self.current_opcode);
    }

    /// If delay_timer or sound_timer are greater than 0, they are decremented by 1.
    pub fn update_timers(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        if self.sound_timer > 0 {
            if !self.is_mute {
                self.audio.play(true).unwrap();
            }
            self.sound_timer -= 1;
        } else if !self.is_mute {
            self.audio.pause().unwrap();
        }
    }
}
