/*
 * R3L Edge CLI — minimal C binary.
 *
 * Usage:
 *   r3l-edge attest  <file>  [--api-key KEY] [--keypair PATH] [--api URL]
 *   r3l-edge query   <hash>  [--api URL]
 *   r3l-edge hash    <file>
 *
 * Compile:
 *   gcc -O2 -o r3l-edge r3l_edge.c main.c -lcurl -lcrypto
 */
#include "r3l_edge.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void usage(void) {
    fprintf(stderr,
        "R3L Edge Client (C)\n"
        "\n"
        "Usage:\n"
        "  r3l-edge hash   <file>                          Hash a file (SHA-256)\n"
        "  r3l-edge attest <file> --api-key KEY [OPTIONS]   Hash + sign + submit\n"
        "  r3l-edge query  <hash> [--api URL]               Query trust verdict\n"
        "\n"
        "Options:\n"
        "  --api URL        API base URL (default: http://localhost:3001)\n"
        "  --api-key KEY    API key from registration\n"
        "  --keypair PATH   Ed25519 keypair JSON file\n"
        "\n"
        "Environment:\n"
        "  R3L_API_URL, R3L_API_KEY, R3L_KEYPAIR\n"
    );
}

static const char *env_or(const char *name, const char *def) {
    const char *v = getenv(name);
    return (v && *v) ? v : def;
}

static const char *find_arg(int argc, char **argv, const char *flag) {
    for (int i = 0; i < argc - 1; i++) {
        if (strcmp(argv[i], flag) == 0)
            return argv[i + 1];
    }
    return NULL;
}

int main(int argc, char **argv) {
    if (argc < 3) {
        usage();
        return 1;
    }

    const char *cmd = argv[1];
    const char *target = argv[2];

    /* ── hash: just hash a file ──────────────────────────────── */
    if (strcmp(cmd, "hash") == 0) {
        uint8_t hash[R3L_HASH_LEN];
        char hex[R3L_HEX_HASH_LEN];
        if (r3l_hash_file(target, hash, hex) != 0) return 1;
        printf("%s  %s\n", hex, target);
        return 0;
    }

    /* ── query: query trust verdict ──────────────────────────── */
    if (strcmp(cmd, "query") == 0) {
        const char *api = find_arg(argc, argv, "--api");
        if (!api) api = env_or("R3L_API_URL", "http://localhost:3001");

        r3l_edge_ctx ctx;
        r3l_init(&ctx, api, NULL);
        return r3l_query(&ctx, target) == 0 ? 0 : 1;
    }

    /* ── attest: hash + sign + submit ────────────────────────── */
    if (strcmp(cmd, "attest") == 0) {
        const char *api = find_arg(argc, argv, "--api");
        if (!api) api = env_or("R3L_API_URL", "http://localhost:3001");

        const char *api_key = find_arg(argc, argv, "--api-key");
        if (!api_key) api_key = env_or("R3L_API_KEY", "");
        if (!*api_key) {
            fprintf(stderr, "Error: --api-key or R3L_API_KEY required\n");
            return 1;
        }

        r3l_edge_ctx ctx;
        r3l_init(&ctx, api, api_key);

        /* Load keypair if available */
        const char *kp = find_arg(argc, argv, "--keypair");
        if (!kp) kp = env_or("R3L_KEYPAIR", "edge-keypair.json");
        r3l_load_keypair(&ctx, kp);  /* OK if this fails — wallet sig is optional */

        /* Hash the file */
        uint8_t hash[R3L_HASH_LEN];
        char hex[R3L_HEX_HASH_LEN];
        if (r3l_hash_file(target, hash, hex) != 0) {
            fprintf(stderr, "Failed to hash file: %s\n", target);
            return 1;
        }
        fprintf(stderr, "Content hash: %s\n", hex);

        /* Submit (has_c2pa = false for IoT devices, no TLSH for C client) */
        return r3l_attest(&ctx, hex, 0, NULL) == 0 ? 0 : 1;
    }

    usage();
    return 1;
}
