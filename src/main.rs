use clap::Parser;
use eframe::egui;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

mod configs;
mod cpu;
mod rand;

use configs::{HEIGHT, PROGRAM_START_ADDRESS, WIDTH};
use cpu::Cpu;

/// Map CHIP-8 keys to egui keys
use egui::Key;

use crate::configs::FONTSET_START_ADDRESS;
const KEY_MAP: [(usize, Key); 16] = [
    (0x0, Key::X),
    (0x1, Key::Num1),
    (0x2, Key::Num2),
    (0x3, Key::Num3),
    (0x4, Key::Q),
    (0x5, Key::W),
    (0x6, Key::E),
    (0x7, Key::A),
    (0x8, Key::S),
    (0x9, Key::D),
    (0xA, Key::Z),
    (0xB, Key::C),
    (0xC, Key::Num4),
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
    /// ROM file path
    file: PathBuf,

    /// Number of CPU instructions per timer update (timer is at 60Hz)
    #[arg(short, long, default_value_t = 1)]
    cycles: u64,

    /// Enable to mute the beep sound
    #[arg(short, long, default_value_t = false)]
    mute: bool,
}

pub struct App {
    cpu: Cpu,
    cycles: u64,
    texture: Option<egui::TextureHandle>,
    last_timer_update: Instant,
    timer_period: Duration,
    memory_scroll_to: Option<usize>,
    is_paused: bool,
}

impl App {
    fn new(cpu: Cpu, cycles: u64) -> Self {
        Self {
            cpu,
            cycles,
            texture: None,
            last_timer_update: Instant::now(),
            timer_period: Duration::from_nanos(16_666_667), // ~60Hz
            memory_scroll_to: None,
            is_paused: false,
        }
    }

    fn update_texture(&mut self, ctx: &egui::Context) {
        let rgba: Vec<u8> = self
            .cpu
            .buffer
            .iter()
            .flat_map(|&b| {
                if b {
                    [255, 255, 255, 255] // white
                } else {
                    [0, 0, 0, 255] // black
                }
            })
            .collect();

        let size = [WIDTH, HEIGHT];
        let image = egui::ColorImage::from_rgba_unmultiplied(size, &rgba);

        if let Some(tex) = &mut self.texture {
            tex.set(image, egui::TextureOptions::NEAREST);
        } else {
            self.texture =
                Some(ctx.load_texture("chip8_screen", image, egui::TextureOptions::NEAREST));
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Only request frequent repaints when running
        if !self.is_paused {
            ctx.request_repaint_after(std::time::Duration::from_millis(16)); // 60Hz when running
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(100)); // 10Hz when paused
        }

        // --- Keyboard input ---
        ctx.input(|i| {
            for (chip8_key, egui_key) in KEY_MAP {
                self.cpu.input_keys[chip8_key] = i.key_down(egui_key);
            }
        });

        // --- Execute instructions (only if not paused) ---
        if !self.is_paused {
            for _ in 0..self.cycles {
                self.cpu.execute_instruction();
            }
            
            // Add a small yield to prevent 100% CPU usage
            std::thread::sleep(std::time::Duration::from_micros(100));
        }

        // --- Timers ---
        if !self.is_paused {
            let now = Instant::now();
            if now.duration_since(self.last_timer_update) >= self.timer_period {
                self.cpu.update_timers();
                self.last_timer_update = now;
            }
        }

        // --- Redraw display if needed ---
        if self.cpu.to_draw {
            self.update_texture(ctx);
            self.cpu.to_draw = false;
        }

        // Get available screen size for responsive design
        let screen_rect = ctx.screen_rect();
        let available_width = screen_rect.width();

        // Make panels wider to take more space from display
        let panel_width = (available_width * 0.25).clamp(200.0, 350.0);

        // --- LEFT PANEL: Registers + Keys + Controls + Stack ---
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(panel_width)
            .width_range(0.0..=200.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    // Control buttons
                    ui.heading("Controls");
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .button(if self.is_paused {
                                "▶ Run"
                            } else {
                                "⏸ Pause"
                            })
                            .clicked()
                        {
                            self.is_paused = !self.is_paused;
                        }
                        if ui.button("⏹ Reset").clicked() {
                            // Reset CPU state
                            self.cpu.program_counter = PROGRAM_START_ADDRESS;
                            self.cpu.index_register = 0;
                            self.cpu.stack_pointer = 0;
                            self.cpu.delay_timer = 0;
                            self.cpu.sound_timer = 0;
                            self.cpu.registers = [0; 16];
                            self.cpu.buffer = [false; WIDTH * HEIGHT];
                            self.cpu.to_draw = true;
                            self.is_paused = true;
                        }
                    });

