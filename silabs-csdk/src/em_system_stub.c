#include <stddef.h>
#include <stdint.h>
#include "em_device.h"

#ifndef HFXO_FREQ
#define HFXO_FREQ 39000000
#endif

uint32_t SystemCoreClock;

static uint32_t system_hfxo_clock = HFXO_FREQ;
static uint32_t system_hfrcodpll_clock = 19000000U;

uint32_t SystemHFXOClockGet(void)
{
    return system_hfxo_clock;
}

void SystemHFXOClockSet(uint32_t freq)
{
    system_hfxo_clock = freq;
}

uint32_t SystemHFRCODPLLClockGet(void)
{
    return system_hfrcodpll_clock;
}

void SystemHFRCODPLLClockSet(uint32_t freq)
{
    system_hfrcodpll_clock = freq;
}

uint32_t SystemSYSCLKGet(void)
{
    switch (CMU->SYSCLKCTRL & _CMU_SYSCLKCTRL_CLKSEL_MASK) {
    case _CMU_SYSCLKCTRL_CLKSEL_HFRCODPLL:
        return SystemHFRCODPLLClockGet();
    case _CMU_SYSCLKCTRL_CLKSEL_HFXO:
        return SystemHFXOClockGet();
    case _CMU_SYSCLKCTRL_CLKSEL_FSRCO:
        return 20000000U;
    default:
        return SystemHFRCODPLLClockGet();
    }
}

uint32_t SystemHCLKGet(void)
{
    uint32_t presc = (CMU->SYSCLKCTRL & _CMU_SYSCLKCTRL_HCLKPRESC_MASK)
                     >> _CMU_SYSCLKCTRL_HCLKPRESC_SHIFT;
    uint32_t hclk = SystemSYSCLKGet() / (presc + 1U);
    SystemCoreClock = hclk;
    return hclk;
}

uint32_t SystemMaxCoreClockGet(void)
{
    return 80000000U;
}

uint32_t SystemHFRCOEM23ClockGet(void)
{
    return 19000000U;
}

uint32_t SystemLFRCOClockGet(void)
{
    return 32768U;
}

uint32_t SystemLFXOClockGet(void)
{
    return 32768U;
}

uint32_t SystemFSRCOClockGet(void)
{
    return 20000000U;
}

uint32_t SystemCLKIN0Get(void)
{
    return 0U;
}

uint32_t SystemULFRCOClockGet(void)
{
    return 1000U;
}

uint64_t sl_hal_system_get_unique(void)
{
    return 0x0123456789ABCDEFULL;
}

typedef struct {
    uint8_t major;
    uint8_t minor;
} sl_hal_system_chip_revision_t;

void sl_hal_system_get_chip_revision(sl_hal_system_chip_revision_t *rev)
{
    if (rev != NULL) {
        rev->major = 1;
        rev->minor = 0;
    }
}
