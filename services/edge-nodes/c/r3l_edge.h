/*
 * R3L Edge Client — minimal C header for IoT/embedded devices.
 *
 * This client handles:
 *   1. SHA-256 content hashing
 *   2. Ed25519 wallet signing
 *   3. JSON payload construction
 *   4. HTTP POST to the R3L API
 *
 * Dependencies: libcurl, openssl (or mbedtls)
 * Compile: gcc -o r3l-edge r3l_edge.c -lcurl -lcrypto
 */
#ifndef R3L_EDGE_H
#define R3L_EDGE_H

#include <stddef.h>
#include <stdint.h>

#define R3L_HASH_LEN      32
#define R3L_SIG_LEN       64
#define R3L_PUBKEY_LEN    32
#define R3L_PRIVKEY_LEN   64  /* Ed25519 expanded key */
#define R3L_HEX_HASH_LEN  65 /* 64 hex chars + null */
#define R3L_API_KEY_LEN   64

typedef struct {
    char     api_url[256];
    char     api_key[R3L_API_KEY_LEN];
    uint8_t  privkey[R3L_PRIVKEY_LEN];
    uint8_t  pubkey[R3L_PUBKEY_LEN];
    int      has_keypair;
} r3l_edge_ctx;

/* Initialize context with API URL. */
void r3l_init(r3l_edge_ctx *ctx, const char *api_url, const char *api_key);

/* Load a 64-byte Ed25519 keypair from a Solana-style JSON array file. */
int r3l_load_keypair(r3l_edge_ctx *ctx, const char *path);

/* SHA-256 hash a file. Writes 32 bytes to hash_out, 64-char hex to hex_out. */
int r3l_hash_file(const char *path, uint8_t hash_out[R3L_HASH_LEN], char hex_out[R3L_HEX_HASH_LEN]);

/* Sign "R3L: attest <hex_hash>" with the loaded keypair. */
int r3l_sign_attest(r3l_edge_ctx *ctx, const char *hex_hash, uint8_t sig_out[R3L_SIG_LEN]);

/* Submit attestation to the API. tlsh_hex may be NULL. Returns 0 on success. */
int r3l_attest(r3l_edge_ctx *ctx, const char *content_hash_hex, int has_c2pa,
               const char *tlsh_hex);

/* Query trust verdict. Prints JSON to stdout. Returns 0 on success. */
int r3l_query(r3l_edge_ctx *ctx, const char *content_hash_hex);

#endif /* R3L_EDGE_H */
