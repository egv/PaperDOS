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
    /* Load the default system font */
    const pd_font_t *font = pd_font_load(sys, "/paperdos/fonts/default.bdf");

    /* Clear screen to white */
    pd_display_clear(sys, PD_COLOR_WHITE);

    /* Draw a border rectangle */
    sys->display_draw_rect(5, 5, pd_screen_w(sys) - 10, pd_screen_h(sys) - 10, PD_COLOR_BLACK);

    /* Draw title */
    pd_display_text(sys, 20, 20, "Hello from PaperDOS!", font);

    /* Draw some info */
    pd_display_text(sys, 20, 60, "This is a .pdb app running on", font);
    pd_display_text(sys, 20, 80, "the Xteink X4 via PaperDOS.", font);

    /* Show free heap */
    int heap = pd_free_heap(sys);
    /* Note: no printf - we'd need a simple itoa. For demo purposes
       we just show a static message */
    pd_display_text(sys, 20, 120, "Free heap: (check sys_log)", font);
    pd_log(sys, PD_LOG_INFO, "Free heap: %d bytes", heap);

    /* Show battery */
    int batt = pd_battery(sys);
    pd_log(sys, PD_LOG_INFO, "Battery: %d%%", batt);

    /* Instructions */
    pd_display_text(sys, 20, pd_screen_h(sys) - 40,
                    "Press any button to exit.", font);

    /* Full refresh to display everything */
    pd_display_refresh(sys, PD_REFRESH_FULL);

    /* Wait for a button press (no timeout) */
    pd_wait_button(sys, 0);

    /* Clean up */
    pd_font_free(sys, font);

    /* Return to launcher */
    pd_exit(sys, 0);
}
