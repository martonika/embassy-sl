#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>
#include "em_device.h"
#include "sl_core.h"
#include "sl_status.h"
#include "sl_sleeptimer.h"

/* Embassy owns SYSRTC; read CNT for tick timebase and poll timer expirations. */

#define SLEEPTIMER_HZ 32768U

typedef uint32_t sl_sleeptimer_tick_count_t;

static sl_sleeptimer_timer_handle_t *timer_head;
static sl_sleeptimer_tick_count_t last_delta_update_count;
static volatile uint32_t overflow_counter;
static bool is_sleeptimer_initialized;

static uint32_t hal_get_counter(void)
{
    return SYSRTC0->CNT;
}

static uint32_t ticks32_to_ms(uint32_t tick)
{
    return (tick >> 15) * 1000U + (((tick & 0x7FFFU) * 1000U) >> 15);
}

static uint64_t ticks64_to_ms(uint64_t tick)
{
    uint64_t ms = (tick >> 15) * 1000ULL;
    ms += ((tick & 0x7FFFULL) * 1000ULL) >> 15;
    return ms;
}

static void update_delta_list(void)
{
    sl_sleeptimer_tick_count_t current_cnt = hal_get_counter();
    sl_sleeptimer_timer_handle_t *timer_handle = timer_head;
    sl_sleeptimer_tick_count_t time_diff = current_cnt - last_delta_update_count;

    while (timer_handle != NULL && time_diff > 0) {
        if (timer_handle->delta >= time_diff) {
            timer_handle->delta -= time_diff;
            time_diff = 0;
        } else {
            time_diff -= timer_handle->delta;
            timer_handle->delta = 0;
        }
        timer_handle = timer_handle->next;
    }

    last_delta_update_count = current_cnt;
}

static void delta_list_insert_timer(sl_sleeptimer_timer_handle_t *handle,
                                    sl_sleeptimer_tick_count_t timeout)
{
    sl_sleeptimer_tick_count_t local_handle_delta = timeout;

    handle->delta = local_handle_delta;

    if (timer_head != NULL) {
        sl_sleeptimer_timer_handle_t *prev = NULL;
        sl_sleeptimer_timer_handle_t *current = timer_head;

        while (current != NULL
               && (local_handle_delta >= current->delta || current->delta == 0u
                   || (((local_handle_delta - current->delta) == 0)
                       && (handle->priority > current->priority)))) {
            local_handle_delta -= current->delta;
            handle->delta = local_handle_delta;
            prev = current;
            current = current->next;
        }

        if (prev != NULL) {
            prev->next = handle;
        } else {
            timer_head = handle;
        }
        handle->next = current;

        if (current != NULL) {
            current->delta -= local_handle_delta;
        }
    } else {
        timer_head = handle;
        handle->next = NULL;
    }
}

static sl_status_t delta_list_remove_timer(sl_sleeptimer_timer_handle_t *handle)
{
    sl_sleeptimer_timer_handle_t *prev = NULL;
    sl_sleeptimer_timer_handle_t *current = timer_head;

    if (handle == NULL) {
        return SL_STATUS_NULL_POINTER;
    }

    while (current != NULL && current != handle) {
        prev = current;
        current = current->next;
    }

    if (current != handle) {
        return SL_STATUS_INVALID_STATE;
    }

    if (prev != NULL) {
        prev->next = handle->next;
    } else {
        timer_head = handle->next;
    }

    if (handle->next != NULL) {
        handle->next->delta += handle->delta;
    }

    handle->next = NULL;
    return SL_STATUS_OK;
}

static void process_expired_timer(sl_sleeptimer_timer_handle_t *timer)
{
    int32_t periodic_correction = 0;
    int64_t timeout_temp = 0;
    bool skip_remove = false;

    if (timer->timeout_periodic != 0u) {
        timeout_temp = timer->timeout_periodic;
        periodic_correction = (int32_t)(hal_get_counter() - timer->timeout_expected_tc);
        if (periodic_correction >= timeout_temp) {
            skip_remove = true;
            timer->timeout_expected_tc += timer->timeout_periodic;
        }
    }

    if (!skip_remove) {
        CORE_DECLARE_IRQ_STATE;
        CORE_ENTER_ATOMIC();
        delta_list_remove_timer(timer);
        CORE_EXIT_ATOMIC();
    }

    if (timer->timeout_periodic != 0u && !skip_remove) {
        timeout_temp -= periodic_correction;
        if (timeout_temp > 0) {
            CORE_DECLARE_IRQ_STATE;
            CORE_ENTER_ATOMIC();
            delta_list_insert_timer(timer, (sl_sleeptimer_tick_count_t)timeout_temp);
            timer->timeout_expected_tc += timer->timeout_periodic;
            CORE_EXIT_ATOMIC();
        }
    }

    if (timer->callback != NULL) {
        timer->callback(timer, timer->callback_data);
    }
}

