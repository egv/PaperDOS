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

    /* If pd_main returns, call sys_exit(0) */
    /* sys_exit is at a known offset in the syscall table:        */
    /*   4 metadata uint32s     = 16 bytes  (0x00-0x0F)          */
    /*   10 display ptrs        = 40 bytes  (0x10-0x37)          */
    /*   3 input ptrs           = 12 bytes  (0x38-0x43)          */
    /*   13 filesystem ptrs     = 52 bytes  (0x44-0x77)          */
    /*   11 network ptrs        = 44 bytes  (0x78-0xA3)          */
    /*   2 sys ptrs (sleep,ms)  =  8 bytes  (0xA4-0xAB)          */
    /*   sys_exit               =  offset 0xAC (field #43)        */
    li      a0, 0           /* exit code 0 */
    lw      t0, 0xAC(s0)    /* load sys_exit function pointer */
    jalr    t0              /* call sys_exit(0) */

    /* Should never reach here, but just in case */
1:  j       1b

    .size _pd_entry, . - _pd_entry
