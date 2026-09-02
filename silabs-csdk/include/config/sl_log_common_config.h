#ifndef SL_LOG_COMMON_CONFIG_H
#define SL_LOG_COMMON_CONFIG_H

/* Embassy BLE build: headers only. Keep compile-time logging off so
 * 2026.12 memory_manager's sli_memory_manager_log.h can include
 * sl_log_component.h without linking the sl_log backend. */

#define SL_LOG_CONFIG_LEVEL_DEBUG 1
#define SL_LOG_CONFIG_LEVEL_INFO 2
#define SL_LOG_CONFIG_LEVEL_WARN 3
#define SL_LOG_CONFIG_LEVEL_ERROR 4
#define SL_LOG_CONFIG_LEVEL_CRASH 5
#define SL_LOG_CONFIG_LEVEL_NONE 6

#define SL_LOG_CONFIG_ARG0 0
#define SL_LOG_CONFIG_ARG1 1
#define SL_LOG_CONFIG_ARG2 2
#define SL_LOG_CONFIG_ARG3 3
#define SL_LOG_CONFIG_ARG4 4
#define SL_LOG_CONFIG_ARG5 5
#define SL_LOG_CONFIG_ARG6 6
#define SL_LOG_CONFIG_ARG7 7
#define SL_LOG_CONFIG_ARG8 8
#define SL_LOG_CONFIG_ARG9 9
#define SL_LOG_CONFIG_ARG10 10

#define SL_LOG_CONFIG_LEVEL_COMPILE_TIME SL_LOG_CONFIG_LEVEL_NONE
#define SL_LOG_CONFIG_ARG 3
#define SL_LOG_NUMBER_OF_EVENTS 128
#define SL_LOG_DEBUG_ASSERT_ENABLE 0
#define SL_LOG_EVENT_ID_CRASH 0x43525348U

#endif /* SL_LOG_COMMON_CONFIG_H */
