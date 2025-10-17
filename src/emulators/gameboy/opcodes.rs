// Game Boy CPU Opcode cycle timings
// Based on Pan Docs (https://gekkio.fi/files/gb-docs/gbctr.pdf)

/// Returns the cycle count for a given opcode
/// Some opcodes have variable cycles based on branch conditions
pub fn get_opcode_cycles(opcode: u8) -> (u32, u32) {
    // Returns (cycles_if_not_taken, cycles_if_taken)
    // For non-branching instructions, both values are the same
    match opcode {
        // 4-cycle instructions
        0x00 | 0x02 | 0x03 | 0x04 | 0x05 | 0x07 | 0x08 | 0x09 | 0x0A | 0x0B | 0x0C | 0x0D | 0x0F => (4, 4),
        0x12 | 0x13 | 0x14 | 0x15 | 0x17 | 0x18 | 0x19 | 0x1A | 0x1B | 0x1C | 0x1D | 0x1F => (4, 4),
        0x22 | 0x23 | 0x24 | 0x25 | 0x27 | 0x28 | 0x29 | 0x2A | 0x2B | 0x2C | 0x2D | 0x2F => (4, 4),
        0x32 | 0x33 | 0x34 | 0x35 | 0x37 | 0x38 | 0x39 | 0x3A | 0x3B | 0x3C | 0x3D | 0x3F => (4, 4),
        
        // LD r,r instructions (4 cycles, except LD r,(HL) and LD (HL),r)
        0x40..=0x75 | 0x77..=0x7F => (4, 4),
        0x76 => (4, 4), // HALT
        
        // ADD, ADC, SUB, SBC, AND, XOR, OR, CP with register (4 cycles)
        0x80..=0xBF => (4, 4),
        
        // 8-cycle instructions
        0x01 | 0x06 | 0x0E | 0x11 | 0x16 | 0x1E | 0x20 | 0x21 | 0x26 | 0x2E | 0x30 | 0x31 | 0x36 | 0x3E => (8, 8),
        0xC6 | 0xCE | 0xD6 | 0xDE | 0xE6 | 0xEE | 0xF6 | 0xFE => (8, 8),
        0xE0 | 0xE2 | 0xF0 | 0xF2 => (12, 12),
        0xEA | 0xFA => (16, 16),
        
        // 12-cycle instructions  
        0xC1 | 0xC5 | 0xD1 | 0xD5 | 0xE1 | 0xE5 | 0xF1 | 0xF5 => (16, 16),
        0xC3 | 0xC9 | 0xCD | 0xD9 | 0xE9 | 0xF9 | 0xFB | 0xF3 => (16, 16),
        
        // Conditional jumps and calls (variable cycles)
        0x20 | 0x30 => (8, 12),  // JR cc,r8
        0x28 | 0x38 => (8, 12),
        0xC0 | 0xD0 => (8, 20),  // RET cc
        0xC8 | 0xD8 => (8, 20),
        0xC2 | 0xD2 => (12, 16), // JP cc,a16
        0xCA | 0xDA => (12, 16),
        0xC4 | 0xD4 => (12, 24), // CALL cc,a16
        0xCC | 0xDC => (12, 24),
        
        // RST instructions (16 cycles)
        0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF => (16, 16),
        
        // CB prefix (variable, handled separately)
        0xCB => (4, 4), // The prefix itself, actual instruction adds more
        
        // Default for unimplemented
        _ => (4, 4),
    }
}

/// Returns the cycle count for CB-prefixed opcodes
pub fn get_cb_opcode_cycles(opcode: u8) -> u32 {
    match opcode {
        // All CB instructions are either 8 cycles (register) or 16 cycles ((HL))
        0x00..=0x3F => {
            if (opcode & 0x07) == 0x06 {
                16 // Operations on (HL)
            } else {
                8  // Operations on registers
            }
        }
        0x40..=0x7F => {
            if (opcode & 0x07) == 0x06 {
                12 // BIT n,(HL)
            } else {
                8  // BIT n,r
            }
        }
        0x80..=0xBF => {
            if (opcode & 0x07) == 0x06 {
                16 // RES n,(HL)
            } else {
                8  // RES n,r
            }
        }
        0xC0..=0xFF => {
            if (opcode & 0x07) == 0x06 {
                16 // SET n,(HL)
            } else {
                8  // SET n,r
            }
        }
    }
}
