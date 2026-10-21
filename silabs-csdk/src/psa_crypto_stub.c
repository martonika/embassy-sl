#include <stddef.h>
#include <stdint.h>

typedef int32_t psa_status_t;
typedef uint32_t mbedtls_svc_key_id_t;

#define PSA_SUCCESS ((psa_status_t)0)
#define PSA_ERROR_GENERIC_ERROR ((psa_status_t)-132)
#define PSA_ERROR_INVALID_HANDLE ((psa_status_t)-136)

psa_status_t psa_crypto_init(void)
{
    return PSA_SUCCESS;
}

psa_status_t psa_generate_random(uint8_t *output, size_t output_size)
{
    static uint32_t seed = 0xC0FFEE01U;
    if (output == NULL) {
        return PSA_ERROR_GENERIC_ERROR;
    }
    for (size_t i = 0; i < output_size; i++) {
        seed = seed * 1664525U + 1013904223U;
        output[i] = (uint8_t)(seed >> 16);
    }
    return PSA_SUCCESS;
}

psa_status_t psa_destroy_key(mbedtls_svc_key_id_t key)
{
    if (key == 0) {
        return PSA_ERROR_INVALID_HANDLE;
    }
    return PSA_SUCCESS;
}