static sl_status_t create_timer(sl_sleeptimer_timer_handle_t *handle,
                                sl_sleeptimer_tick_count_t timeout_initial,
                                sl_sleeptimer_tick_count_t timeout_periodic,
                                sl_sleeptimer_timer_callback_t callback,
                                void *callback_data,
                                uint8_t priority,
                                uint16_t option_flags)
{
    CORE_DECLARE_IRQ_STATE;

    handle->priority = priority;
    handle->callback_data = callback_data;
    handle->next = NULL;
    handle->timeout_periodic = timeout_periodic;
    handle->callback = callback;
    handle->option_flags = option_flags;
    handle->conversion_error = 0;
    handle->accumulated_error = 0;

    if (timeout_periodic == 0) {
        handle->timeout_expected_tc = hal_get_counter() + timeout_initial;
    } else {
        handle->timeout_expected_tc = hal_get_counter() + timeout_periodic;
    }

    if (timeout_initial == 0) {
        handle->delta = 0;
        if (handle->callback != NULL) {
            handle->callback(handle, handle->callback_data);
        }
        if (timeout_periodic != 0) {
            timeout_initial = timeout_periodic;
        } else {
            return SL_STATUS_OK;
        }
    }

    CORE_ENTER_CRITICAL();
    update_delta_list();
    delta_list_insert_timer(handle, timeout_initial);
    CORE_EXIT_CRITICAL();

    return SL_STATUS_OK;
}

#define SILABS_SLEEPTIMER_MAX_EXPIRED_PER_STEP 32u

void silabs_sleeptimer_step(void)
{
    if (!is_sleeptimer_initialized) {
        return;
    }

    CORE_DECLARE_IRQ_STATE;
    unsigned processed = 0;

    CORE_ENTER_ATOMIC();
    update_delta_list();

    while (timer_head != NULL && timer_head->delta == 0
           && processed < SILABS_SLEEPTIMER_MAX_EXPIRED_PER_STEP) {
        sl_sleeptimer_timer_handle_t *current = timer_head;
        sl_sleeptimer_timer_handle_t *temp = timer_head;

        while (temp != NULL && temp->delta == 0) {
            if (current->priority > temp->priority) {
                current = temp;
            }
            temp = temp->next;
        }
        CORE_EXIT_ATOMIC();

        process_expired_timer(current);
        processed++;

        CORE_ENTER_ATOMIC();
        update_delta_list();
    }
    CORE_EXIT_ATOMIC();
}

sl_status_t sl_sleeptimer_init(void)
{
    CORE_DECLARE_IRQ_STATE;

    CORE_ENTER_ATOMIC();
    if (!is_sleeptimer_initialized) {
        timer_head = NULL;
        last_delta_update_count = hal_get_counter();
        overflow_counter = 0;
        is_sleeptimer_initialized = true;
    }
    CORE_EXIT_ATOMIC();

    return SL_STATUS_OK;
}

uint32_t sl_sleeptimer_get_tick_count(void)
{
    return hal_get_counter();
}

uint64_t sl_sleeptimer_get_tick_count64(void)
{
    return ((uint64_t)overflow_counter << 32) | hal_get_counter();
}

uint32_t sl_sleeptimer_get_timer_frequency(void)
{
    return SLEEPTIMER_HZ;
}

uint32_t sl_sleeptimer_tick_to_ms(uint32_t tick)
{
    return ticks32_to_ms(tick);
}

sl_status_t sl_sleeptimer_tick64_to_ms(uint64_t tick, uint64_t *ms)
{
    if (ms == NULL) {
        return SL_STATUS_NULL_POINTER;
    }
    *ms = ticks64_to_ms(tick);
    return SL_STATUS_OK;
}

sl_status_t sl_sleeptimer_start_timer(sl_sleeptimer_timer_handle_t *handle,
                                      uint32_t timeout,
                                      sl_sleeptimer_timer_callback_t callback,
                                      void *callback_data,
                                      uint8_t priority,
                                      uint16_t option_flags)
{
    if (handle == NULL) {
        return SL_STATUS_NULL_POINTER;
    }

    return create_timer(handle,
                        timeout,
                        0,
                        callback,
                        callback_data,
                        priority,
                        option_flags);
}

sl_status_t sl_sleeptimer_restart_timer(sl_sleeptimer_timer_handle_t *handle,
                                        uint32_t timeout,
                                        sl_sleeptimer_timer_callback_t callback,
                                        void *callback_data,
                                        uint8_t priority,
                                        uint16_t option_flags)
{
    if (handle == NULL) {
        return SL_STATUS_NULL_POINTER;
    }

    (void)sl_sleeptimer_stop_timer(handle);

    return create_timer(handle,
                        timeout,
                        0,
                        callback,
                        callback_data,
                        priority,
                        option_flags);
}

sl_status_t sl_sleeptimer_stop_timer(sl_sleeptimer_timer_handle_t *handle)
{
    CORE_DECLARE_IRQ_STATE;
    sl_status_t status;

    if (handle == NULL) {
        return SL_STATUS_NULL_POINTER;
    }

    CORE_ENTER_ATOMIC();
    update_delta_list();
    status = delta_list_remove_timer(handle);
    CORE_EXIT_ATOMIC();

    return status;
}

sl_status_t sl_sleeptimer_ms32_to_tick(uint32_t time_ms, uint32_t *tick)
{
    if (tick == NULL) {
        return SL_STATUS_NULL_POINTER;
    }
    *tick = (time_ms * SLEEPTIMER_HZ + 500U) / 1000U;
    return SL_STATUS_OK;
}

uint16_t sl_sleeptimer_get_clock_accuracy(void)
{
    return 500;
}
