; Minimal Game Boy test ROM
; This ROM fills VRAM with a pattern and loops

SECTION "Header", ROM0[$100]
    nop
    jp Start

; Minimal header (required for Game Boy ROMs)
SECTION "ROM", ROM0[$150]
Start:
    ; Disable interrupts
    di
    
    ; Set up stack pointer
    ld sp, $FFFE
    
    ; Fill VRAM with a simple pattern
    ld hl, $8000      ; VRAM start
    ld bc, $2000      ; VRAM size
    ld a, $AA         ; Pattern
.fillLoop:
    ld [hl+], a
    dec bc
    ld a, b
    or c
    jr nz, .fillLoop
    
    ; Infinite loop
.mainLoop:
    nop
    jr .mainLoop
