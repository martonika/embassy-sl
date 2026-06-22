#include "em_gpio.h"

typedef struct {
    unsigned int port;
    unsigned int pin;
} sl_gpio_t;

void sl_gpio_set_pin(const sl_gpio_t *gpio)
{
    GPIO_PinOutSet((GPIO_Port_TypeDef)gpio->port, gpio->pin);
}

void sl_gpio_clear_pin(const sl_gpio_t *gpio)
{
    GPIO_PinOutClear((GPIO_Port_TypeDef)gpio->port, gpio->pin);
}

typedef enum {
    SL_GPIO_MODE_PUSH_PULL = 0,
} sl_gpio_mode_t;

int sl_gpio_set_pin_mode(const sl_gpio_t *gpio,
                         sl_gpio_mode_t mode,
                         unsigned char output_value)
{
    (void)mode;
    GPIO_PinModeSet((GPIO_Port_TypeDef)gpio->port,
                    gpio->pin,
                    gpioModePushPull,
                    output_value);
    return 0;
}
