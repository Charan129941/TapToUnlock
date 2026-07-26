#ifndef OPENTAP_FFI_H
#define OPENTAP_FFI_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Generates an Ed25519 keypair and writes hex-encoded public and private keys to output buffers.
 * Returns 0 on success, negative error code on failure.
 */
int32_t opentap_ffi_generate_keypair(
    char* out_pub_hex,
    size_t pub_max_len,
    char* out_priv_hex,
    size_t priv_max_len
);

/**
 * Signs an unlock payload with Ed25519 private key and serializes to Postcard binary format.
 * Returns 0 on success.
 */
int32_t opentap_ffi_sign_payload(
    const char* uuid_str,
    const char* priv_hex,
    const char* pc_id,
    const char* action_str,
    uint64_t counter,
    uint8_t* out_buf,
    size_t out_max_len,
    size_t* out_actual_len
);

/**
 * Parses an Out-Of-Band QR Code challenge URI and writes JSON parameters to output buffer.
 * Returns 0 on success.
 */
int32_t opentap_ffi_parse_qr_uri(
    const char* uri_str,
    char* out_json,
    size_t max_len
);

#ifdef __cplusplus
}
#endif

#endif // OPENTAP_FFI_H
