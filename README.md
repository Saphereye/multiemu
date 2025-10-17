# Multi-Emulator Platform

A modular emulator platform built with Rust and egui, currently supporting CHIP-8 and Game Boy.

## Features

- **Modular Architecture**: Trait-based design for easy addition of new emulators
- **GUI Interface**: Clean UI with file picker, controls, and debugger
- **Multiple Emulators**:
  - ✅ CHIP-8 (fully implemented)
  - ✅ Game Boy (CPU implemented with accurate cycle timing)

## How to Run

```bash
cargo run --release
```

The application will open with a GUI where you can:
1. Select an emulator from the dropdown (CHIP-8 or Game Boy)
2. Load a ROM file using the file picker
3. Click the Play button (▶) or press Space to start emulation
4. Use the controls to pause/resume/reset the emulator

### Quick Start with Test ROMs

For Game Boy testing, a simple test ROM is provided:

```bash
# The test ROM is located in test_roms/simple_test.gb
# Load it in the UI after selecting "Game Boy" from the dropdown
```

This test ROM fills VRAM with a pattern and loops continuously - perfect for verifying the emulator works.

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
│   └── gameboy/        # Game Boy implementation
│       ├── mod.rs
│       └── opcodes.rs  # Cycle timing data
└── main.rs             # GUI application
```

### Adding New Emulators

1. Create a new module in `src/emulators/`
2. Implement the `Emulator` trait
3. Add to the emulator dropdown in `main.rs`

## Game Boy Implementation

The Game Boy emulator implements the Sharp LR35902 CPU (modified Z80) with:

- **Complete instruction set**: All 256 base opcodes + CB-prefixed instructions
- **Accurate cycle timing**: Each instruction returns the correct number of M-cycles based on the Pan Docs specification
- **CPU features**: 
  - 8-bit registers: A, F (flags), B, C, D, E, H, L
  - 16-bit register pairs: AF, BC, DE, HL
  - Stack pointer (SP), Program counter (PC)
  - Flag register with Z (Zero), N (Subtract), H (Half-carry), C (Carry) flags
- **Memory management**: 64KB address space with proper memory-mapped I/O
- **Instruction categories**:
  - Load/Store: All LD variants
  - Arithmetic: ADD, ADC, SUB, SBC, INC, DEC
  - Logic: AND, OR, XOR, CP
  - Rotate/Shift: RLCA, RRCA, RLA, RRA, and CB-prefixed RLC, RRC, RL, RR, SLA, SRA, SRL, SWAP
  - Bit operations: BIT, RES, SET (CB-prefixed)
  - Control flow: JP, JR, CALL, RET, RST, RETI
  - Stack: PUSH, POP
  - Misc: DAA, CPL, SCF, CCF, HALT, STOP, DI, EI

### Cycle Timing

All instructions use accurate M-cycle counts as documented in the Pan Docs:
- Simple operations: 4 cycles
- 8-bit immediate loads: 8 cycles
- 16-bit immediate loads: 12 cycles
- Memory operations: 8-16 cycles depending on addressing mode
- Conditional jumps/calls: Different cycles for taken vs not taken branches
- CB-prefixed instructions: 8 cycles for register operations, 12-16 for (HL) operations

## Resources

### CHIP-8
- [CHIP-8 Specification](https://www.cs.columbia.edu/~sedwards/classes/2016/4840-spring/designs/Chip8.pdf)
- [CHIP-8 Test Suite](https://github.com/Timendus/chip8-test-suite)
- [CHIP-8 ROMs](https://github.com/dmatlack/chip8/tree/master/roms/games)

### Game Boy
- [Pan Docs - Game Boy Technical Reference](https://gekkio.fi/files/gb-docs/gbctr.pdf) - Complete hardware specification including CPU instruction set and cycle timings
- [Game Boy CPU Manual](http://marc.rawer.de/Gameboy/Docs/GBCPUman.pdf) - Detailed CPU instruction reference
- [Game Boy Development Resources](https://gbdev.io/) - Community resources and development tools
- [Game Boy Homebrew Hub](https://gbhh.avivace.com/) - Free homebrew ROMs for testing
- [Blargg's Test ROMs](https://github.com/retrio/gb-test-roms) - Comprehensive CPU test suite

### Controls

**CHIP-8:**
- Keys 1-4, Q-R, A-F, Z-C, V map to the 16-key hex keypad

**Game Boy:**
- Arrow Keys: D-pad (Up, Down, Left, Right)
- X: A button
- Z: B button  
- Enter: Start
- Shift: Select

**Common:**
- Space: Play/Pause emulation
- R: Reset emulation
- Speed slider: Adjust emulation speed (CHIP-8: 1-1000 cycles, Game Boy: 0.1x-10x)

## Troubleshooting

### Game Boy emulator stuck on green/yellow screen

If the Game Boy emulator appears frozen:

1. **Make sure you pressed Play** - The emulator starts paused after loading a ROM
2. **Try the test ROM** - Use `test_roms/simple_test.gb` to verify basic functionality
3. **Check console logs** - Look for ROM loading messages and any error output
4. **Verify ROM compatibility** - Currently supports ROMs up to 32KB without Memory Bank Controllers
5. **Some games may hit HALT** - If a game uses HALT without proper interrupt handling, it may appear stuck. Press any button to wake from HALT.

### Where to get ROMs

- **Test ROM included**: `test_roms/simple_test.gb` - Minimal ROM for testing
- **CHIP-8 ROMs**: Available at https://github.com/dmatlack/chip8/tree/master/roms/games
- **Game Boy Homebrew**: Free games at https://gbhh.avivace.com/
- **Commercial ROMs**: Only use if you own the original cartridge

