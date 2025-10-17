# Emulator Dropdown Feature

The new emulator dropdown selector is located at the top of the left control panel.

## Visual Preview (Text Mockup)

```
┌─────────────────────────────────────────────┐
│ Left Panel                                  │
├─────────────────────────────────────────────┤
│                                             │
│ ┌─────────────────────────────────────────┐ │
│ │ Emulator                                │ │
│ └─────────────────────────────────────────┘ │
│                                             │
│ ┌─────────────────────────────────────────┐ │
│ │ CHIP-8                              ▼   │ │  ← Dropdown selector
│ └─────────────────────────────────────────┘ │
│                                             │
│ ───────────────────────────────────────────│
│                                             │
│ ┌─────────────────────────────────────────┐ │
│ │ Controls                                │ │
│ └─────────────────────────────────────────┘ │
│                                             │
│   [▶ Run]  [⏹ Reset]                       │
│                                             │
│   Speed: ────■──────────── 10x              │
│                                             │
│ ...                                         │
└─────────────────────────────────────────────┘
```

## When Dropdown is Clicked

```
┌─────────────────────────────────────────────┐
│ ┌─────────────────────────────────────────┐ │
│ │ Emulator                                │ │
│ └─────────────────────────────────────────┘ │
│                                             │
│ ┌─────────────────────────────────────────┐ │
│ │ CHIP-8                              ▼   │ │
│ ├─────────────────────────────────────────┤ │
│ │ ● CHIP-8                                │ │  ← Currently selected
│ │   NES (coming soon)                     │ │  ← Commented out
│ │   Game Boy (coming soon)                │ │  ← Commented out
│ └─────────────────────────────────────────┘ │
│                                             │
│ ───────────────────────────────────────────│
│ ...                                         │
└─────────────────────────────────────────────┘
```

## Code Implementation

The dropdown is implemented using egui's ComboBox:

```rust
ui.heading("Emulator");
egui::ComboBox::from_label("")
    .selected_text(&self.selected_emulator)
    .show_ui(ui, |ui| {
        ui.selectable_value(
            &mut self.selected_emulator,
            "CHIP-8".to_string(),
            "CHIP-8",
        );
        // Future emulators can be added here:
        // ui.selectable_value(&mut self.selected_emulator, "NES".to_string(), "NES");
        // ui.selectable_value(&mut self.selected_emulator, "GB".to_string(), "Game Boy");
    });
```

## Adding New Emulators to Dropdown

To add a new emulator to the dropdown:

1. **Create the emulator implementation:**
   ```rust
   // src/emulators/nes.rs
   pub struct NesEmulator { /* ... */ }
   impl Emulator for NesEmulator { /* ... */ }
   ```

2. **Add to the dropdown in main.rs:**
   ```rust
   ui.selectable_value(
       &mut self.selected_emulator,
       "NES".to_string(),
       "NES",
   );
   ```

3. **Handle emulator switching (future enhancement):**
   ```rust
   // When dropdown selection changes, swap out the emulator
   if self.selected_emulator != current_emulator {
       self.emulator = create_emulator(&self.selected_emulator);
   }
   ```

## Current State

- ✅ Dropdown UI component is working
- ✅ Shows "CHIP-8" as the current emulator
- ✅ Ready for future emulators to be added
- ℹ️ Currently only CHIP-8 is selectable
- 💡 Commented examples show how to add more emulators

## Benefits

1. **User-Friendly:** Clear visual indicator of current emulator
2. **Extensible:** Easy to add new emulators as they're implemented
3. **Future-Proof:** Architecture supports hot-swapping emulators
4. **Consistent:** Uses standard egui ComboBox component

## Screenshot Location

The actual UI screenshot with the dropdown can be seen when running the application:

```bash
cargo run --release -- path/to/rom.ch8
```

The dropdown appears at the top of the left panel, above the control buttons.
