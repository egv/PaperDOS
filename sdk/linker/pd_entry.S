/**
 * PaperDOS App Entry Trampoline
 *
 * This tiny assembly stub is the first code executed when the kernel
 * jumps to a loaded app. It:
 *   1. Sets up a minimal stack frame
 *   2. Calls pd_main(syscalls) where syscalls is passed in a0
 *   3. If pd_main returns, calls sys_exit(0) through the syscall table
 *
 * The kernel places the syscall table pointer in a0 before jumping here,
 * following the standard RISC-V calling convention.
 */

    .section .text.pd_entry, "ax"
    .global _pd_entry
    .type _pd_entry, @function

_pd_entry:
    /* a0 already contains pd_syscalls_t* from kernel */

    /* Save the syscall table pointer - we'll need it if pd_main returns */
    mv      s0, a0

    /* Call pd_main(sys) - a0 is already the argument */
    call    pd_main

    /* Current app model: returning from pd_main returns control to the kernel. */
    ret

    .size _pd_entry, . - _pd_entry
