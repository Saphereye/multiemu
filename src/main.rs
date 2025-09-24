use clap::Parser;
use minifb::{Key, Scale, Window, WindowOptions};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::time::Duration;

const WIDTH: usize = 64;
const HEIGHT: usize = 32;
const PROGRAM_START_ADDRESS: u16 = 0x200;
const FONTSET_START_ADDRESS: u16 = 0x50;

/// The CHIP-8 font set.
static FONT_SET: LazyLock<[u8; 80]> = LazyLock::new(|| {
    [
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
    ]
});

/// Maps the CHIP-8 keypad to the keyboard.
static KEY_MAP: LazyLock<HashMap<u8, Key>> = LazyLock::new(|| {
    HashMap::from([
        (0x0, Key::X),
        (0x1, Key::Key1),
        (0x2, Key::Key2),
        (0x3, Key::Key3),
        (0x4, Key::Q),
        (0x5, Key::W),
        (0x6, Key::E),
        (0x7, Key::A),
        (0x8, Key::S),
        (0x9, Key::D),
        (0xA, Key::Z),
        (0xB, Key::C),
        (0xC, Key::Key4),
        (0xD, Key::R),
        (0xE, Key::F),
        (0xF, Key::V),
    ])
});

/// Implementation of the Chip-8 CPU
struct Cpu {
    registers: [u8; 16],
    memory: [u8; 4096],
    index_register: u16,
    program_counter: u16,
    stack: [u16; 16], // 16 levels
    stack_pointer: u8,
    delay_timer: u8,
    sound_timer: u8,
    input_keys: [bool; 16],                     // true if nth key is pressed
    previous_input_keys: [bool; 16],
    monochrome_display: [bool; WIDTH * HEIGHT], // true if pixel is on
    current_opcode: u16,
}

impl Cpu {
    /// Initialize the CPU with default values.
    ///
    /// The program counter is set to 0x200, the start address of the program.
    /// All registers are set to 0 and the monochrome display is cleared.
    fn new() -> Self {
        let mut memory = vec![0; 4096];
        memory[FONTSET_START_ADDRESS as usize
            ..(FONTSET_START_ADDRESS + FONT_SET.len() as u16) as usize]
            .copy_from_slice(&*FONT_SET);
        Self {
            registers: [0; 16],
            memory: [0; 4096],
            index_register: 0,
            program_counter: PROGRAM_START_ADDRESS,
            stack: [0; 16],
            stack_pointer: 0,
            delay_timer: 0,
            sound_timer: 0,
            input_keys: [false; 16],
            previous_input_keys: [false; 16],
            current_opcode: 0,
            monochrome_display: [false; 64 * 32],
        }
    }

    /// Loads the ROM into memory starting at 0x200 given file name.
    ///
    /// # Examples
    /// ```
    /// let mut cpu = Cpu::default();
    /// cpu.load_rom("roms/tetris.ch8")?;
    ///
    /// println!("{:?}", cpu.get_buffer());
    /// ```
    fn load_rom(&mut self, file_path: &Path) -> io::Result<()> {
        let mut file = File::open(file_path)?;
        file.read_exact(&mut self.memory[PROGRAM_START_ADDRESS as usize..])?;
        Ok(())
    }

