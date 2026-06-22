#include <stdint.h>

typedef struct {
    void (*on_event)(void *context);
    void *context;
} sl_power_manager_em_transition_event_info_t;

int sl_power_manager_subscribe_em_transition_event(
    sl_power_manager_em_transition_event_info_t *info)
{
    (void)info;
    return 0;
}

void sl_power_manager_unsubscribe_em_transition_event(
    sl_power_manager_em_transition_event_info_t *info)
{
    (void)info;
}

void sli_power_manager_update_hf_clock_settings_preservation_requirement(int add)
{
    (void)add;
}

void sl_power_manager_sleep(void)
{
    __asm volatile("wfi");
}
