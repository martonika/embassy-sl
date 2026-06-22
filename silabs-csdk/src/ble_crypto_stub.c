#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include "sl_status.h"

#define SLI_CRYPTO_AES_BLOCK_SIZE 16

typedef struct sli_crypto_descriptor sli_crypto_descriptor_t;

sl_status_t sli_crypto_ccm_encrypt_and_tag_ble(sli_crypto_descriptor_t *key_descriptor,
                                               unsigned char *data,
                                               size_t length,
                                               const unsigned char *iv,
                                               unsigned char header,
                                               unsigned char *tag)
{
    (void)key_descriptor;
    (void)data;
    (void)length;
    (void)iv;
    (void)header;
    (void)tag;
    return SL_STATUS_OK;
}

sl_status_t sli_crypto_ccm_auth_decrypt_ble(sli_crypto_descriptor_t *key_descriptor,
                                            unsigned char *data,
                                            size_t length,
                                            const unsigned char *iv,
                                            unsigned char header,
                                            unsigned char *tag)
{
    (void)key_descriptor;
    (void)data;
    (void)length;
    (void)iv;
    (void)header;
    (void)tag;
    return SL_STATUS_OK;
}

sl_status_t sli_crypto_aes_ecb_radio(bool encrypt,
                                     sli_crypto_descriptor_t *key_descriptor,
                                     unsigned int keybits,
                                     const unsigned char input[SLI_CRYPTO_AES_BLOCK_SIZE],
                                     volatile unsigned char output[SLI_CRYPTO_AES_BLOCK_SIZE])
{
    (void)encrypt;
    (void)key_descriptor;
    (void)keybits;
    for (int i = 0; i < SLI_CRYPTO_AES_BLOCK_SIZE; i++) {
        output[i] = input[i];
    }
    return SL_STATUS_OK;
}

sl_status_t sli_crypto_aes_ctr_radio(sli_crypto_descriptor_t *key_descriptor,
                                     unsigned int keybits,
                                     const unsigned char input[SLI_CRYPTO_AES_BLOCK_SIZE],
                                     const unsigned char iv_in[SLI_CRYPTO_AES_BLOCK_SIZE],
                                     volatile unsigned char iv_out[SLI_CRYPTO_AES_BLOCK_SIZE],
                                     volatile unsigned char output[SLI_CRYPTO_AES_BLOCK_SIZE])
{
    (void)key_descriptor;
    (void)keybits;
    (void)iv_in;
    for (int i = 0; i < SLI_CRYPTO_AES_BLOCK_SIZE; i++) {
        iv_out[i] = iv_in[i];
        output[i] = input[i];
    }
    return SL_STATUS_OK;
}