                    ui.add_space(8.0);

                    // Speed control with slider
                    ui.label("Speed:");
                    let mut speed = self.cycles as f32;
                    ui.add(egui::Slider::new(&mut speed, 1.0..=20.0)
                        .logarithmic(true)
                        .text("cycles")
                        .show_value(true));
                    self.cycles = speed as u64;
                    ui.small(format!("{}x speed", self.cycles));

                    ui.separator();

                    // Compact registers in 4 columns with larger font
                    ui.heading("Registers");
                    egui::Grid::new("registers_grid")
                        .num_columns(4)
                        .spacing([8.0, 2.0])
                        .show(ui, |ui| {
                            for (i, reg) in self.cpu.registers.iter().enumerate() {
                                ui.label(egui::RichText::new(format!("V{:X}:{:02X}", i, reg))
                                    .size(12.0)
                                    .monospace());
                                if (i + 1) % 4 == 0 {
                                    ui.end_row();
                                }
                            }
                        });

                    ui.add_space(4.0);

                    // System registers in 2 columns
                    egui::Grid::new("sys_registers_grid")
                        .num_columns(2)
                        .spacing([8.0, 2.0])
                        .show(ui, |ui| {
                            ui.small(format!("I:{:04X}", self.cpu.index_register));
                            ui.small(format!("PC:{:04X}", self.cpu.program_counter));
                            ui.end_row();
                            ui.small(format!("SP:{}", self.cpu.stack_pointer));
                            ui.small(format!("OP:{:04X}", self.cpu.current_opcode));
                            ui.end_row();
                            ui.small(format!("DT:{}", self.cpu.delay_timer));
                            ui.small(format!("ST:{}", self.cpu.sound_timer));
                        });

                    ui.separator();
                    ui.heading("Keys");

                    // Compact keypad
                    let keys = [
                        [0x1, 0x2, 0x3, 0xC],
                        [0x4, 0x5, 0x6, 0xD],
                        [0x7, 0x8, 0x9, 0xE],
                        [0xA, 0x0, 0xB, 0xF],
                    ];

                    let button_size = ((ui.available_width() - 24.0) / 4.0).clamp(20.0, 35.0);

                    for row in keys {
                        ui.horizontal(|ui| {
                            for &k in &row {
                                let pressed = self.cpu.input_keys[k];
                                let button = egui::Button::new(format!("{:X}", k))
                                    .min_size(egui::vec2(button_size, button_size));
                                if pressed {
                                    ui.add_sized(
                                        [button_size, button_size],
                                        button.fill(egui::Color32::GREEN),
                                    );
                                } else {
                                    ui.add_sized([button_size, button_size], button);
                                }
                            }
                        });
                    }

                    // --- STACK SECTION (moved from top panel) ---
                    ui.separator();
                    ui.heading("Stack");

                    if self.cpu.stack_pointer == 0 {
                        ui.label("Empty");
                    } else {
                        // Stack visualization as a vertical list for better readability
                        ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);

                        for i in (0..self.cpu.stack_pointer).rev() {
                            if (i as usize) < self.cpu.stack.len() {
                                let color = if i == self.cpu.stack_pointer - 1 {
                                    egui::Color32::YELLOW
                                } else {
                                    egui::Color32::WHITE
                                };
                                ui.horizontal(|ui| {
                                    ui.colored_label(color, format!("[{}]", i));
                                    ui.colored_label(
                                        color,
                                        format!("0x{:04X}", self.cpu.stack[i as usize]),
                                    );
                                });
                            }
                        }

