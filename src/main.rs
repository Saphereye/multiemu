extern crate minifb;

use minifb::{Key, Window, WindowOptions};
use rand::Rng;
use std::fmt;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::thread;
use std::time::{Duration, Instant};

struct CPU {
    memory: Vec<u8>,
    display: Vec<bool>,
    program_counter: u16,
    stack_pointer: u16,
    i: u16,
    function_stack: Vec<u16>,
    delay_timer: u8,
    sound_timer: u8,
    v: Vec<u8>,
}

impl fmt::Debug for CPU {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CPU")
            .field("memory", &"...")
            .field("display", &"...")
            .field("program_counter", &format!("{:x}", self.program_counter))
            .field("i", &format!("{:x}", self.i))
            .field("function_stack", &"...")
            .field("delay_timer", &format!("{:x}", self.delay_timer))
            .field("sound_timer", &format!("{:x}", self.sound_timer))
            .field("v0", &format!("{:x}", self.v[0x0]))
            .field("v1", &format!("{:x}", self.v[0x1]))
            .field("v2", &format!("{:x}", self.v[0x2]))
            .field("v3", &format!("{:x}", self.v[0x3]))
            .field("v4", &format!("{:x}", self.v[0x4]))
            .field("v5", &format!("{:x}", self.v[0x5]))
            .field("v6", &format!("{:x}", self.v[0x6]))
            .field("v7", &format!("{:x}", self.v[0x7]))
            .field("v8", &format!("{:x}", self.v[0x8]))
            .field("v9", &format!("{:x}", self.v[0x9]))
            .field("va", &format!("{:x}", self.v[0xa]))
            .field("vb", &format!("{:x}", self.v[0xb]))
            .field("vc", &format!("{:x}", self.v[0xc]))
            .field("vd", &format!("{:x}", self.v[0xd]))
            .field("ve", &format!("{:x}", self.v[0xe]))
            .field("vf", &format!("{:x}", self.v[0xf]))
            .finish()
    }
}

