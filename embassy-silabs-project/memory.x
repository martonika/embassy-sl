MEMORY
{
  FLASH (rx) : ORIGIN = 0x08000000, LENGTH = 0x17e000
  RAM (xrw)  : ORIGIN = 0x20000000, LENGTH = 0x40000
}

_stack_start = ORIGIN(RAM) + LENGTH(RAM);