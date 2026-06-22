#include <stdint.h>

/*
 * bg_message_run can block in bg_message_queue_wait_time when the host waits on
 * an internal timer. Return {0, 0} ("no wait") so pumping can proceed.
 *
 * The SDK returns two uint32 words (ms + fraction); void was wrong and left
 * undefined values in r0/r1, which could spin forever in bg_message_run.
 */
uint64_t __wrap_bg_message_queue_wait_time(uint32_t now_ms, uint32_t now_ms_frac)
{
  (void)now_ms;
  (void)now_ms_frac;
  return 0;
}
