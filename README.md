# Chip-8 emulator
This project is an implementation of an emulator for the [CHIP-8](https://en.wikipedia.org/wiki/CHIP-8) [fantasy video game console](https://en.wikipedia.org/wiki/Fantasy_video_game_console).

The project is structured to support multiple emulators in the future through a trait-based architecture.

## Architecture

The emulator is built with a modular architecture:

- **`emulators/`** - Module containing all emulator implementations
  - **`mod.rs`** - Defines the `Emulator` trait and `EmuError` enum
  - **`chip8.rs`** - CHIP-8 emulator implementation

### Emulator Trait

All emulators implement the `Emulator` trait which provides:
- ROM loading (`load_rom`)
- Execution control (`step`, `reset`)
- Timer/audio updates (`update_timers`)
- Display output (`framebuffer`, `resolution`)
- Input handling (`set_input_state`)
- System metadata access (`metadata`)

This design makes it easy to add support for other systems (NES, Game Boy, etc.) in the future.

## How to run
The program can be run by cloning the repo and creating the executable using:
```bash
cargo run --release

```
This will create the executable at ./target/release/chip-8-emulator.
Use `./chip-8-emulator --help` to know more about the arguments that can be provided.



## Screenshot
![image](./assets/screenshot.png)



More roms can be found [here](https://github.com/dmatlack/chip8/tree/master/roms/games)

## Resources
- [CHIP 8 Specification](https://www.cs.columbia.edu/~sedwards/classes/2016/4840-spring/designs/Chip8.pdf)
- [Chip 8 test suite](https://github.com/Timendus/chip8-test-suite)
