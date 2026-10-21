/* BRD4186C / EFR32MG24 — layout for cortex-m-rt + Silicon Labs memory manager.
 *
 * RAM layout (low → high):
 *   SDK heap → .data/.bss/.uninit → main stack (to RAM top)
 *
 * cortex-m-rt sets _stack_end = end of .uninit and _stack_start = RAM top.
 * Shrinking the heap region moves static sections lower and grows the main stack.
 */

MEMORY
{
  FLASH (rx)  : ORIGIN = 0x08000000, LENGTH = 0x17e000
  RAM   (rwx) : ORIGIN = 0x20000000, LENGTH = 0x40000
  /* Dummy region: .nvm collects .simee size only (not loaded). */
  DUMMY (!ax) : ORIGIN = 0xFFE82000, LENGTH = 0x17e000
}

/* ~88 KiB main stack with current static size (cortex-m-rt: RAM top − __euninit). */
_heap_size = 0x28000;

SECTIONS
{
  /* SDK heap at the bottom of RAM; link.x .data follows at __HeapLimit. */
  .memory_manager_heap (NOLOAD) : {
    . = ALIGN(8);
    PROVIDE(__HeapBase = .);
    PROVIDE(__HeapLimit = ORIGIN(RAM) + _heap_size);
    . = __HeapLimit;
    KEEP(*(.memory_manager_heap*));
  } > RAM

  /* NVM3: size from .simee, base address computed at end of flash. */
  .nvm (NOLOAD) : {
    KEEP(*(.simee*));
  } > DUMMY
}

PROVIDE(__main_flash_end__ = ORIGIN(FLASH) + LENGTH(FLASH));
PROVIDE(linker_nvm_end = __main_flash_end__);
PROVIDE(linker_nvm_begin = linker_nvm_end - SIZEOF(.nvm));
PROVIDE(__nvm3Base = linker_nvm_begin);
