# X4 Display Bring-Up Notes

These notes capture the main issues we hit while bringing up the SSD1677 display on the Xteink X4.

## What Went Wrong

- The controller did not like our original full-frame write path. A single `WRITE_RAM_*` command followed by host-side DMA chunking was not enough when chip select and addressing did not match the working `pulp-os` sequence, so only a small stripe of the image landed on screen.
- Our first addressing code treated the RAM window like a simple landscape framebuffer. The X4 panel is wired with flipped gate order, and the working sequence is the `pulp-os` `set_partial_ram_area()` logic: program `DATA_ENTRY_MODE = 0x01`, use 16-bit X pixel addresses, and flip Y before setting the RAM window and cursor.
- After the image finally rendered, it was rotated 90 degrees clockwise. The panel's practical default orientation matches `pulp-os` `Rotation::Deg270`, so a naive `800x480` landscape logical API does not match what the device naturally shows.

## Working Baseline

- Use the `pulp-os`-style RAM window/cursor setup for both full-frame and strip writes.
- Write both RAM planes (`0x26` previous, then `0x24` current) before a full refresh.
- Stream the framebuffer in 40-row strips so each SPI transfer stays within the DMA budget.
- Expose portrait logical coordinates (`480x800`) at the syscall/API layer, and rotate logical drawing into the physical `800x480` packed framebuffer.

## Current Runtime Model

- The boot splash is streamed directly from the packed image asset in flash; it no longer needs a 48 KB RAM framebuffer.
- The retained drawing API keeps a compact display scene (clear color + draw ops) and replays it into a 4 KB strip buffer during refresh, which matches the original no-full-framebuffer PaperDOS design.

## Temporary Splash Note

- `kernel/assets/boris.bin` is currently pre-rotated so the boot splash appears upright while the rest of the display stack moves to the portrait logical orientation.
