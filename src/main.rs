use clap::Parser;
use minifb::{Key, Scale, Window, WindowOptions};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

mod configs;
mod cpu;
mod rand;

use configs::{HEIGHT, PROGRAM_START_ADDRESS, WIDTH};
use cpu::Cpu;

const KEY_MAP: [(i32, Key); 16] = [
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
];

fn load_rom(file_path: &Path) -> std::io::Result<Cpu> {
    let mut cpu = Cpu::new();
    let mut file = File::open(file_path)?;
    file.read(&mut cpu.memory[PROGRAM_START_ADDRESS as usize..])?;
    Ok(cpu)
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Input file name
    file: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let mut cpu = load_rom(&cli.file)?;
    let mut window = Window::new(
        "Chip-8",
        WIDTH,
        HEIGHT,
        WindowOptions {
            scale: Scale::X8,
            ..WindowOptions::default()
        },
    )?;

    let timer_period = Duration::from_nanos(16_666_667); // Exactly 60Hz
    let mut last_timer_update = Instant::now();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let now = Instant::now();

        for (key, value) in KEY_MAP.iter() {
            cpu.input_keys[*key as usize] = window.is_key_down(*value);
        }

        cpu.execute_instruction(); // Execute one instruction

        if now.duration_since(last_timer_update) >= timer_period {
            cpu.update_timers();
            last_timer_update = now;
        }

        cpu.previous_input_keys = cpu.input_keys;

        let buffer: Vec<u32> = cpu
            .buffer
            .iter()
            .map(|b| if *b { 0xFFFFFF } else { 0 })
            .collect();
        window.update_with_buffer(&buffer, WIDTH, HEIGHT)?;

        std::thread::sleep(Duration::from_micros(50));
    }
    Ok(())
}
