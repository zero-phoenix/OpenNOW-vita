#ifndef VITA_ADBD_SHA256_H
#define VITA_ADBD_SHA256_H

#include <stdint.h>

void vita_adbd_hmac_sha256(const uint8_t *key, unsigned int key_len,
                           const uint8_t *data, unsigned int data_len,
                           uint8_t output[32]);
int vita_adbd_constant_time_equal(const uint8_t *left, const uint8_t *right,
                                  unsigned int length);

#endif