                        ui.style_mut().override_text_style = None; // Reset text style
                    }
                });
            });

        // --- RIGHT PANEL: Memory Viewer ---
        egui::SidePanel::right("right_panel")
            .resizable(true)
            .default_width(panel_width)
            .width_range(0.0..=420.0)
            .show(ctx, |ui| {
                ui.heading("Memory");

                // Navigation controls
                ui.horizontal_wrapped(|ui| {
                    if ui.small_button("Program Counter").clicked() {
                        self.memory_scroll_to = Some(self.cpu.program_counter as usize);
                    }
                    if ui.small_button("Index Register").clicked() {
                        self.memory_scroll_to = Some(self.cpu.index_register as usize);
                    }
                    if ui.small_button("Program Start").clicked() {
                        self.memory_scroll_to = Some(PROGRAM_START_ADDRESS as usize);
                    }
                    if ui.small_button("Font Start").clicked() {
                        self.memory_scroll_to = Some(FONTSET_START_ADDRESS as usize);
                    }
                });

                ui.separator();

                let mut scroll_area = egui::ScrollArea::vertical()
                    .id_salt("memory_scroll")
                    .auto_shrink([false, false]);

                if let Some(scroll_to) = self.memory_scroll_to.take() {
                    let row = scroll_to / 16;
                    scroll_area = scroll_area.vertical_scroll_offset((row as f32) * 18.0);
                }

                scroll_area.show(ui, |ui| {
                    ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);

                    for (addr, chunk) in self.cpu.memory.chunks(16).enumerate() {
                        let base_addr = addr * 16;

                        ui.horizontal(|ui| {
                            // Address
                            let addr_color = if base_addr == self.cpu.program_counter as usize {
                                egui::Color32::YELLOW
                            } else if base_addr <= self.cpu.index_register as usize
                                && (self.cpu.index_register as usize) < base_addr + 16
                            {
                                egui::Color32::LIGHT_BLUE
                            } else {
                                egui::Color32::GRAY
                            };

                            ui.colored_label(addr_color, format!("{:04X}:", base_addr));

                            // Hex bytes
                            for (i, &byte) in chunk.iter().enumerate() {
                                let byte_addr = base_addr + i;
                                let color = if byte_addr == self.cpu.program_counter as usize {
                                    egui::Color32::YELLOW
                                } else if byte_addr == self.cpu.index_register as usize {
                                    egui::Color32::LIGHT_BLUE
                                } else if byte != 0 {
                                    egui::Color32::WHITE
                                } else {
                                    egui::Color32::DARK_GRAY
                                };

                                ui.colored_label(color, format!("{:02X}", byte));
                            }
                        });
                    }
                });
            });

        // --- BOTTOM PANEL: Instructions/Disassembly (expanded) ---
        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(true)
            .default_height(150.0)
            .height_range(300.0..=400.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Instructions");
                    if ui.button("Follow PC").clicked() {
                        // Auto-scroll will happen in the scroll area below
                    }
                });

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);

                        // Show more instructions around PC for better context
                        let pc = self.cpu.program_counter as usize;
                        let start = pc.saturating_sub(20) & !1; // Align to even address
                        let end = (pc + 40).min(self.cpu.memory.len() - 1) & !1;

                        for addr in (start..end).step_by(2) {
                            if addr + 1 < self.cpu.memory.len() {
                                let opcode = ((self.cpu.memory[addr] as u16) << 8) | (self.cpu.memory[addr + 1] as u16);

                                ui.horizontal(|ui| {
                                    let is_current = addr == pc;
                                    let color = if is_current {
                                        egui::Color32::YELLOW
                                    } else {
                                        egui::Color32::WHITE
                                    };

                                    // Add background highlight for current instruction
                                    if is_current {
                                        ui.painter().rect_filled(
                                            ui.available_rect_before_wrap(),
                                            0.0,
                                            egui::Color32::from_rgba_unmultiplied(255, 255, 0, 30),
                                        );
                                    }

                                    ui.colored_label(color, format!("{:04X}: {:04X}", addr, opcode));

                                    ui.separator();

                                    // Simple instruction decoding with expanded format
                                    let instruction = match opcode & 0xF000 {
                                        0x0000 => match opcode {
                                            0x00E0 => "CLS              ; Clear display".to_string(),
                                            0x00EE => "RET              ; Return from subroutine".to_string(),
                                            _ => format!("SYS {:03X}         ; Call system routine", opcode & 0x0FFF),
                                        },
                                        0x1000 => format!("JP {:03X}          ; Jump to address {:03X}", opcode & 0x0FFF, opcode & 0x0FFF),
                                        0x2000 => format!("CALL {:03X}        ; Call subroutine at {:03X}", opcode & 0x0FFF, opcode & 0x0FFF),
                                        0x3000 => format!("SE V{:X}, {:02X}       ; Skip if V{:X} == {:02X}", (opcode & 0x0F00) >> 8, opcode & 0x00FF, (opcode & 0x0F00) >> 8, opcode & 0x00FF),
                                        0x4000 => format!("SNE V{:X}, {:02X}      ; Skip if V{:X} != {:02X}", (opcode & 0x0F00) >> 8, opcode & 0x00FF, (opcode & 0x0F00) >> 8, opcode & 0x00FF),
                                        0x5000 => format!("SE V{:X}, V{:X}       ; Skip if V{:X} == V{:X}", (opcode & 0x0F00) >> 8, (opcode & 0x00F0) >> 4, (opcode & 0x0F00) >> 8, (opcode & 0x00F0) >> 4),
                                        0x6000 => format!("LD V{:X}, {:02X}       ; Load {:02X} into V{:X}", (opcode & 0x0F00) >> 8, opcode & 0x00FF, opcode & 0x00FF, (opcode & 0x0F00) >> 8),
                                        0x7000 => format!("ADD V{:X}, {:02X}      ; Add {:02X} to V{:X}", (opcode & 0x0F00) >> 8, opcode & 0x00FF, opcode & 0x00FF, (opcode & 0x0F00) >> 8),
                                        0x8000 => {
                                            let x = (opcode & 0x0F00) >> 8;
                                            let y = (opcode & 0x00F0) >> 4;
                                            match opcode & 0x000F {
                                                0x0 => format!("LD V{:X}, V{:X}       ; V{:X} = V{:X}", x, y, x, y),
                                                0x1 => format!("OR V{:X}, V{:X}       ; V{:X} |= V{:X}", x, y, x, y),
                                                0x2 => format!("AND V{:X}, V{:X}      ; V{:X} &= V{:X}", x, y, x, y),
                                                0x3 => format!("XOR V{:X}, V{:X}      ; V{:X} ^= V{:X}", x, y, x, y),
                                                0x4 => format!("ADD V{:X}, V{:X}      ; V{:X} += V{:X}, VF = carry", x, y, x, y),
                                                0x5 => format!("SUB V{:X}, V{:X}      ; V{:X} -= V{:X}, VF = borrow", x, y, x, y),
                                                0x6 => format!("SHR V{:X}           ; V{:X} >>= 1, VF = LSB", x, x),
                                                0x7 => format!("SUBN V{:X}, V{:X}     ; V{:X} = V{:X} - V{:X}, VF = borrow", x, y, x, y, x),
                                                0xE => format!("SHL V{:X}           ; V{:X} <<= 1, VF = MSB", x, x),
                                                _ => format!("8{:X}{:X}{:X}          ; Unknown 8xxx instruction", x, y, opcode & 0x000F),
                                            }
                                        },
                                        0x9000 => format!("SNE V{:X}, V{:X}      ; Skip if V{:X} != V{:X}", (opcode & 0x0F00) >> 8, (opcode & 0x00F0) >> 4, (opcode & 0x0F00) >> 8, (opcode & 0x00F0) >> 4),
                                        0xA000 => format!("LD I, {:03X}        ; I = {:03X}", opcode & 0x0FFF, opcode & 0x0FFF),
                                        0xB000 => format!("JP V0, {:03X}       ; Jump to V0 + {:03X}", opcode & 0x0FFF, opcode & 0x0FFF),
                                        0xC000 => format!("RND V{:X}, {:02X}      ; V{:X} = random() & {:02X}", (opcode & 0x0F00) >> 8, opcode & 0x00FF, (opcode & 0x0F00) >> 8, opcode & 0x00FF),
                                        0xD000 => format!("DRW V{:X}, V{:X}, {:X}    ; Draw sprite at (V{:X}, V{:X}) height {:X}", (opcode & 0x0F00) >> 8, (opcode & 0x00F0) >> 4, opcode & 0x000F, (opcode & 0x0F00) >> 8, (opcode & 0x00F0) >> 4, opcode & 0x000F),
                                        0xE000 => {
                                            let x = (opcode & 0x0F00) >> 8;
                                            match opcode & 0x00FF {
                                                0x9E => format!("SKP V{:X}          ; Skip if key V{:X} pressed", x, x),
                                                0xA1 => format!("SKNP V{:X}         ; Skip if key V{:X} not pressed", x, x),
                                                _ => format!("E{:X}{:02X}           ; Unknown Exxx instruction", x, opcode & 0x00FF),
                                            }
                                        },
                                        0xF000 => {
                                            let x = (opcode & 0x0F00) >> 8;
                                            match opcode & 0x00FF {
                                                0x07 => format!("LD V{:X}, DT       ; V{:X} = delay timer", x, x),
                                                0x0A => format!("LD V{:X}, K        ; Wait for key, store in V{:X}", x, x),
                                                0x15 => format!("LD DT, V{:X}       ; Delay timer = V{:X}", x, x),
                                                0x18 => format!("LD ST, V{:X}       ; Sound timer = V{:X}", x, x),
                                                0x1E => format!("ADD I, V{:X}       ; I += V{:X}", x, x),
                                                0x29 => format!("LD F, V{:X}        ; I = sprite address for digit V{:X}", x, x),
                                                0x33 => format!("LD B, V{:X}        ; Store BCD of V{:X} at [I]", x, x),
                                                0x55 => format!("LD [I], V{:X}      ; Store V0-V{:X} at [I]", x, x),
                                                0x65 => format!("LD V{:X}, [I]      ; Load V0-V{:X} from [I]", x, x),
                                                _ => format!("F{:X}{:02X}           ; Unknown Fxxx instruction", x, opcode & 0x00FF),
                                            }
                                        },
                                        _ => format!("UNKNOWN {:04X}    ; Unrecognized instruction", opcode),
                                    };

                                    ui.colored_label(color, instruction);
                                });
                            }
                        }
                    });
            });

        // --- CENTER PANEL: Chip-8 Display (smaller) ---
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(tex) = &self.texture {
                let available_rect = ui.available_rect_before_wrap();

                // Make display smaller to give more space to panels
                let max_scale = 12.0; // Reduced from previous scaling
                let tex_size = tex.size_vec2();
                let scale_x = available_rect.width() / tex_size.x;
                let scale_y = available_rect.height() / tex_size.y;
                let scale = (scale_x.min(scale_y)).min(max_scale).max(2.0);

                let display_size = tex_size * scale;

                // Center the display
                let center_pos = available_rect.center() - display_size / 2.0;

                ui.allocate_ui_at_rect(egui::Rect::from_min_size(center_pos, display_size), |ui| {
                    ui.image((tex.id(), display_size));
                });
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Loading display...");
                });
            }
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let cli = Cli::parse();
    let mut cpu = load_rom(&cli.file).expect("Failed to load ROM");
    cpu.is_mute = cli.mute;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0]) // Larger default for more info panels
            .with_min_inner_size([1000.0, 700.0])
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "Chip-8",
        options,
        Box::new(|_cc| Ok(Box::new(App::new(cpu, cli.cycles)))),
    )
}
