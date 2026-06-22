/*
 * init_node() loads a halfword from [sp, #72], which aliases saved r2 on the
 * sli_btmesh_cmd_node_init() stack frame. BGAPI dispatches here with garbage
 * in r2; mesh_hal_init_node() then returns SL_STATUS_INVALID_PARAMETER (0x21).
 */
void __real_sli_btmesh_cmd_node_init(void);

void __wrap_sli_btmesh_cmd_node_init(void)
{
  __asm volatile(
      "mov r0, #0\n"
      "mov r1, #0\n"
      "mov r2, #0\n"
      "mov r3, #0\n"
      "mov r4, #0\n"
      ::: "r0", "r1", "r2", "r3", "r4", "memory");
  __real_sli_btmesh_cmd_node_init();
}
