#include "sl_btmesh_api.h"
#include "sl_status.h"

/*
 * Belt-and-suspenders: zero argument registers before the BGAPI SOC command
 * wrapper saves them on its own stack frame.
 */
sl_status_t __real_sl_btmesh_node_init(void);

sl_status_t __wrap_sl_btmesh_node_init(void)
{
  __asm volatile(
      "mov r0, #0\n"
      "mov r1, #0\n"
      "mov r2, #0\n"
      "mov r3, #0\n"
      ::: "r0", "r1", "r2", "r3", "memory");
  return __real_sl_btmesh_node_init();
}
