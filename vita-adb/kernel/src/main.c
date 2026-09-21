#include <psp2kern/kernel/threadmgr.h>
#include <psp2kern/netps.h>
#include <psp2kern/usbserial.h>
#include <stdint.h>

#include "sha256.h"

/* VAD1 is a local USB serial diagnostic transport. VAD2 is the authenticated
 * TCP transport for the Vita's trusted LAN. Neither accepts a general shell. */
#define USB_FRAME_HEADER_SIZE 8u
#define USB_MAX_PAYLOAD 1024u
#define USB_RX_CAPACITY (USB_FRAME_HEADER_SIZE + USB_MAX_PAYLOAD)
#define TCP_FRAME_HEADER_SIZE 12u
#define TCP_TAG_SIZE 32u
#define TCP_MAX_PAYLOAD 512u
#define TCP_PORT 39999u
#define OP_PING 0x01u
#define OP_INFO 0x02u
#define OP_ERROR 0xFFu

static volatile int g_running;
static volatile int g_listener = -1;
static volatile int g_client = -1;
static int g_usb_active;
static SceUID g_usb_thread = -1;
static SceUID g_tcp_thread = -1;
static uint8_t g_rx[USB_RX_CAPACITY];
static unsigned int g_rx_len;
static uint8_t g_pairing_key[32];

static uint16_t read_u16(const uint8_t *source) { return (uint16_t)source[0] | ((uint16_t)source[1] << 8); }
static uint32_t read_u32(const uint8_t *source) { return (uint32_t)source[0] | ((uint32_t)source[1] << 8) | ((uint32_t)source[2] << 16) | ((uint32_t)source[3] << 24); }
static void write_u16(uint8_t *destination, uint16_t value) { destination[0] = (uint8_t)value; destination[1] = (uint8_t)(value >> 8); }
static void write_u32(uint8_t *destination, uint32_t value) { destination[0] = (uint8_t)value; destination[1] = (uint8_t)(value >> 8); destination[2] = (uint8_t)(value >> 16); destination[3] = (uint8_t)(value >> 24); }

static int hex_value(char value) {
    if (value >= '0' && value <= '9') return value - '0';
    if (value >= 'a' && value <= 'f') return value - 'a' + 10;
    if (value >= 'A' && value <= 'F') return value - 'A' + 10;
    return -1;
}

static int load_pairing_key(void) {
    static const char encoded[] = VITA_ADBD_PAIRING_KEY_HEX;
    unsigned int index;
    for (index = 0; index < 32u; ++index) {
        int high = hex_value(encoded[index * 2u]);
        int low = hex_value(encoded[index * 2u + 1u]);
        if (high < 0 || low < 0) return -1;
        g_pairing_key[index] = (uint8_t)((high << 4) | low);
    }
    return encoded[64] == '\0' ? 0 : -1;
}

static void discard(unsigned int count) {
    unsigned int i;
    if (count >= g_rx_len) { g_rx_len = 0; return; }
    for (i = 0; i + count < g_rx_len; ++i) g_rx[i] = g_rx[i + count];
    g_rx_len -= count;
}

static int send_usb_frame(uint8_t operation, const uint8_t *payload, uint16_t payload_len) {
    uint8_t frame[USB_FRAME_HEADER_SIZE + USB_MAX_PAYLOAD];
    unsigned int i;
    SceSize written;
    if (payload_len > USB_MAX_PAYLOAD) return -1;
    frame[0] = 'V'; frame[1] = 'A'; frame[2] = 'D'; frame[3] = '1'; frame[4] = 1; frame[5] = operation;
    write_u16(&frame[6], payload_len);
    for (i = 0; i < payload_len; ++i) frame[USB_FRAME_HEADER_SIZE + i] = payload[i];
    written = ksceUsbSerialSend(frame, USB_FRAME_HEADER_SIZE + payload_len, 0, 0);
    return written == (SceSize)(USB_FRAME_HEADER_SIZE + payload_len) ? 0 : -1;
}

static void usb_reply_error(uint8_t rejected_operation) { const uint8_t payload[] = { rejected_operation, 1 }; (void)send_usb_frame(OP_ERROR, payload, sizeof(payload)); }

