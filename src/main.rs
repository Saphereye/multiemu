use clap::Parser;
use eframe::egui;
use std::time::{Duration, Instant};

mod emulators;

use emulators::chip8::{Chip8Emulator, Chip8Metadata};
use emulators::Emulator;



#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Number of CPU instructions per timer update (timer is at 60Hz)
    #[arg(short, long, default_value_t = 1)]
    cycles: u64,

    /// Enable to mute the beep sound
    #[arg(short, long, default_value_t = false)]
    mute: bool,
}

pub struct App {
    emulator: Box<dyn Emulator<Metadata = Chip8Metadata>>,
    cycles: u64,
    texture: Option<egui::TextureHandle>,
    last_timer_update: Instant,
    timer_period: Duration,
    memory_scroll_to: Option<usize>,
    is_paused: bool,
    selected_emulator: String,
    rom_path: Option<std::path::PathBuf>,
}

impl App {
    fn new(cycles: u64, mute: bool) -> Self {
        let mut emulator = Chip8Emulator::new();
        emulator.set_mute(mute);
        
        Self {
            emulator: Box::new(emulator),
            cycles,
            texture: None,
            last_timer_update: Instant::now(),
            timer_period: Duration::from_nanos(16_666_667), // ~60Hz
            memory_scroll_to: None,
            is_paused: true,
            selected_emulator: "CHIP-8".to_string(),
            rom_path: None,
        }
    }

    fn update_texture(&mut self, ctx: &egui::Context) {
        let (width, height) = self.emulator.resolution();
        let framebuffer = self.emulator.framebuffer();

        let rgba: Vec<u8> = framebuffer
            .iter()
            .flat_map(|&argb| {
                let a = ((argb >> 24) & 0xFF) as u8;
                let r = ((argb >> 16) & 0xFF) as u8;
                let g = ((argb >> 8) & 0xFF) as u8;
                let b = (argb & 0xFF) as u8;
                [r, g, b, a]
            })
            .collect();

        let size = [width, height];
        let image = egui::ColorImage::from_rgba_unmultiplied(size, &rgba);

        if let Some(tex) = &mut self.texture {
            tex.set(image, egui::TextureOptions::NEAREST);
        } else {
            self.texture = Some(ctx.load_texture(
                &format!("{}_screen", self.emulator.system_name()),
                image,
                egui::TextureOptions::NEAREST,
            ));
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Only request frequent repaints when running to avoid
        // OS complaining about application not responding.
        if !self.is_paused {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        } else {
            ctx.request_repaint_after(std::time::Duration::from_secs(1000));
        }

        // --- Keyboard input ---
        let mut inputs = [false; 16];
        let keymap = self.emulator.keymap();
        ctx.input(|i| {
            for (chip8_key, key_str) in &keymap {
                let key = match key_str.as_str() {
                    "X" => egui::Key::X,
                    "1" => egui::Key::Num1,
                    "2" => egui::Key::Num2,
                    "3" => egui::Key::Num3,
                    "4" => egui::Key::Num4,
                    "Q" => egui::Key::Q,
                    "W" => egui::Key::W,
                    "E" => egui::Key::E,
                    "R" => egui::Key::R,
                    "A" => egui::Key::A,
                    "S" => egui::Key::S,
                    "D" => egui::Key::D,
                    "F" => egui::Key::F,
                    "Z" => egui::Key::Z,
                    "C" => egui::Key::C,
                    "V" => egui::Key::V,
                    _ => continue,
                };
                inputs[*chip8_key] = i.key_down(key);
            }
        });
        self.emulator.set_input_state(&inputs);

        // --- Timers ---
        if !self.is_paused {
            let now = Instant::now();
            let elapsed = now.duration_since(self.last_timer_update);
            if elapsed >= self.timer_period {
                self.emulator.update_timers(elapsed);
                self.last_timer_update = now;
            }
        }

        // --- Execute instructions (only if not paused) ---
        if !self.is_paused {
            for _ in 0..self.cycles {
                self.emulator.step().unwrap_or_else(|e| {
                    log::error!("{}", e);
                    std::process::exit(1);
                });
            }
        }

        // --- Redraw display if needed ---
        self.update_texture(ctx);

        // Get available screen size for responsive design
        let screen_rect = ctx.screen_rect();
        let available_width = screen_rect.width();

        // Make panels wider to take more space from display
        let panel_width = (available_width * 0.25).clamp(200.0, 350.0);

        // --- LEFT PANEL: Registers + Keys + Controls + Stack ---
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(panel_width)
            .width_range(0.0..=220.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    // Emulator selector dropdown
                    ui.heading("Emulator");
                    egui::ComboBox::from_label("")
                        .selected_text(&self.selected_emulator)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.selected_emulator,
                                "CHIP-8".to_string(),
                                "CHIP-8",
                            );
                            // Future emulators can be added here
                            // ui.selectable_value(&mut self.selected_emulator, "Game Boy".to_string(), "Game Boy");
                        });

