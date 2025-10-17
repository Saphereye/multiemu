# Multi-Emulator Platform

A modular emulator platform built with Rust and egui, currently supporting CHIP-8 with plans for more systems.

## Features

- **Modular Architecture**: Trait-based design for easy addition of new emulators
- **GUI Interface**: Clean UI with file picker, controls, and debugger
- **Multiple Emulators**:
  - ✅ CHIP-8 (fully implemented)
  - 🚧 Game Boy (skeleton, in development)

## How to Run

```bash
cargo run --release
```

The application will open with a GUI where you can:
1. Select an emulator from the dropdown
2. Load a ROM file using the file picker
3. Use the controls to run/pause/reset the emulator

### Command-line Options

```bash
cargo run --release -- [OPTIONS]

Options:
  -c, --cycles <CYCLES>  Number of CPU instructions per timer update [default: 1]
  -m, --mute             Enable to mute the beep sound
  -h, --help             Print help
```

## Architecture

The project uses a trait-based architecture for modularity:

```
src/
├── emulators/
│   ├── mod.rs          # Emulator trait and error types
│   ├── chip8/          # CHIP-8 implementation
│   │   ├── mod.rs
│   │   ├── configs.rs
│   │   └── rand.rs
│   └── gameboy/        # Game Boy implementation (skeleton)
│       └── mod.rs
└── main.rs             # GUI application
```

### Adding New Emulators

1. Create a new module in `src/emulators/`
2. Implement the `Emulator` trait
3. Add to the emulator dropdown in `main.rs`

## Resources

- [CHIP-8 Specification](https://www.cs.columbia.edu/~sedwards/classes/2016/4840-spring/designs/Chip8.pdf)
- [CHIP-8 Test Suite](https://github.com/Timendus/chip8-test-suite)
- [CHIP-8 ROMs](https://github.com/dmatlack/chip8/tree/master/roms/games)