static void handle_usb_frame(uint8_t operation) {
    static const uint8_t pong[] = { 1, 0, 0, 0 };
    static const uint8_t info[] = "vita-adbd/0.2 usb restricted";
    if (operation == OP_PING) (void)send_usb_frame((uint8_t)(OP_PING | 0x80u), pong, sizeof(pong));
    else if (operation == OP_INFO) (void)send_usb_frame((uint8_t)(OP_INFO | 0x80u), info, sizeof(info) - 1u);
    else usb_reply_error(operation);
}

static void parse_usb_frames(void) {
    while (g_rx_len >= USB_FRAME_HEADER_SIZE) {
        uint16_t payload_len;
        unsigned int frame_len;
        if (g_rx[0] != 'V' || g_rx[1] != 'A' || g_rx[2] != 'D' || g_rx[3] != '1') { discard(1); continue; }
        if (g_rx[4] != 1) { usb_reply_error(g_rx[5]); discard(USB_FRAME_HEADER_SIZE); continue; }
        payload_len = read_u16(&g_rx[6]);
        if (payload_len > USB_MAX_PAYLOAD) { usb_reply_error(g_rx[5]); discard(USB_FRAME_HEADER_SIZE); continue; }
        frame_len = USB_FRAME_HEADER_SIZE + payload_len;
        if (g_rx_len < frame_len) return;
        handle_usb_frame(g_rx[5]);
        discard(frame_len);
    }
}

static int usb_worker(SceSize args, void *argp) {
    (void)args; (void)argp;
    while (g_running && g_usb_active) {
        unsigned int available = ksceUsbSerialGetRecvBufferSize();
        if (ksceUsbSerialStatus() && available && g_rx_len < USB_RX_CAPACITY) {
            unsigned int free_space = USB_RX_CAPACITY - g_rx_len;
            unsigned int wanted = available < free_space ? available : free_space;
            SceSize received = ksceUsbSerialRecv(&g_rx[g_rx_len], wanted, 0, 0);
            if (received > 0) { g_rx_len += (unsigned int)received; parse_usb_frames(); }
        }
        ksceKernelDelayThread(5 * 1000);
    }
    return 0;
}

static int recv_all(int socket, uint8_t *buffer, unsigned int length) {
    unsigned int received = 0;
    while (received < length && g_running) {
        int result = ksceNetRecv(socket, &buffer[received], length - received, 0);
        if (result <= 0) return -1;
        received += (unsigned int)result;
    }
    return received == length ? 0 : -1;
}

static int send_all(int socket, const uint8_t *buffer, unsigned int length) {
    unsigned int sent = 0;
    while (sent < length && g_running) {
        int result = ksceNetSend(socket, &buffer[sent], length - sent, 0);
        if (result <= 0) return -1;
        sent += (unsigned int)result;
    }
    return sent == length ? 0 : -1;
}

static int send_tcp_frame(int socket, uint8_t operation, uint32_t sequence, const uint8_t *payload, uint16_t payload_len) {
    uint8_t frame[TCP_FRAME_HEADER_SIZE + TCP_MAX_PAYLOAD + TCP_TAG_SIZE];
    unsigned int frame_len = TCP_FRAME_HEADER_SIZE + payload_len;
    unsigned int index;
    if (payload_len > TCP_MAX_PAYLOAD) return -1;
    frame[0] = 'V'; frame[1] = 'A'; frame[2] = 'D'; frame[3] = '2'; frame[4] = 1; frame[5] = operation;
    write_u32(&frame[6], sequence); write_u16(&frame[10], payload_len);
    for (index = 0; index < payload_len; ++index) frame[TCP_FRAME_HEADER_SIZE + index] = payload[index];
    vita_adbd_hmac_sha256(g_pairing_key, sizeof(g_pairing_key), frame, frame_len, &frame[frame_len]);
    return send_all(socket, frame, frame_len + TCP_TAG_SIZE);
}

static int valid_tcp_request(const uint8_t *frame, unsigned int frame_len, const uint8_t *tag) {
    uint8_t expected[TCP_TAG_SIZE];
    vita_adbd_hmac_sha256(g_pairing_key, sizeof(g_pairing_key), frame, frame_len, expected);
    return vita_adbd_constant_time_equal(expected, tag, sizeof(expected));
}