impl CPU {
    fn default() -> Self {
        let fontset: Vec<u8> = vec![
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

        let mut memory = vec![0; 4096];
        memory[0..fontset.len()].copy_from_slice(&fontset);

        Self {
            memory: memory,
            display: vec![false; 64 * 32],
            program_counter: 0,
            stack_pointer: 0,
            i: 0,
            function_stack: Vec::new(),
            delay_timer: 0,
            sound_timer: 0,
            v: vec![0; 16],
        }
    }

    fn get_display(&self) -> Vec<u32> {
        return self
            .display
            .iter()
            .map(|&b| if b { 0xFFFFFF } else { 0 })
            .collect();
    }

    fn print_memory(&self) {
        for (i, chunk) in self.memory.chunks(16).enumerate() {
            print!("{:04X}: ", i * 16);
            for &value in chunk {
                print!("{:02X} ", value);
            }
            println!();
        }
    }
}

fn main() {
    println!("Starting.");
    // Dimensions of the window
    let width = 64;
    let height = 32;
    let scaling = 5;
    let target_frequency_hz = 60;
    let iteration_duration = Duration::from_micros(1_000_000 / target_frequency_hz);

    // Create a new window
    let mut window = Window::new(
        "Pixel Grid",
        width * scaling,
        height * scaling,
        WindowOptions {
            borderless: false,
            title: true,
            resize: true,
            scale: minifb::Scale::X4,
            scale_mode: minifb::ScaleMode::Stretch,
            topmost: false,
        },
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    let mut cpu = CPU::default();

    // Draw the grid
    for y in 0..height {
        for x in 0..width {
            // Calculate the index in the buffer
            let index = y * width + x;

            // Color the pixels to form the grid pattern
            if (x + y) % 2 == 0 {
                cpu.display[index] = true; // White color
            }
        }
    }

    // Open the CHIP-8 source file
    let file_path = "src/IBM Logo.ch8";
    let mut file = File::open(file_path).expect("Failed to open file");

    // Read the contents of the file into a Vec<u8>
    let mut file_contents: Vec<u8> = Vec::new();
    file.read_to_end(&mut file_contents)
        .expect("Failed to read file");

    // Convert Vec<u8> to Vec<u16>
    let mut program: Vec<u16> = Vec::new();
    let mut i = 0;
    while i < file_contents.len() {
        let opcode = ((file_contents[i] as u16) << 8) | (file_contents[i + 1] as u16);
        program.push(opcode);
        i += 2;
    }

    let instructions: Vec<u16> = program;
    // let instructions: Vec<u16> = vec![0x00e0, ];

    // Set up the main loop
    while window.is_open() && !window.is_key_down(Key::Escape) {
        thread::sleep(iteration_duration);
        println!("{:?}", cpu);
        // // Clear the buffer to black
        // buffer.iter_mut().for_each(|pixel| *pixel = 0);

        if cpu.program_counter as usize >= instructions.len() {
            break;
        }
        // println!(
        //     "{:?}",
        //     (
        //         (instructions[cpu.program_counter] & 0xF000) >> 12,
        //         (instructions[cpu.program_counter] & 0x0F00) >> 8,
        //         (instructions[cpu.program_counter] & 0x00F0) >> 4,
        //         (instructions[cpu.program_counter] & 0x000F)
        //     )
        // );

        let opcode = instructions[cpu.program_counter as usize];

        print!("{:x}: ", opcode);

        // todo: need to deal with carry flag

        match (opcode & 0xF000) >> 12 {
            0x0 => {
                let nnn = opcode & 0x0FFF;
                match nnn {
                    // Nested match on the value of nnn
                    0x0E0 => {
                        cpu.display = vec![false; width * height];
                        println!("CLS")
                    }
                    0x0EE => {
                        cpu.program_counter = match cpu.function_stack.last() {
                            Some(a) => *a,
                            _ => 0,
                        };
                        cpu.stack_pointer -= 1;
                        println!("RET")
                    }
                    _ => {
                        println!("NOOP");
                        // println!("Execute machine language subroutine at {}", nnn)
                    }
                }
            }
            0x1 => {
                let nnn = opcode & 0x0FFF;
                cpu.program_counter = nnn;
                println!("JP {:x}", nnn);
                continue;
            }
            0x2 => {
                let nnn = opcode & 0x0FFF;
                cpu.stack_pointer += 1;
                cpu.function_stack.push(cpu.program_counter);
                cpu.program_counter = nnn;
                println!("CALL {:x}", nnn);
                continue;
            }
            0x3 => {
                let x = ((opcode & 0x0F00) >> 8) as usize;
                let kk = opcode & 0x00FF;
                if cpu.v[x] == kk as u8 {
                    cpu.program_counter += 1;
                }
                println!("SE V{:x}, {:x}", x, kk);
            }
            0x4 => {
                let x = ((opcode & 0x0F00) >> 8) as usize;
                let kk = opcode & 0x00FF;
                if cpu.v[x] != kk as u8 {
                    cpu.program_counter += 1;
                }
                println!("SNE V{:x}, {:x}", x, kk);
            }
            0x5 => {
                let x = ((opcode & 0x0F00) >> 8) as usize;
                let y = ((opcode & 0x00F0) >> 8) as usize;
                if cpu.v[x] == cpu.v[y] {
                    cpu.program_counter += 1;
                }
                println!("SE V{:x}, V{:x}", x, y);
            }
            0x6 => {
                let x = ((opcode & 0x0F00) >> 8) as usize;
                let kk = opcode & 0x00FF;
                cpu.v[x] = kk as u8;
                println!("LD V{:x}, {:x}", x, kk);
            }
            0x7 => {
                let x = ((opcode & 0x0F00) >> 8) as usize;
                let kk = opcode & 0x00FF;
                cpu.v[x] += kk as u8;
                println!("ADD V{:x}, {:x}", x, kk);
            }
            // todo: set the carry flags accordingly for 8xxx
            0x8 => match opcode & 0x000F {
                0x0 => {
                    let x = ((opcode & 0x0F00) >> 8) as usize;
                    let y = ((opcode & 0x00F0) >> 8) as usize;
                    cpu.v[x] = cpu.v[y];
                    println!("LD V{:x}, V{:x}", x, y);
                }
                0x1 => {
                    let x = ((opcode & 0x0F00) >> 8) as usize;
                    let y = ((opcode & 0x00F0) >> 8) as usize;
                    cpu.v[x] |= cpu.v[y];
                    println!("OR V{:x}, V{:x}", x, y);
                }
                0x2 => {
                    let x = ((opcode & 0x0F00) >> 8) as usize;
                    let y = ((opcode & 0x00F0) >> 8) as usize;
                    cpu.v[x] &= cpu.v[y];
                    println!("AND V{:x}, V{:x}", x, y);
                }
                0x3 => {
                    let x = ((opcode & 0x0F00) >> 8) as usize;
                    let y = ((opcode & 0x00F0) >> 8) as usize;
                    cpu.v[x] ^= cpu.v[y];
                    println!("XOR V{:x}, V{:x}", x, y);
                }
                0x4 => {
                    let x = ((opcode & 0x0F00) >> 8) as usize;
                    let y = ((opcode & 0x00F0) >> 8) as usize;
                    cpu.v[x] += cpu.v[y];
                    println!("ADD V{:x}, V{:x}", x, y);
                }
                0x5 => {
                    let x = ((opcode & 0x0F00) >> 8) as usize;
                    let y = ((opcode & 0x00F0) >> 8) as usize;
                    cpu.v[x] -= cpu.v[y];
                    println!("SUB V{:x}, V{:x}", x, y);
                }
                0x6 => {
                    let x = ((opcode & 0x0F00) >> 8) as usize;
                    let y = ((opcode & 0x00F0) >> 8) as usize;
                    cpu.v[x] >>= 1;
                    println!("SHR V{:x}, V{:x}", x, y);
                }
                0x7 => {
                    let x = ((opcode & 0x0F00) >> 8) as usize;
                    let y = ((opcode & 0x00F0) >> 8) as usize;
                    cpu.v[x] = cpu.v[y] - cpu.v[x];
                    println!("SUBN V{:x}, V{:x}", x, y);
                }
                0xE => {
                    let x = ((opcode & 0x0F00) >> 8) as usize;
                    let y = ((opcode & 0x00F0) >> 8) as usize;
                    cpu.v[x] <<= 1;
                    println!("SHL V{:x}, V{:x}", x, y);
                }
                _ => println!("Unknow opcode ({:x})", opcode),
            },
            0x9 => {
                let x = ((opcode & 0x0F00) >> 8) as usize;
                let y = ((opcode & 0x00F0) >> 8) as usize;
                if cpu.v[x] != cpu.v[y] {
                    cpu.program_counter += 1;
                }
                println!("SNE V{:x}, V{:x}", x, y);
            }
            0xA => {
                let nnn = opcode & 0x0FFF;
                cpu.i = nnn;
                println!("LD I, {:x}", nnn);
            }
            0xB => {
                let nnn = opcode & 0x0FFF;
                cpu.program_counter = cpu.v[0] as u16 + nnn;
                println!("JP V0, {:x}", nnn);
            }
            0xC => {
                let x = ((opcode & 0x0F00) >> 8) as usize;
                let kk = opcode & 0x00FF;
                let mut rng = rand::thread_rng();
                let random_byte: u8 = rng.gen();
                cpu.v[x] = random_byte & kk as u8;
                println!("RND V{:x}, {:x}", x, kk);
            }
            // More cases for other opcodes...
            _ => println!("Unknown opcode ({:x})", opcode),
        }

        // Update the window with the new buffer
        window
            .update_with_buffer(&cpu.get_display(), width, height)
            .unwrap();

        cpu.program_counter += 1;

        // // Handle window events
        // if let Some(keys) = window.get_keys() {
        //     for key in keys {
        //         match key {
        //             Key::Escape => break,
        //             _ => {}
        //         }
        //     }
        // }
    }
    println!("{:?}", cpu);
    cpu.print_memory();
    println!("Bye Bye.");
}
