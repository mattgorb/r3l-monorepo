/*
 * R3L Edge Client — minimal C implementation for IoT/embedded.
 *
 * Compile:
 *   gcc -O2 -o r3l-edge r3l_edge.c main.c -lcurl -lcrypto
 *
 * Or for mbedtls (embedded):
 *   gcc -O2 -DR3L_USE_MBEDTLS -o r3l-edge r3l_edge.c main.c -lmbedcrypto -lcurl
 */
#include "r3l_edge.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <openssl/evp.h>
#include <openssl/sha.h>
#include <curl/curl.h>

/* ── Helpers ──────────────────────────────────────────────────── */

static void bytes_to_hex(const uint8_t *bytes, size_t len, char *hex) {
    static const char tab[] = "0123456789abcdef";
    for (size_t i = 0; i < len; i++) {
        hex[i * 2]     = tab[bytes[i] >> 4];
        hex[i * 2 + 1] = tab[bytes[i] & 0x0f];
    }
    hex[len * 2] = '\0';
}

/* Base58 encode (Bitcoin alphabet). Caller must provide buf of sufficient size. */
static int base58_encode(const uint8_t *data, size_t len, char *buf, size_t buf_size) {
    static const char ALPHA[] = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

    /* Count leading zeros */
    size_t zeros = 0;
    while (zeros < len && data[zeros] == 0) zeros++;

    /* Allocate enough space (log(256)/log(58) ~ 1.366) */
    size_t out_size = (len - zeros) * 138 / 100 + 1;
    uint8_t *tmp = calloc(out_size, 1);
    if (!tmp) return -1;

    for (size_t i = zeros; i < len; i++) {
        int carry = data[i];
        for (size_t j = 0; j < out_size; j++) {
            carry += 256 * tmp[j];
            tmp[j] = carry % 58;
            carry /= 58;
        }
    }

    /* Find first non-zero in tmp (reversed) */
    size_t k = out_size;
    while (k > 0 && tmp[k - 1] == 0) k--;

    size_t total = zeros + k;
    if (total + 1 > buf_size) { free(tmp); return -1; }

    for (size_t i = 0; i < zeros; i++) buf[i] = '1';
    for (size_t i = 0; i < k; i++) buf[zeros + i] = ALPHA[tmp[k - 1 - i]];
    buf[total] = '\0';

    free(tmp);
    return (int)total;
}

/* curl write callback — append to a dynamic buffer */
struct response_buf {
    char  *data;
    size_t len;
};

static size_t write_cb(void *ptr, size_t size, size_t nmemb, void *userdata) {
    struct response_buf *buf = (struct response_buf *)userdata;
    size_t total = size * nmemb;
    char *tmp = realloc(buf->data, buf->len + total + 1);
    if (!tmp) return 0;
    buf->data = tmp;
    memcpy(buf->data + buf->len, ptr, total);
    buf->len += total;
    buf->data[buf->len] = '\0';
    return total;
}

/* ── Public API ───────────────────────────────────────────────── */

void r3l_init(r3l_edge_ctx *ctx, const char *api_url, const char *api_key) {
    memset(ctx, 0, sizeof(*ctx));
    strncpy(ctx->api_url, api_url, sizeof(ctx->api_url) - 1);
    if (api_key)
        strncpy(ctx->api_key, api_key, sizeof(ctx->api_key) - 1);
}

int r3l_load_keypair(r3l_edge_ctx *ctx, const char *path) {
    FILE *f = fopen(path, "r");
    if (!f) { perror("fopen keypair"); return -1; }

    /* Read the entire JSON array */
    fseek(f, 0, SEEK_END);
    long fsize = ftell(f);
    fseek(f, 0, SEEK_SET);

    char *json = malloc(fsize + 1);
    if (!json) { fclose(f); return -1; }
    fread(json, 1, fsize, f);
    json[fsize] = '\0';
    fclose(f);

    /* Parse JSON array of integers [b0, b1, ..., b63] */
    uint8_t bytes[64];
    int count = 0;
    char *p = json;

    while (*p && count < 64) {
        if (*p >= '0' && *p <= '9') {
            bytes[count++] = (uint8_t)strtol(p, &p, 10);
        } else {
            p++;
        }
    }
    free(json);

    if (count < 64) {
        fprintf(stderr, "Keypair file must contain 64 bytes, got %d\n", count);
        return -1;
    }

    memcpy(ctx->privkey, bytes, 64);
    memcpy(ctx->pubkey, bytes + 32, 32);
    ctx->has_keypair = 1;
    return 0;
}

int r3l_hash_file(const char *path, uint8_t hash_out[R3L_HASH_LEN], char hex_out[R3L_HEX_HASH_LEN]) {
    FILE *f = fopen(path, "rb");
    if (!f) { perror("fopen"); return -1; }

    EVP_MD_CTX *mdctx = EVP_MD_CTX_new();
    EVP_DigestInit_ex(mdctx, EVP_sha256(), NULL);

    uint8_t buf[8192];
    size_t n;
    while ((n = fread(buf, 1, sizeof(buf), f)) > 0) {
        EVP_DigestUpdate(mdctx, buf, n);
    }
    fclose(f);

    unsigned int md_len;
    EVP_DigestFinal_ex(mdctx, hash_out, &md_len);
    EVP_MD_CTX_free(mdctx);

    bytes_to_hex(hash_out, R3L_HASH_LEN, hex_out);
    return 0;
}

