# Game Boy Test ROMs

This directory contains test ROMs for debugging the Game Boy emulator.

## simple_test.gb

A minimal Game Boy ROM that:
1. Disables interrupts (DI)
2. Sets up the stack pointer at 0xFFFE
3. Fills VRAM at 0x8000 with pattern 0xAA
4. Enters an infinite loop with NOPs

This ROM should display a pattern on screen and never halt.

## Usage

1. Build and run the emulator:
   ```bash
   cargo run --release
   ```

2. In the UI:
   - Select "Game Boy" from the emulator dropdown
   - Click "Load ROM" and select `test_roms/simple_test.gb`
   - Click the Play button (or press Space)

3. Expected behavior:
   - The screen should show a checkered pattern (alternating light/dark green)
   - The emulator should run continuously without freezing
   - You can adjust speed with the slider (0.1x - 10x)

## Debugging

If the emulator appears stuck:

1. **Check the console logs** - The ROM loading should show:
   ```
   Loaded Game Boy ROM: 32768 bytes
   PC initialized to: 0x0100
   First 16 bytes at PC: ...
   ```

2. **Check register display** - After loading, you should see:
   - PC starting at 0x0100, then jumping to 0x0150
   - SP set to 0xFFFE
   - HL incrementing as it fills VRAM

3. **Common issues**:
   - If stuck at yellow/green screen: The ROM might be hitting a HALT instruction
   - If frozen: Check that the emulator is not paused (Play button should show "Pause")
   - If nothing happens: Make sure you pressed Play after loading the ROM

## Controls

- **Arrow Keys**: D-pad (Up, Down, Left, Right)
- **X**: A button
- **Z**: B button
- **Enter**: Start
- **Shift**: Select
- **Space**: Play/Pause emulation
- **R**: Reset emulation

## Where to Find Real Game Boy ROMs

For testing with actual games, you can use:
- **Homebrew ROMs**: Free Game Boy homebrew from https://gbhh.avivace.com/
- **Test ROMs**: Blargg's test ROMs from https://github.com/retrio/gb-test-roms
- **Your own backups**: If you own the original cartridges

Note: Commercial ROM files are copyrighted and should only be used if you own the original game.
