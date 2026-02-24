CC      ?= gcc
CFLAGS  ?= -O2 -Wall -Wextra

# OpenSSL / libcurl paths (adjust for your platform)
OPENSSL_PREFIX ?= $(shell brew --prefix openssl 2>/dev/null || echo /usr)
CURL_PREFIX    ?= $(shell brew --prefix curl 2>/dev/null || echo /usr)

CFLAGS  += -I$(OPENSSL_PREFIX)/include -I$(CURL_PREFIX)/include
LDFLAGS ?= -L$(OPENSSL_PREFIX)/lib -L$(CURL_PREFIX)/lib -lcurl -lcrypto

TARGET  = r3l-edge

SRCS    = r3l_edge.c main.c
OBJS    = $(SRCS:.c=.o)

all: $(TARGET)

$(TARGET): $(OBJS)
	$(CC) $(CFLAGS) -o $@ $^ $(LDFLAGS)

%.o: %.c r3l_edge.h
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -f $(OBJS) $(TARGET)

.PHONY: all clean