    /// Parses the opcode and executes the instruction.
    ///
    /// Uses the CHIP-8 instruction set. A
    ///
    /// # Panics
    /// If the input instruction is not a valid CHIP-8 instruction, unreachable!() is called.
    fn parse_opcode(&mut self, opcode: u16) {
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let y = ((opcode & 0x00F0) >> 4) as usize;
        match opcode {
            0x00E0 => {
                // CLS, clear display
                self.monochrome_display = [false; 64 * 32];
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
                    self.program_counter += 2;
                }
            }
            0x4000..=0x4FFF => {
                // SNE Vx, byte, skip next instruction if Vx != byte
                let byte = (opcode & 0x00FF) as u8;
                if self.registers[x] != byte {
                    self.program_counter += 2;
                }
            }
            0x5000..=0x5FFF => {
                // SE Vx, Vy, skip next instruction if Vx == Vy
                if self.registers[x] == self.registers[y] {
                    self.program_counter += 2;
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
                    }
                    0x2 => {
                        // AND Vx, Vy, Vx &= Vy
                        self.registers[x] &= self.registers[y];
                    }
                    0x3 => {
                        // XOR Vx, Vy, Vx ^= Vy
                        self.registers[x] ^= self.registers[y];
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
                        // SHR Vx {, Vy}, Vx >>= 1, VF = carry
                        self.registers[0xF] = self.registers[x] & 0x1;
                        self.registers[x] >>= 1;
                    }
                    0x7 => {
                        // SUBN Vx, Vy, Vx = Vy - Vx, VF = !borrow
                        let (result, overflow) =
                            self.registers[y].overflowing_sub(self.registers[x]);
                        self.registers[x] = result;
                        self.registers[0xF] = !overflow as u8;
                    }
                    0xE => {
                        // SHL Vx {, Vy}, Vx <<= 1, VF = carry
                        self.registers[0xF] = (self.registers[x] & 0x80) >> 7;
                        self.registers[x] <<= 1;
                    }
                    _ => unreachable!("opcode {:X}", opcode),
                }
            }
            0x9000..=0x9FFF => {
                // SNE Vx, Vy, skip next instruction if Vx != Vy
                if self.registers[x] != self.registers[y] {
                    self.program_counter += 2;
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
                self.registers[x] = rand::random::<u8>() & byte;
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
                        let screen_pixel = &mut self.monochrome_display[screen_index];
                        if sprite_pixel != 0 {
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
                        if self.input_keys[self.registers[x] as usize] {
                            self.program_counter += 2;
                        }
                    }
                    0xA1 => {
                        // SKP Vx, skip next instruction if key Vx is NOT pressed
                        if !self.input_keys[self.registers[x] as usize] {
                            self.program_counter += 2;
                        }
                    }
                    _ => unreachable!("opcode {:X}", opcode),
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
                        let mut key_pressed = false;
                        for index in 0..16 {
                            if self.input_keys[index] && !self.previous_input_keys[index] {
                                self.registers[x] = index as u8;
                                key_pressed = true;
                                break;
                            }
                        }
                        if !key_pressed {
                            self.program_counter -= 2; // Stay on this instruction
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
                        self.index_register += self.registers[x] as u16;
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
                    }
                    0x65 => {
                        // LD Vx, [I], Read registers V0 through Vx from memory starting at location I.
                        for index in 0..=x {
                            self.registers[index] =
                                self.memory[self.index_register as usize + index];
                        }
                    }
                    _ => unreachable!("opcode {:X}", opcode),
                }
            }
            _ => unreachable!("opcode {:X}", opcode),
        }
    }

    /// Simulates one execution cycle of the cpu.
    ///
    /// If delay_timer or sound_timer are greater than 0, they are decremented by 1.
    fn cycle(&mut self) {
        self.current_opcode = (self.memory[self.program_counter as usize] as u16) << 8
            | self.memory[self.program_counter as usize + 1] as u16;
        self.program_counter += 2;
        self.parse_opcode(self.current_opcode);

        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
    }

    /// Returns a mutable reference to the monochrome display buffer.
    fn get_buffer(&mut self) -> &mut [bool; 2048] {
        &mut self.monochrome_display
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Input file name
    file: PathBuf,

    /// Clock delta time (usec)
    #[arg(short, long, default_value = "200")]
    delta: u64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let mut cpu = Cpu::new();
    cpu.load_rom(&cli.file)?;

    let mut window = Window::new(
        "Test - ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions {
            scale: Scale::X8,
            ..WindowOptions::default()
        },
    )?;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        for (key, value) in KEY_MAP.iter() {
            cpu.input_keys[*key as usize] = window.is_key_down(*value);
        }

        std::thread::sleep(Duration::from_micros(cli.delta));
        cpu.cycle();

        cpu.previous_input_keys = cpu.input_keys;

        // We unwrap here as we want this code to exit if it fails. Real applications may want to handle this in a different way
        let buffer: Vec<u32> = cpu
            .get_buffer()
            .iter()
            .map(|b| if *b { 0xFFFFFF } else { 0 })
            .collect();
        window.update_with_buffer(&buffer, WIDTH, HEIGHT)?;
    }

    Ok(())
}
