#include "sl_mbedtls.h"
#include "psa/crypto.h"
#include "sl_se_manager.h"
#include "sli_protocol_crypto.h"
#include "sli_crypto.h"

void silabs_crypto_platform_init(void)
{
  sl_mbedtls_init();
  (void)psa_crypto_init();
  (void)sl_se_init();
  (void)sli_protocol_crypto_init();
  (void)sli_crypto_init();
  sli_aes_seed_mask();
}