                    ui.separator();

                    // ROM file selector
                    ui.heading("ROM File");
                    if ui.button("📁 Load ROM").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("ROM files", &["ch8", "rom"])
                            .pick_file()
                        {
                            match self.emulator.load_rom(&path) {
                                Ok(_) => {
                                    self.rom_path = Some(path.clone());
                                    self.is_paused = false;
                                    log::info!("Loaded ROM: {:?}", path);
                                }
                                Err(e) => {
                                    log::error!("Failed to load ROM: {}", e);
                                }
                            }
                        }
                    }
                    if let Some(path) = &self.rom_path {
                        ui.label(format!("📄 {}", path.file_name().unwrap_or_default().to_string_lossy()));
                    }

                    ui.separator();

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
                            self.emulator.reset();
                            self.is_paused = true;
                        }
                    });

                    ui.add_space(8.0);

                    // Speed control with slider
                    ui.label("Speed:");
                    let mut speed = self.cycles as f32;
                    ui.add(
                        egui::Slider::new(&mut speed, 1.0..=1000.0)
                            .logarithmic(true)
                            .text("cycles")
                            .show_value(true),
                    );
                    self.cycles = speed as u64;
                    ui.small(format!("{}x speed", self.cycles));

                    ui.separator();

                    // Compact registers in 4 columns with larger font
                    ui.heading("Registers");
                    let metadata = self.emulator.metadata();
                    egui::Grid::new("registers_grid")
                        .num_columns(4)
                        .spacing([8.0, 2.0])
                        .show(ui, |ui| {
                            for (i, reg) in metadata.registers.iter().enumerate() {
                                ui.label(
                                    egui::RichText::new(format!("V{:X}:{:02X}", i, reg))
                                        .size(14.0)
                                        .monospace(),
                                );
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
                            ui.label(
                                egui::RichText::new(format!("I:{:04X}", metadata.index_register))
                                    .size(14.0)
                                    .monospace(),
                            );
                            ui.label(
                                egui::RichText::new(format!("PC:{:04X}", metadata.program_counter))
                                    .size(14.0)
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(
                                egui::RichText::new(format!("SP:{}", metadata.stack_pointer))
                                    .size(14.0)
                                    .monospace(),
                            );
                            ui.label(
                                egui::RichText::new(format!("OP:{:04X}", metadata.current_opcode))
                                    .size(14.0)
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(
                                egui::RichText::new(format!("DT:{}", metadata.delay_timer))
                                    .size(14.0)
                                    .monospace(),
                            );
                            ui.label(
                                egui::RichText::new(format!("ST:{}", metadata.sound_timer))
                                    .size(14.0)
                                    .monospace(),
                            );
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
                                let pressed = inputs[k];
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

                    // --- STACK SECTION ---
                    ui.separator();
                    ui.heading("Stack");

                    let metadata = self.emulator.metadata();
                    if metadata.stack_pointer == 0 {
                        ui.label("Empty");
                    } else {
                        // Stack visualization as a vertical list for better readability
                        ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);

                        for i in (0..metadata.stack_pointer).rev() {
                            if (i as usize) < metadata.stack.len() {
                                let color = if i == metadata.stack_pointer - 1 {
                                    egui::Color32::YELLOW
                                } else {
                                    egui::Color32::WHITE
                                };
                                ui.horizontal(|ui| {
                                    ui.colored_label(color, format!("[{}]", i));
                                    ui.colored_label(
                                        color,
                                        format!("0x{:04X}", metadata.stack[i as usize]),
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

                let metadata = self.emulator.metadata();

                // Navigation controls
                ui.horizontal_wrapped(|ui| {
                    if ui.small_button("Program Counter").clicked() {
                        self.memory_scroll_to = Some(metadata.program_counter as usize);
                    }
                    if ui.small_button("Index Register").clicked() {
                        self.memory_scroll_to = Some(metadata.index_register as usize);
                    }
                    if ui.small_button("Program Start").clicked() {
                        self.memory_scroll_to = Some(0x200); // CHIP-8 program start
                    }
                    if ui.small_button("Font Start").clicked() {
                        self.memory_scroll_to = Some(0x50); // CHIP-8 font start
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

                    for (addr, chunk) in metadata.memory.chunks(16).enumerate() {
                        let base_addr = addr * 16;

                        ui.horizontal(|ui| {
                            // Address
                            let addr_color = if base_addr == metadata.program_counter as usize {
                                egui::Color32::YELLOW
                            } else if base_addr <= metadata.index_register as usize
                                && (metadata.index_register as usize) < base_addr + 16
                            {
                                egui::Color32::LIGHT_BLUE
                            } else {
                                egui::Color32::GRAY
                            };

                            ui.colored_label(addr_color, format!("{:04X}:", base_addr));

                            // Hex bytes
                            for (i, &byte) in chunk.iter().enumerate() {
                                let byte_addr = base_addr + i;
                                let color = if byte_addr == metadata.program_counter as usize {
                                    egui::Color32::YELLOW
                                } else if byte_addr == metadata.index_register as usize {
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

        // --- BOTTOM PANEL: Instructions/Disassembly ---
        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(true)
            .default_height(150.0)
            .height_range(300.0..=400.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Instructions");
                });

                let metadata = self.emulator.metadata();

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);

                        // Show more instructions around PC for better context
                        let pc = metadata.program_counter as usize;
                        let start = pc.saturating_sub(20) & !1; // Align to even address
                        let end = (pc + 40).min(metadata.memory.len() - 1) & !1;

                        for addr in (start..end).step_by(2) {
                            if addr + 1 < metadata.memory.len() {
                                let opcode = ((metadata.memory[addr] as u16) << 8)
                                    | (metadata.memory[addr + 1] as u16);

                                ui.horizontal(|ui| {
                                    let is_current = addr == pc;
                                    let color = if is_current {
                                        egui::Color32::YELLOW
                                    } else {
                                        egui::Color32::WHITE
                                    };

                                    // Add yellowish background highlight for current instxn
                                    if is_current {
                                        ui.painter().rect_filled(
                                            ui.available_rect_before_wrap(),
                                            0.0,
                                            egui::Color32::from_rgba_unmultiplied(255, 255, 0, 30),
                                        );
                                    }

                                    ui.colored_label(
                                        color,
                                        format!("{:04X}: {:04X}", addr, opcode),
                                    );

                                    ui.separator();

                                    let instruction = match opcode & 0xF000 {
                                        0x0000 => match opcode {
                                            0x00E0 => format!("{:<20}; Clear display", "CLS"),
                                            0x00EE => {
                                                format!("{:<20}; Return from subroutine", "RET")
                                            }
                                            _ => format!(
                                                "{:<20}; Call system routine SYS {:03X}",
                                                format!("SYS {:03X}", opcode & 0x0FFF),
                                                opcode & 0x0FFF
                                            ),
                                        },
                                        0x1000 => format!(
                                            "{:<20}; Jump to address {:03X}",
                                            format!("JP {:03X}", opcode & 0x0FFF),
                                            opcode & 0x0FFF
                                        ),
                                        0x2000 => format!(
                                            "{:<20}; Call subroutine at {:03X}",
                                            format!("CALL {:03X}", opcode & 0x0FFF),
                                            opcode & 0x0FFF
                                        ),
                                        0x3000 => {
                                            let x = (opcode & 0x0F00) >> 8;
                                            let kk = opcode & 0x00FF;
                                            format!(
                                                "{:<20}; Skip if V{:X} == {:02X}",
                                                format!("SE V{:X}, {:02X}", x, kk),
                                                x,
                                                kk
                                            )
                                        }
                                        0x4000 => {
                                            let x = (opcode & 0x0F00) >> 8;
                                            let kk = opcode & 0x00FF;
                                            format!(
                                                "{:<20}; Skip if V{:X} != {:02X}",
                                                format!("SNE V{:X}, {:02X}", x, kk),
                                                x,
                                                kk
                                            )
                                        }
                                        0x5000 => {
                                            let x = (opcode & 0x0F00) >> 8;
                                            let y = (opcode & 0x00F0) >> 4;
                                            format!(
                                                "{:<20}; Skip if V{:X} == V{:X}",
                                                format!("SE V{:X}, V{:X}", x, y),
                                                x,
                                                y
                                            )
                                        }
                                        0x6000 => {
                                            let x = (opcode & 0x0F00) >> 8;
                                            let kk = opcode & 0x00FF;
                                            format!(
                                                "{:<20}; Load {:02X} into V{:X}",
                                                format!("LD V{:X}, {:02X}", x, kk),
                                                kk,
                                                x
                                            )
                                        }
                                        0x7000 => {
                                            let x = (opcode & 0x0F00) >> 8;
                                            let kk = opcode & 0x00FF;
                                            format!(
                                                "{:<20}; Add {:02X} to V{:X}",
                                                format!("ADD V{:X}, {:02X}", x, kk),
                                                kk,
                                                x
                                            )
                                        }
                                        0x8000 => {
                                            let x = (opcode & 0x0F00) >> 8;
                                            let y = (opcode & 0x00F0) >> 4;
                                            match opcode & 0x000F {
                                                0x0 => format!(
                                                    "{:<20}; V{:X} = V{:X}",
                                                    format!("LD V{:X}, V{:X}", x, y),
                                                    x,
                                                    y
                                                ),
                                                0x1 => format!(
                                                    "{:<20}; V{:X} |= V{:X}",
                                                    format!("OR V{:X}, V{:X}", x, y),
                                                    x,
                                                    y
                                                ),
                                                0x2 => format!(
                                                    "{:<20}; V{:X} &= V{:X}",
                                                    format!("AND V{:X}, V{:X}", x, y),
                                                    x,
                                                    y
                                                ),
                                                0x3 => format!(
                                                    "{:<20}; V{:X} ^= V{:X}",
                                                    format!("XOR V{:X}, V{:X}", x, y),
                                                    x,
                                                    y
                                                ),
                                                0x4 => format!(
                                                    "{:<20}; V{:X} += V{:X}, VF = carry",
                                                    format!("ADD V{:X}, V{:X}", x, y),
                                                    x,
                                                    y
                                                ),
                                                0x5 => format!(
                                                    "{:<20}; V{:X} -= V{:X}, VF = borrow",
                                                    format!("SUB V{:X}, V{:X}", x, y),
                                                    x,
                                                    y
                                                ),
                                                0x6 => format!(
                                                    "{:<20}; V{:X} >>= 1, VF = carry",
                                                    format!("SHR V{:X}", x),
                                                    x
                                                ),
                                                0x7 => format!(
                                                    "{:<20}; V{:X} = V{:X} - V{:X}, VF = borrow",
                                                    format!("SUBN V{:X}, V{:X}", x, y),
                                                    x,
                                                    y,
                                                    x
                                                ),
                                                0xE => format!(
                                                    "{:<20}; V{:X} <<= 1, VF = carry",
                                                    format!("SHL V{:X}", x),
                                                    x
                                                ),
                                                _ => format!(
                                                    "{:<20}; Unknown 8xxx instruction",
                                                    format!("8{:X}{:X}{:X}", x, y, opcode & 0x000F)
                                                ),
                                            }
                                        }
                                        0x9000 => {
                                            let x = (opcode & 0x0F00) >> 8;
                                            let y = (opcode & 0x00F0) >> 4;
                                            format!(
                                                "{:<20}; Skip if V{:X} != V{:X}",
                                                format!("SNE V{:X}, V{:X}", x, y),
                                                x,
                                                y
                                            )
                                        }
                                        0xA000 => format!(
                                            "{:<20}; I = {:03X}",
                                            format!("LD I, {:03X}", opcode & 0x0FFF),
                                            opcode & 0x0FFF
                                        ),
                                        0xB000 => format!(
                                            "{:<20}; Jump to V0 + {:03X}",
                                            format!("JP V0, {:03X}", opcode & 0x0FFF),
                                            opcode & 0x0FFF
                                        ),
                                        0xC000 => {
                                            let x = (opcode & 0x0F00) >> 8;
                                            let kk = opcode & 0x00FF;
                                            format!(
                                                "{:<20}; V{:X} = random() & {:02X}",
                                                format!("RND V{:X}, {:02X}", x, kk),
                                                x,
                                                kk
                                            )
                                        }
                                        0xD000 => {
                                            let x = (opcode & 0x0F00) >> 8;
                                            let y = (opcode & 0x00F0) >> 4;
                                            let n = opcode & 0x000F;
                                            format!(
                                                "{:<20}; Draw sprite at (V{:X}, V{:X}) height {:X}",
                                                format!("DRW V{:X}, V{:X}, {:X}", x, y, n),
                                                x,
                                                y,
                                                n
                                            )
                                        }
                                        0xE000 => {
                                            let x = (opcode & 0x0F00) >> 8;
                                            match opcode & 0x00FF {
                                                0x9E => format!(
                                                    "{:<20}; Skip if key V{:X} pressed",
                                                    format!("SKP V{:X}", x),
                                                    x
                                                ),
                                                0xA1 => format!(
                                                    "{:<20}; Skip if key V{:X} not pressed",
                                                    format!("SKNP V{:X}", x),
                                                    x
                                                ),
                                                _ => format!(
                                                    "{:<20}; Unknown Exxx instruction",
                                                    format!("E{:X}{:02X}", x, opcode & 0x00FF)
                                                ),
                                            }
                                        }
                                        0xF000 => {
                                            let x = (opcode & 0x0F00) >> 8;
                                            match opcode & 0x00FF {
                                                0x07 => format!(
                                                    "{:<20}; V{:X} = delay timer",
                                                    format!("LD V{:X}, DT", x),
                                                    x
                                                ),
                                                0x0A => format!(
                                                    "{:<20}; Wait for key, store in V{:X}",
                                                    format!("LD V{:X}, K", x),
                                                    x
                                                ),
                                                0x15 => format!(
                                                    "{:<20}; Delay timer = V{:X}",
                                                    format!("LD DT, V{:X}", x),
                                                    x
                                                ),
                                                0x18 => format!(
                                                    "{:<20}; Sound timer = V{:X}",
                                                    format!("LD ST, V{:X}", x),
                                                    x
                                                ),
                                                0x1E => format!(
                                                    "{:<20}; I += V{:X}",
                                                    format!("ADD I, V{:X}", x),
                                                    x
                                                ),
                                                0x29 => format!(
                                                    "{:<20}; I = sprite address for digit V{:X}",
                                                    format!("LD F, V{:X}", x),
                                                    x
                                                ),
                                                0x33 => format!(
                                                    "{:<20}; Store BCD of V{:X} at [I]",
                                                    format!("LD B, V{:X}", x),
                                                    x
                                                ),
                                                0x55 => format!(
                                                    "{:<20}; Store V0-V{:X} at [I]",
                                                    format!("LD [I], V{:X}", x),
                                                    x
                                                ),
                                                0x65 => format!(
                                                    "{:<20}; Load V0-V{:X} from [I]",
                                                    format!("LD V{:X}, [I]", x),
                                                    x
                                                ),
                                                _ => format!(
                                                    "{:<20}; Unknown Fxxx instruction",
                                                    format!("F{:X}{:02X}", x, opcode & 0x00FF)
                                                ),
                                            }
                                        }
                                        _ => format!(
                                            "{:<20}; Unrecognized instruction",
                                            format!("UNKNOWN {:04X}", opcode)
                                        ),
                                    };

                                    ui.colored_label(color, instruction);
                                });
                            }
                        }
                    });
            });

        // --- CENTER PANEL: Chip-8 Display ---
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

                let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(center_pos, display_size)));
                child_ui.image((tex.id(), display_size));
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Loading display...");
                });
            }
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    env_logger::init();
    let cli = Cli::parse();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0]) // Larger default for more info panels
            .with_min_inner_size([1000.0, 700.0])
            .with_resizable(true),
        ..Default::default()
    };

    log::info!("Starting emulator");
    eframe::run_native(
        "Multi-Emulator",
        options,
        Box::new(|_cc| Ok(Box::new(App::new(cli.cycles, cli.mute)))),
    )
}
