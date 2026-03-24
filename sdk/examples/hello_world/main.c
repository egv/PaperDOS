/**
 * Hello World - PaperDOS Example Application
 *
 * The simplest possible PaperDOS app. Demonstrates:
 *   - Display text rendering
 *   - Button input
 *   - Clean exit
 *
 * Build:
 *   riscv32-esp-elf-gcc -march=rv32imc -Os -nostdlib \
 *       -T ../../linker/paperdos.ld ../../linker/pd_entry.S main.c \
 *       -o hello.elf
 *   riscv32-esp-elf-objcopy -O binary hello.elf hello.bin
 *   python3 ../../tools/pdpack.py hello.elf hello.pdb \
 *       --name "Hello World" --version "1.0.0" --abi 1
 */

#include "../../include/paperdos.h"

void pd_main(pd_syscalls_t *sys) {
    /* Clear screen to white */
    pd_display_clear(sys, PD_COLOR_WHITE);

    /* Draw a bold frame and blocks so launch is visually obvious even without
       font rendering support. */
    pd_display_fill(sys, 0, 0, pd_screen_w(sys), 56, PD_COLOR_BLACK);
    pd_display_rect(sys, 8, 8, pd_screen_w(sys) - 16, pd_screen_h(sys) - 16, PD_COLOR_BLACK);
    pd_display_fill(sys, 32, 96, pd_screen_w(sys) - 64, 40, PD_COLOR_BLACK);
    pd_display_fill(sys, 32, 176, pd_screen_w(sys) - 128, 40, PD_COLOR_BLACK);
    pd_display_fill(sys, 32, 256, pd_screen_w(sys) - 192, 40, PD_COLOR_BLACK);
    pd_display_fill(sys, 32, pd_screen_h(sys) - 104, pd_screen_w(sys) - 64, 24, PD_COLOR_BLACK);

    /* Full refresh to display everything */
    pd_display_refresh(sys, PD_REFRESH_FULL);

    /* Wait for a button press (no timeout) */
    pd_wait_button(sys, 0);
}
