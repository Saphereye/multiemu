use clap::Parser;
use eframe::egui;
use std::time::{Duration, Instant};

mod emulators;

use emulators::chip8::Chip8Emulator;
use emulators::gameboy::GameBoyEmulator;
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

enum EmulatorType {
    Chip8(Box<Chip8Emulator>),
    GameBoy(Box<GameBoyEmulator>),
}

impl EmulatorType {
    fn system_name(&self) -> &'static str {
        match self {
            EmulatorType::Chip8(emu) => emu.system_name(),
            EmulatorType::GameBoy(emu) => emu.system_name(),
        }
    }

    fn load_rom(&mut self, path: &std::path::Path) -> Result<(), emulators::EmuError> {
        match self {
            EmulatorType::Chip8(emu) => emu.load_rom(path),
            EmulatorType::GameBoy(emu) => emu.load_rom(path),
        }
    }

    fn reset(&mut self) {
        match self {
            EmulatorType::Chip8(emu) => emu.reset(),
            EmulatorType::GameBoy(emu) => emu.reset(),
        }
    }

    fn step(&mut self) -> Result<(), emulators::EmuError> {
        match self {
            EmulatorType::Chip8(emu) => emu.step(),
            EmulatorType::GameBoy(emu) => emu.step(),
        }
    }

    fn update_timers(&mut self, delta: Duration) {
        match self {
            EmulatorType::Chip8(emu) => emu.update_timers(delta),
            EmulatorType::GameBoy(emu) => emu.update_timers(delta),
        }
    }

    fn framebuffer(&self) -> &[u32] {
        match self {
            EmulatorType::Chip8(emu) => emu.framebuffer(),
            EmulatorType::GameBoy(emu) => emu.framebuffer(),
        }
    }

    fn resolution(&self) -> (usize, usize) {
        match self {
            EmulatorType::Chip8(emu) => emu.resolution(),
            EmulatorType::GameBoy(emu) => emu.resolution(),
        }
    }

    fn set_input_state(&mut self, inputs: &[bool]) {
        match self {
            EmulatorType::Chip8(emu) => emu.set_input_state(inputs),
            EmulatorType::GameBoy(emu) => emu.set_input_state(inputs),
        }
    }

    fn keymap(&self) -> Vec<(usize, String)> {
        match self {
            EmulatorType::Chip8(emu) => emu.keymap(),
            EmulatorType::GameBoy(emu) => emu.keymap(),
        }
    }
}