int r3l_sign_attest(r3l_edge_ctx *ctx, const char *hex_hash, uint8_t sig_out[R3L_SIG_LEN]) {
    if (!ctx->has_keypair) {
        fprintf(stderr, "No keypair loaded\n");
        return -1;
    }

    /* Build message: "R3L: attest <hex_hash>" */
    char msg[128];
    snprintf(msg, sizeof(msg), "R3L: attest %s", hex_hash);
    size_t msg_len = strlen(msg);

    /* Ed25519 sign using OpenSSL 3.x EVP API */
    EVP_PKEY *pkey = EVP_PKEY_new_raw_private_key(EVP_PKEY_ED25519, NULL, ctx->privkey, 32);
    if (!pkey) {
        fprintf(stderr, "Failed to create Ed25519 key\n");
        return -1;
    }

    EVP_MD_CTX *mdctx = EVP_MD_CTX_new();
    size_t sig_len = R3L_SIG_LEN;

    if (EVP_DigestSignInit(mdctx, NULL, NULL, NULL, pkey) != 1 ||
        EVP_DigestSign(mdctx, sig_out, &sig_len, (const uint8_t *)msg, msg_len) != 1) {
        fprintf(stderr, "Ed25519 signing failed\n");
        EVP_MD_CTX_free(mdctx);
        EVP_PKEY_free(pkey);
        return -1;
    }

    EVP_MD_CTX_free(mdctx);
    EVP_PKEY_free(pkey);
    return 0;
}

int r3l_attest(r3l_edge_ctx *ctx, const char *content_hash_hex, int has_c2pa,
               const char *tlsh_hex) {
    CURL *curl = curl_easy_init();
    if (!curl) return -1;

    /* Build URL */
    char url[512];
    snprintf(url, sizeof(url), "%s/api/edge/attest", ctx->api_url);

    /* Build JSON body */
    char body[2048];
    int off = snprintf(body, sizeof(body),
        "{\"content_hash\":\"%s\",\"has_c2pa\":%s",
        content_hash_hex,
        has_c2pa ? "true" : "false");

    /* Add wallet signature if keypair loaded */
    if (ctx->has_keypair) {
        uint8_t sig[R3L_SIG_LEN];
        if (r3l_sign_attest(ctx, content_hash_hex, sig) == 0) {
            char sig_b58[128];
            if (base58_encode(sig, R3L_SIG_LEN, sig_b58, sizeof(sig_b58)) > 0) {
                off += snprintf(body + off, sizeof(body) - off,
                    ",\"wallet_signature\":\"%s\"", sig_b58);
            }
        }
    }

    /* Add TLSH hash if provided */
    if (tlsh_hex && tlsh_hex[0]) {
        off += snprintf(body + off, sizeof(body) - off,
            ",\"tlsh_hash\":\"%s\"", tlsh_hex);
    }

    snprintf(body + off, sizeof(body) - off, "}");

    /* Headers */
    struct curl_slist *headers = NULL;
    headers = curl_slist_append(headers, "Content-Type: application/json");
    char auth[128];
    snprintf(auth, sizeof(auth), "X-API-Key: %s", ctx->api_key);
    headers = curl_slist_append(headers, auth);

    /* Response buffer */
    struct response_buf resp = {NULL, 0};

    curl_easy_setopt(curl, CURLOPT_URL, url);
    curl_easy_setopt(curl, CURLOPT_POSTFIELDS, body);
    curl_easy_setopt(curl, CURLOPT_HTTPHEADER, headers);
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, write_cb);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &resp);
    curl_easy_setopt(curl, CURLOPT_TIMEOUT, 30L);

    CURLcode res = curl_easy_perform(curl);

    long http_code = 0;
    curl_easy_getinfo(curl, CURLINFO_RESPONSE_CODE, &http_code);

    if (res != CURLE_OK) {
        fprintf(stderr, "curl error: %s\n", curl_easy_strerror(res));
    } else if (http_code >= 400) {
        fprintf(stderr, "HTTP %ld: %s\n", http_code, resp.data ? resp.data : "");
    } else {
        printf("%s\n", resp.data ? resp.data : "{}");
    }

    free(resp.data);
    curl_slist_free_all(headers);
    curl_easy_cleanup(curl);
    return (res == CURLE_OK && http_code < 400) ? 0 : -1;
}

int r3l_query(r3l_edge_ctx *ctx, const char *content_hash_hex) {
    CURL *curl = curl_easy_init();
    if (!curl) return -1;

    char url[512];
    snprintf(url, sizeof(url), "%s/api/v1/query/%s", ctx->api_url, content_hash_hex);

    struct response_buf resp = {NULL, 0};

    curl_easy_setopt(curl, CURLOPT_URL, url);
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, write_cb);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &resp);
    curl_easy_setopt(curl, CURLOPT_TIMEOUT, 30L);

    CURLcode res = curl_easy_perform(curl);

    long http_code = 0;
    curl_easy_getinfo(curl, CURLINFO_RESPONSE_CODE, &http_code);

    if (res != CURLE_OK) {
        fprintf(stderr, "curl error: %s\n", curl_easy_strerror(res));
    } else if (http_code >= 400) {
        fprintf(stderr, "HTTP %ld: %s\n", http_code, resp.data ? resp.data : "");
    } else {
        printf("%s\n", resp.data ? resp.data : "{}");
    }

    free(resp.data);
    curl_easy_cleanup(curl);
    return (res == CURLE_OK && http_code < 400) ? 0 : -1;
}