static void serve_tcp_client(int socket) {
    uint8_t frame[TCP_FRAME_HEADER_SIZE + TCP_MAX_PAYLOAD];
    uint8_t tag[TCP_TAG_SIZE];
    uint16_t payload_len;
    uint32_t sequence;
    static const uint8_t pong[] = { 2, 0, 0, 0 };
    static const uint8_t info[] = "vita-adbd/0.2 tcp hmac restricted";
    if (recv_all(socket, frame, TCP_FRAME_HEADER_SIZE) < 0) return;
    if (frame[0] != 'V' || frame[1] != 'A' || frame[2] != 'D' || frame[3] != '2' || frame[4] != 1) return;
    payload_len = read_u16(&frame[10]);
    if (payload_len > TCP_MAX_PAYLOAD) return;
    if (recv_all(socket, &frame[TCP_FRAME_HEADER_SIZE], payload_len) < 0 || recv_all(socket, tag, sizeof(tag)) < 0) return;
    if (!valid_tcp_request(frame, TCP_FRAME_HEADER_SIZE + payload_len, tag)) return;
    sequence = read_u32(&frame[6]);
    if (frame[5] == OP_PING) (void)send_tcp_frame(socket, (uint8_t)(OP_PING | 0x80u), sequence, pong, sizeof(pong));
    else if (frame[5] == OP_INFO) (void)send_tcp_frame(socket, (uint8_t)(OP_INFO | 0x80u), sequence, info, sizeof(info) - 1u);
    else { const uint8_t error[] = { frame[5], 1 }; (void)send_tcp_frame(socket, OP_ERROR, sequence, error, sizeof(error)); }
}

static int open_listener(void) {
    SceNetSockaddrIn address = {0};
    int socket = ksceNetSocket("vita-adbd", SCE_NET_AF_INET, SCE_NET_SOCK_STREAM, 0);
    if (socket < 0) return -1;
    address.sin_len = sizeof(address); address.sin_family = SCE_NET_AF_INET; address.sin_port = ksceNetHtons(TCP_PORT); address.sin_addr.s_addr = SCE_NET_INADDR_ANY;
    if (ksceNetBind(socket, (const SceNetSockaddr *)&address, sizeof(address)) < 0 || ksceNetListen(socket, 1) < 0) { ksceNetClose(socket); return -1; }
    return socket;
}

static int tcp_worker(SceSize args, void *argp) {
    (void)args; (void)argp;
    while (g_running) {
        SceNetSockaddrIn peer = {0}; unsigned int peer_len = sizeof(peer); int client;
        if (g_listener < 0) { g_listener = open_listener(); if (g_listener < 0) { ksceKernelDelayThread(1000 * 1000); continue; } }
        client = ksceNetAccept(g_listener, (SceNetSockaddr *)&peer, &peer_len);
        if (client < 0) { if (g_running) ksceKernelDelayThread(100 * 1000); continue; }
        g_client = client;
        serve_tcp_client(client);
        if (g_client == client) { ksceNetClose(client); g_client = -1; }
    }
    return 0;
}

int _start(SceSize args, void *argp) __attribute__((weak, alias("module_start")));

int module_start(SceSize args, void *argp) {
    int result;
    (void)args; (void)argp;
    if (load_pairing_key() < 0) return -1;
    g_running = 1;
    g_tcp_thread = ksceKernelCreateThread("vita-adbd-tcp", tcp_worker, 0x10000100, 0x4000, 0, 0, 0);
    if (g_tcp_thread < 0 || ksceKernelStartThread(g_tcp_thread, 0, 0) < 0) { g_running = 0; return -1; }
    result = ksceUsbSerialStart();
    if (result >= 0 && ksceUsbSerialSetup(0) >= 0) {
        g_usb_active = 1;
        g_usb_thread = ksceKernelCreateThread("vita-adbd-usb", usb_worker, 0x10000100, 0x4000, 0, 0, 0);
        if (g_usb_thread < 0 || ksceKernelStartThread(g_usb_thread, 0, 0) < 0) { g_usb_active = 0; ksceUsbSerialClose(); }
    }
    return 0;
}

int module_stop(SceSize args, void *argp) {
    int client;
    int listener;
    (void)args; (void)argp; g_running = 0;
    client = g_client; g_client = -1;
    listener = g_listener; g_listener = -1;
    if (client >= 0) ksceNetClose(client);
    if (listener >= 0) ksceNetClose(listener);
    if (g_usb_active) ksceUsbSerialClose();
    return 0;
}
