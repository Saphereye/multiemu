# UI Features

The emulator provides a comprehensive debugging and visualization interface.

## Main Window Layout

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         CHIP-8 Emulator                                  │
├──────────────┬───────────────────────────────────────┬──────────────────┤
│              │                                       │                  │
│  EMULATOR    │                                       │    MEMORY        │
│  ┌────────┐  │                                       │                  │
│  │ CHIP-8 │  │         EMULATOR DISPLAY             │  0200: 00 E0 ... │
│  └────────┘  │                                       │  0202: 60 00 ... │
│              │        [64x32 pixels]                 │  0204: 61 00 ... │
│  CONTROLS    │                                       │  ...             │
│  [▶ Run  ]   │                                       │                  │
│  [⏹ Reset]   │                                       │  Navigation:     │
│              │                                       │  [PC] [I] [0x200]│
│  Speed: 10x  │                                       │                  │
│  ────────    │                                       │                  │
│              │                                       │                  │
│  REGISTERS   │                                       │                  │
│  V0:00 V1:00 │                                       │                  │
│  V2:00 V3:00 │                                       │                  │
│  ...         │                                       │                  │
│  I:0210      │                                       │                  │
│  PC:0200     │                                       │                  │
│              │                                       │                  │
│  KEYS        │                                       │                  │
│  [1][2][3][C]│                                       │                  │
│  [4][5][6][D]│                                       │                  │
│  [7][8][9][E]│                                       │                  │
│  [A][0][B][F]│                                       │                  │
│              │                                       │                  │
│  STACK       │                                       │                  │
│  [0] 0x0200  │                                       │                  │
│              │                                       │                  │
├──────────────┴───────────────────────────────────────┴──────────────────┤
│                        INSTRUCTIONS                                      │
│                                                                          │
│  0200: 00E0    CLS               ; Clear display                        │
│  0202: 6000    LD V0, 00         ; Load 0 into V0                       │
│  0204: 6100    LD V1, 00         ; Load 0 into V1                       │
│  ...                                                                     │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

## Key Features

### 1. Emulator Selector (NEW!)
- Dropdown menu to select between different emulators
- Currently supports CHIP-8
- Designed for easy addition of future emulators (NES, Game Boy, etc.)

### 2. Control Panel
- **Play/Pause Button**: Start or pause emulation
- **Reset Button**: Reset emulator to initial state
- **Speed Slider**: Adjust execution speed (1x to 1000x)

### 3. Register Display
- All 16 general-purpose registers (V0-VF)
- Index register (I)
- Program counter (PC)
- Stack pointer (SP)
- Current opcode (OP)
- Delay timer (DT)
- Sound timer (ST)

### 4. Keypad Visualizer
- Shows all 16 CHIP-8 keys (0-F)
- Highlights pressed keys in green
- Visual feedback for input state

### 5. Stack Viewer
- Shows current stack contents
- Highlights top of stack
- Shows stack depth

### 6. Memory Viewer
- Hexdump view of all 4KB memory
- Color-coded highlighting:
  - Yellow: Current program counter
  - Light Blue: Index register location
  - White: Non-zero bytes
  - Dark Gray: Zero bytes
- Quick navigation buttons to jump to:
  - Program Counter (PC)
  - Index Register (I)
  - Program Start (0x200)
  - Font Start (0x50)

### 7. Instruction Disassembler
- Real-time disassembly of CHIP-8 instructions
- Shows surrounding instructions for context
- Highlights current instruction
- Includes instruction comments

### 8. Display
- Scaled emulator screen output
- CHIP-8: 64x32 monochrome display
- Crisp pixel-perfect rendering

## Keyboard Mapping

CHIP-8 uses a 16-key hexadecimal keypad. The keys are mapped as follows:

```
CHIP-8 Keypad:        Keyboard Mapping:
┌─┬─┬─┬─┐             ┌─┬─┬─┬─┐
│1│2│3│C│             │1│2│3│4│
├─┼─┼─┼─┤             ├─┼─┼─┼─┤
│4│5│6│D│             │Q│W│E│R│
├─┼─┼─┼─┤      →      ├─┼─┼─┼─┤
│7│8│9│E│             │A│S│D│F│
├─┼─┼─┼─┤             ├─┼─┼─┼─┤
│A│0│B│F│             │Z│X│C│V│
└─┴─┴─┴─┘             └─┴─┴─┴─┘
```

## Command Line Options

```bash
chip8 [OPTIONS] <FILE>

Arguments:
  <FILE>  ROM file path

Options:
  -c, --cycles <CYCLES>  Number of CPU instructions per timer update [default: 1]
  -m, --mute             Enable to mute the beep sound
  -h, --help             Print help
  -V, --version          Print version
```
