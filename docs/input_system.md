# AtulyaOS Input Subsystems

AtulyaOS interacts with hardware using PS/2 controller polling. The system disables standard hardware interrupts during boot, and polls I/O ports in the main loop to ensure absolute predictability and simple control flow.

## I/O Port Layout

- **Data Port (`0x60`)**: Reads keyboard scan codes and mouse packets; writes commands to the PS/2 devices.
- **Status/Command Port (`0x64`)**: Reads controller status; writes commands to the PS/2 controller.

## Keyboard Translation

The keyboard subsystem polls port `0x64` to check if bit 0 (Output Buffer Full) is set. It then checks if the buffer belongs to the keyboard (bit 5 is 0) or the mouse (bit 5 is 1).
If it's keyboard data, the raw scan code is read from `0x60` and translated using a state tracker:
- **Shift Tracker**: Remembers if Left/Right Shift keys are held to capitalize letters.
- **Alt Commands**: Used for window operations:
  - `Tab`: Toggles focus between the Terminal and System Monitor windows.
  - `Alt + Arrow Keys`: Moves the active window.

## Polling-Based PS/2 Mouse Driver

To support direct mouse interaction (window dragging, button clicking), the PS/2 mouse is initialized and polled synchronously in the main loop.

### Initialization Sequence
1. Write command `0xA8` to `0x64` (Enable Auxiliary/Mouse Device).
2. Read current command byte: Write command `0x20` to `0x64`, wait, and read command byte from `0x60`.
3. Enable mouse packets: Set bit 1 (enable mouse interrupt/data line) and clear bit 5 (enable mouse clock), then write `0x60` to `0x64` followed by the modified command byte to `0x60`.
4. Set default settings: Write `0xD4` to `0x64` (send next byte to mouse) and write `0xF6` to `0x60`. Wait for ACK (`0xFA`).
5. Enable data reporting: Write `0xD4` to `0x64` and write `0xF4` to `0x60`. Wait for ACK (`0xFA`).

### Packet Format
A standard PS/2 mouse packet consists of 3 bytes:

| Byte | Bit 7 | Bit 6 | Bit 5 | Bit 4 | Bit 3 | Bit 2 | Bit 1 | Bit 0 |
|---|---|---|---|---|---|---|---|---|
| **0** | Y Overflow | X Overflow | Y Sign | X Sign | Always 1 | Middle Button | Right Button | Left Button |
| **1** | Delta X (Relative X movement) | | | | | | | |
| **2** | Delta Y (Relative Y movement) | | | | | | | |

*Note: The Y-axis delta from the PS/2 mouse points upwards. To translate to screen coordinates (where Y points downwards), we subtract the Y delta from the current Y coordinate.*