pub struct App {
    emulator: EmulatorType,
    cycles: u64,
    speed_multiplier: f32, // Speed multiplier for emulation (0.1x to 10x)
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
            emulator: EmulatorType::Chip8(Box::new(emulator)),
            cycles,
            speed_multiplier: 1.0, // Default 1x speed
            texture: None,
            last_timer_update: Instant::now(),
            timer_period: Duration::from_nanos(16_666_667), // ~60Hz
            memory_scroll_to: None,
            is_paused: true,
            selected_emulator: "CHIP-8".to_string(),
            rom_path: None,
        }
    }

    fn switch_emulator(&mut self, emulator_name: &str) {
        match emulator_name {
            "CHIP-8" => {
                let mut emu = Chip8Emulator::new();
                emu.set_mute(false); // TODO: preserve mute state
                self.emulator = EmulatorType::Chip8(Box::new(emu));
            }
            "Game Boy" => {
                let emu = GameBoyEmulator::new();
                self.emulator = EmulatorType::GameBoy(Box::new(emu));
            }
            _ => {}
        }
        self.texture = None; // Clear texture when switching
        self.rom_path = None;
        self.is_paused = true;
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
            for (key_index, key_str) in &keymap {
                let key = match key_str.as_str() {
                    // CHIP-8 keys
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
                    // Game Boy keys
                    "Up" => egui::Key::ArrowUp,
                    "Down" => egui::Key::ArrowDown,
                    "Left" => egui::Key::ArrowLeft,
                    "Right" => egui::Key::ArrowRight,
                    "Return" => egui::Key::Enter,
                    "RShift" => {
                        // Check for shift modifier
                        inputs[*key_index] = i.modifiers.shift;
                        continue;
                    },
                    _ => continue,
                };
                inputs[*key_index] = i.key_down(key);
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
            // Calculate actual cycles to execute based on speed multiplier
            let cycles_to_execute = match &self.emulator {
                EmulatorType::Chip8(_) => self.cycles, // CHIP-8 uses cycles directly
                EmulatorType::GameBoy(_) => {
                    // Game Boy: ~70224 cycles per frame at 60Hz (~4.19 MHz)
                    // Apply speed multiplier
                    (70224.0 * self.speed_multiplier) as u64
                }
            };
            
            for _ in 0..cycles_to_execute {
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
                    let current_emulator = self.selected_emulator.clone();
                    egui::ComboBox::from_label("")
                        .selected_text(&self.selected_emulator)
                        .show_ui(ui, |ui| {
                            if ui.selectable_value(
                                &mut self.selected_emulator,
                                "CHIP-8".to_string(),
                                "CHIP-8",
                            ).clicked() && current_emulator != "CHIP-8" {
                                self.switch_emulator("CHIP-8");
                            }
                            if ui.selectable_value(
                                &mut self.selected_emulator,
                                "Game Boy".to_string(),
                                "Game Boy",
                            ).clicked() && current_emulator != "Game Boy" {
                                self.switch_emulator("Game Boy");
                            }
                        });

                    ui.separator();

                    // ROM file selector
                    ui.heading("ROM File");
                    if ui.button("📁 Load ROM").clicked() {
                        let extensions = match self.selected_emulator.as_str() {
                            "CHIP-8" => vec!["ch8", "rom"],
                            "Game Boy" => vec!["gb", "gbc", "rom"],
                            _ => vec!["rom"],
                        };
                        
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("ROM files", &extensions)
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
                    match &self.emulator {
                        EmulatorType::Chip8(_) => {
                            // CHIP-8: cycles per timer update
                            let mut speed = self.cycles as f32;
                            ui.add(
                                egui::Slider::new(&mut speed, 1.0..=1000.0)
                                    .logarithmic(true)
                                    .text("cycles")
                                    .show_value(true),
                            );
                            self.cycles = speed as u64;
                            ui.small(format!("{}x speed", self.cycles));
                        }
                        EmulatorType::GameBoy(_) => {
                            // Game Boy: speed multiplier (0.1x to 10x)
                            ui.add(
                                egui::Slider::new(&mut self.speed_multiplier, 0.1..=10.0)
                                    .logarithmic(true)
                                    .text("speed")
                                    .show_value(true),
                            );
                            ui.small(format!("{:.1}x speed", self.speed_multiplier));
                        }
                    }

                    ui.separator();

                    // Compact registers in 4 columns with larger font
                    ui.heading("Registers");
                    match &self.emulator {
                        EmulatorType::Chip8(emu) => {
                            let metadata = emu.metadata();
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
                        }
                        EmulatorType::GameBoy(emu) => {
                            let metadata = emu.metadata();
                            egui::Grid::new("registers_grid")
                                .num_columns(2)
                                .spacing([8.0, 2.0])
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new(format!("A:{:02X}", metadata.registers[0]))
                                            .size(14.0)
                                            .monospace(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("F:{:02X}", metadata.registers[1]))
                                            .size(14.0)
                                            .monospace(),
                                    );
                                    ui.end_row();
                                    ui.label(
                                        egui::RichText::new(format!("B:{:02X}", metadata.registers[2]))
                                            .size(14.0)
                                            .monospace(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("C:{:02X}", metadata.registers[3]))
                                            .size(14.0)
                                            .monospace(),
                                    );
                                    ui.end_row();
                                    ui.label(
                                        egui::RichText::new(format!("D:{:02X}", metadata.registers[4]))
                                            .size(14.0)
                                            .monospace(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("E:{:02X}", metadata.registers[5]))
                                            .size(14.0)
                                            .monospace(),
                                    );
                                    ui.end_row();
                                    ui.label(
                                        egui::RichText::new(format!("H:{:02X}", metadata.registers[6]))
                                            .size(14.0)
                                            .monospace(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("L:{:02X}", metadata.registers[7]))
                                            .size(14.0)
                                            .monospace(),
                                    );
                                    ui.end_row();
                                    ui.label(
                                        egui::RichText::new(format!("SP:{:04X}", metadata.sp))
                                            .size(14.0)
                                            .monospace(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("PC:{:04X}", metadata.pc))
                                            .size(14.0)
                                            .monospace(),
                                    );
                                    ui.end_row();
                                    ui.label(
                                        egui::RichText::new(format!("OP:{:02X}", metadata.current_opcode))
                                            .size(14.0)
                                            .monospace(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("IME:{}", if metadata.ime { "1" } else { "0" }))
                                            .size(14.0)
                                            .monospace(),
                                    );
                                });
                        }
                    }

                    ui.separator();
                    ui.heading("Keys");

                    // Display keys based on emulator type
                    match &self.selected_emulator[..] {
                        "CHIP-8" => {
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
                        }
                        "Game Boy" => {
                            // Game Boy button layout
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label("D-Pad:");
                                    ui.horizontal(|ui| {
                                        ui.add_space(30.0);
                                        let up_pressed = inputs[4];
                                        let btn = egui::Button::new("↑").min_size(egui::vec2(25.0, 25.0));
                                        if up_pressed {
                                            ui.add(btn.fill(egui::Color32::GREEN));
                                        } else {
                                            ui.add(btn);
                                        }
                                    });
                                    ui.horizontal(|ui| {
                                        let left_pressed = inputs[6];
                                        let btn = egui::Button::new("←").min_size(egui::vec2(25.0, 25.0));
                                        if left_pressed {
                                            ui.add(btn.fill(egui::Color32::GREEN));
                                        } else {
                                            ui.add(btn);
                                        }
                                        
                                        let down_pressed = inputs[5];
                                        let btn = egui::Button::new("↓").min_size(egui::vec2(25.0, 25.0));
                                        if down_pressed {
                                            ui.add(btn.fill(egui::Color32::GREEN));
                                        } else {
                                            ui.add(btn);
                                        }
                                        
                                        let right_pressed = inputs[7];
                                        let btn = egui::Button::new("→").min_size(egui::vec2(25.0, 25.0));
                                        if right_pressed {
                                            ui.add(btn.fill(egui::Color32::GREEN));
                                        } else {
                                            ui.add(btn);
                                        }
                                    });
                                });
                                
                                ui.add_space(10.0);
                                
                                ui.vertical(|ui| {
                                    ui.label("Buttons:");
                                    ui.horizontal(|ui| {
                                        let a_pressed = inputs[0];
                                        let btn = egui::Button::new("A").min_size(egui::vec2(30.0, 30.0));
                                        if a_pressed {
                                            ui.add(btn.fill(egui::Color32::GREEN));
                                        } else {
                                            ui.add(btn);
                                        }
                                        
                                        let b_pressed = inputs[1];
                                        let btn = egui::Button::new("B").min_size(egui::vec2(30.0, 30.0));
                                        if b_pressed {
                                            ui.add(btn.fill(egui::Color32::GREEN));
                                        } else {
                                            ui.add(btn);
                                        }
                                    });
                                    ui.horizontal(|ui| {
                                        let select_pressed = inputs[3];
                                        let btn = egui::Button::new("Select").min_size(egui::vec2(45.0, 20.0));
                                        if select_pressed {
                                            ui.add(btn.fill(egui::Color32::GREEN));
                                        } else {
                                            ui.add(btn);
                                        }
                                        
                                        let start_pressed = inputs[2];
                                        let btn = egui::Button::new("Start").min_size(egui::vec2(45.0, 20.0));
                                        if start_pressed {
                                            ui.add(btn.fill(egui::Color32::GREEN));
                                        } else {
                                            ui.add(btn);
                                        }
                                    });
                                });
                            });
                        }
                        _ => {}
                    }

                    // --- STACK SECTION (CHIP-8 only) ---
                    if let EmulatorType::Chip8(emu) = &self.emulator {
                        ui.separator();
                        ui.heading("Stack");

                        let metadata = emu.metadata();
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

                // Navigation controls based on emulator type
                match &self.emulator {
                    EmulatorType::Chip8(emu) => {
                        let metadata = emu.metadata();
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
                    }
                    EmulatorType::GameBoy(emu) => {
                        let metadata = emu.metadata();
                        ui.horizontal_wrapped(|ui| {
                            if ui.small_button("Program Counter").clicked() {
                                self.memory_scroll_to = Some(metadata.pc as usize);
                            }
                            if ui.small_button("Stack Pointer").clicked() {
                                self.memory_scroll_to = Some(metadata.sp as usize);
                            }
                            if ui.small_button("ROM Start").clicked() {
                                self.memory_scroll_to = Some(0x0000);
                            }
                            if ui.small_button("Entry Point").clicked() {
                                self.memory_scroll_to = Some(0x0100);
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
                                    let addr_color = if base_addr == metadata.pc as usize {
                                        egui::Color32::YELLOW
                                    } else if base_addr <= metadata.sp as usize
                                        && (metadata.sp as usize) < base_addr + 16
                                    {
                                        egui::Color32::LIGHT_BLUE
                                    } else {
                                        egui::Color32::GRAY
                                    };

                                    ui.colored_label(addr_color, format!("{:04X}:", base_addr));

                                    // Hex bytes
                                    for (i, &byte) in chunk.iter().enumerate() {
                                        let byte_addr = base_addr + i;
                                        let color = if byte_addr == metadata.pc as usize {
                                            egui::Color32::YELLOW
                                        } else if byte_addr == metadata.sp as usize {
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
                    }
                }
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

                match &self.emulator {
                    EmulatorType::Chip8(emu) => {
                        let metadata = emu.metadata();

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
                    }
                    EmulatorType::GameBoy(emu) => {
                        let metadata = emu.metadata();

                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);

                                // Show instructions around PC
                                let pc = metadata.pc as usize;
                                let start = pc.saturating_sub(20);
                                let end = (pc + 40).min(metadata.memory.len());

                                for addr in start..end {
                                    if addr < metadata.memory.len() {
                                        let opcode = metadata.memory[addr];

                                        ui.horizontal(|ui| {
                                            let is_current = addr == pc;
                                            let color = if is_current {
                                                egui::Color32::YELLOW
                                            } else {
                                                egui::Color32::WHITE
                                            };

                                            if is_current {
                                                ui.painter().rect_filled(
                                                    ui.available_rect_before_wrap(),
                                                    0.0,
                                                    egui::Color32::from_rgba_unmultiplied(255, 255, 0, 30),
                                                );
                                            }

                                            ui.colored_label(
                                                color,
                                                format!("{:04X}: {:02X}", addr, opcode),
                                            );

                                            ui.separator();

                                            // Comprehensive Game Boy disassembly
                                            let instruction = match opcode {
                                                0x00 => "NOP".to_string(),
                                                0x01 => "LD BC, d16".to_string(),
                                                0x02 => "LD (BC), A".to_string(),
                                                0x03 => "INC BC".to_string(),
                                                0x04 => "INC B".to_string(),
                                                0x05 => "DEC B".to_string(),
                                                0x06 => "LD B, d8".to_string(),
                                                0x07 => "RLCA".to_string(),
                                                0x08 => "LD (a16), SP".to_string(),
                                                0x09 => "ADD HL, BC".to_string(),
                                                0x0A => "LD A, (BC)".to_string(),
                                                0x0B => "DEC BC".to_string(),
                                                0x0C => "INC C".to_string(),
                                                0x0D => "DEC C".to_string(),
                                                0x0E => "LD C, d8".to_string(),
                                                0x0F => "RRCA".to_string(),
                                                0x10 => "STOP".to_string(),
                                                0x11 => "LD DE, d16".to_string(),
                                                0x12 => "LD (DE), A".to_string(),
                                                0x13 => "INC DE".to_string(),
                                                0x14 => "INC D".to_string(),
                                                0x15 => "DEC D".to_string(),
                                                0x16 => "LD D, d8".to_string(),
                                                0x17 => "RLA".to_string(),
                                                0x18 => "JR r8".to_string(),
                                                0x19 => "ADD HL, DE".to_string(),
                                                0x1A => "LD A, (DE)".to_string(),
                                                0x1B => "DEC DE".to_string(),
                                                0x1C => "INC E".to_string(),
                                                0x1D => "DEC E".to_string(),
                                                0x1E => "LD E, d8".to_string(),
                                                0x1F => "RRA".to_string(),
                                                0x20 => "JR NZ, r8".to_string(),
                                                0x21 => "LD HL, d16".to_string(),
                                                0x22 => "LD (HL+), A".to_string(),
                                                0x23 => "INC HL".to_string(),
                                                0x24 => "INC H".to_string(),
                                                0x25 => "DEC H".to_string(),
                                                0x26 => "LD H, d8".to_string(),
                                                0x27 => "DAA".to_string(),
                                                0x28 => "JR Z, r8".to_string(),
                                                0x29 => "ADD HL, HL".to_string(),
                                                0x2A => "LD A, (HL+)".to_string(),
                                                0x2B => "DEC HL".to_string(),
                                                0x2C => "INC L".to_string(),
                                                0x2D => "DEC L".to_string(),
                                                0x2E => "LD L, d8".to_string(),
                                                0x2F => "CPL".to_string(),
                                                0x30 => "JR NC, r8".to_string(),
                                                0x31 => "LD SP, d16".to_string(),
                                                0x32 => "LD (HL-), A".to_string(),
                                                0x33 => "INC SP".to_string(),
                                                0x34 => "INC (HL)".to_string(),
                                                0x35 => "DEC (HL)".to_string(),
                                                0x36 => "LD (HL), d8".to_string(),
                                                0x37 => "SCF".to_string(),
                                                0x38 => "JR C, r8".to_string(),
                                                0x39 => "ADD HL, SP".to_string(),
                                                0x3A => "LD A, (HL-)".to_string(),
                                                0x3B => "DEC SP".to_string(),
                                                0x3C => "INC A".to_string(),
                                                0x3D => "DEC A".to_string(),
                                                0x3E => "LD A, d8".to_string(),
                                                0x3F => "CCF".to_string(),
                                                0x76 => "HALT".to_string(),
                                                0xC0 => "RET NZ".to_string(),
                                                0xC1 => "POP BC".to_string(),
                                                0xC2 => "JP NZ, a16".to_string(),
                                                0xC3 => "JP a16".to_string(),
                                                0xC4 => "CALL NZ, a16".to_string(),
                                                0xC5 => "PUSH BC".to_string(),
                                                0xC6 => "ADD A, d8".to_string(),
                                                0xC7 => "RST 00H".to_string(),
                                                0xC8 => "RET Z".to_string(),
                                                0xC9 => "RET".to_string(),
                                                0xCA => "JP Z, a16".to_string(),
                                                0xCB => "PREFIX CB".to_string(),
                                                0xCC => "CALL Z, a16".to_string(),
                                                0xCD => "CALL a16".to_string(),
                                                0xCE => "ADC A, d8".to_string(),
                                                0xCF => "RST 08H".to_string(),
                                                0xD0 => "RET NC".to_string(),
                                                0xD1 => "POP DE".to_string(),
                                                0xD2 => "JP NC, a16".to_string(),
                                                0xD4 => "CALL NC, a16".to_string(),
                                                0xD5 => "PUSH DE".to_string(),
                                                0xD6 => "SUB d8".to_string(),
                                                0xD7 => "RST 10H".to_string(),
                                                0xD8 => "RET C".to_string(),
                                                0xD9 => "RETI".to_string(),
                                                0xDA => "JP C, a16".to_string(),
                                                0xDC => "CALL C, a16".to_string(),
                                                0xDE => "SBC A, d8".to_string(),
                                                0xDF => "RST 18H".to_string(),
                                                0xE0 => "LDH (a8), A".to_string(),
                                                0xE1 => "POP HL".to_string(),
                                                0xE2 => "LD (C), A".to_string(),
                                                0xE5 => "PUSH HL".to_string(),
                                                0xE6 => "AND d8".to_string(),
                                                0xE7 => "RST 20H".to_string(),
                                                0xE8 => "ADD SP, r8".to_string(),
                                                0xE9 => "JP (HL)".to_string(),
                                                0xEA => "LD (a16), A".to_string(),
                                                0xEE => "XOR d8".to_string(),
                                                0xEF => "RST 28H".to_string(),
                                                0xF0 => "LDH A, (a8)".to_string(),
                                                0xF1 => "POP AF".to_string(),
                                                0xF2 => "LD A, (C)".to_string(),
                                                0xF3 => "DI".to_string(),
                                                0xF5 => "PUSH AF".to_string(),
                                                0xF6 => "OR d8".to_string(),
                                                0xF7 => "RST 30H".to_string(),
                                                0xF8 => "LD HL, SP+r8".to_string(),
                                                0xF9 => "LD SP, HL".to_string(),
                                                0xFA => "LD A, (a16)".to_string(),
                                                0xFB => "EI".to_string(),
                                                0xFE => "CP d8".to_string(),
                                                0xFF => "RST 38H".to_string(),
                                                0x40..=0x75 | 0x77..=0x7F => {
                                                    let to = (opcode >> 3) & 0x07;
                                                    let from = opcode & 0x07;
                                                    format!("LD {}, {}", 
                                                        ["B", "C", "D", "E", "H", "L", "(HL)", "A"][to as usize],
                                                        ["B", "C", "D", "E", "H", "L", "(HL)", "A"][from as usize])
                                                }
                                                0x80..=0x87 => {
                                                    let reg = opcode & 0x07;
                                                    format!("ADD A, {}", ["B", "C", "D", "E", "H", "L", "(HL)", "A"][reg as usize])
                                                }
                                                0x88..=0x8F => {
                                                    let reg = opcode & 0x07;
                                                    format!("ADC A, {}", ["B", "C", "D", "E", "H", "L", "(HL)", "A"][reg as usize])
                                                }
                                                0x90..=0x97 => {
                                                    let reg = opcode & 0x07;
                                                    format!("SUB {}", ["B", "C", "D", "E", "H", "L", "(HL)", "A"][reg as usize])
                                                }
                                                0x98..=0x9F => {
                                                    let reg = opcode & 0x07;
                                                    format!("SBC A, {}", ["B", "C", "D", "E", "H", "L", "(HL)", "A"][reg as usize])
                                                }
                                                0xA0..=0xA7 => {
                                                    let reg = opcode & 0x07;
                                                    format!("AND {}", ["B", "C", "D", "E", "H", "L", "(HL)", "A"][reg as usize])
                                                }
                                                0xA8..=0xAF => {
                                                    let reg = opcode & 0x07;
                                                    format!("XOR {}", ["B", "C", "D", "E", "H", "L", "(HL)", "A"][reg as usize])
                                                }
                                                0xB0..=0xB7 => {
                                                    let reg = opcode & 0x07;
                                                    format!("OR {}", ["B", "C", "D", "E", "H", "L", "(HL)", "A"][reg as usize])
                                                }
                                                0xB8..=0xBF => {
                                                    let reg = opcode & 0x07;
                                                    format!("CP {}", ["B", "C", "D", "E", "H", "L", "(HL)", "A"][reg as usize])
                                                }
                                                _ => format!("??? (0x{:02X})", opcode),
                                            };

                                            ui.colored_label(color, instruction);
                                        });
                                    }
                                }
                            });
                    }
                }
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
