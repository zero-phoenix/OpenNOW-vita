#include <psp2kern/kernel/threadmgr.h>
#include <psp2kern/usbserial.h>
#include <stdint.h>

#define FRAME_HEADER_SIZE 8u
#define MAX_PAYLOAD 1024u
#define RX_CAPACITY (FRAME_HEADER_SIZE + MAX_PAYLOAD)

#define OP_PING 0x01u
#define OP_INFO 0x02u
#define OP_ERROR 0xFFu

static volatile int g_running;
static SceUID g_thread = -1;
static uint8_t g_rx[RX_CAPACITY];
static unsigned int g_rx_len;

static uint16_t read_u16(const uint8_t *source) {
    return (uint16_t)source[0] | ((uint16_t)source[1] << 8);
}

static void discard(unsigned int count) {
    unsigned int i;
    if (count >= g_rx_len) {
        g_rx_len = 0;
        return;
    }
    for (i = 0; i + count < g_rx_len; ++i) {
        g_rx[i] = g_rx[i + count];
    }
    g_rx_len -= count;
}

static int send_frame(uint8_t operation, const uint8_t *payload, uint16_t payload_len) {
    uint8_t frame[FRAME_HEADER_SIZE + MAX_PAYLOAD];
    unsigned int i;
    SceSize written;

    if (payload_len > MAX_PAYLOAD) {
        return -1;
    }
    frame[0] = 'V';
    frame[1] = 'A';
    frame[2] = 'D';
    frame[3] = '1';
    frame[4] = 1;
    frame[5] = operation;
    frame[6] = (uint8_t)(payload_len & 0xffu);
    frame[7] = (uint8_t)(payload_len >> 8);
    for (i = 0; i < payload_len; ++i) {
        frame[FRAME_HEADER_SIZE + i] = payload[i];
    }
    written = ksceUsbSerialSend(frame, FRAME_HEADER_SIZE + payload_len, 0, 0);
    return written == (SceSize)(FRAME_HEADER_SIZE + payload_len) ? 0 : -1;
}

static void reply_error(uint8_t rejected_operation) {
    const uint8_t payload[] = { rejected_operation, 1 };
    (void)send_frame(OP_ERROR, payload, sizeof(payload));
}

static void handle_frame(uint8_t operation) {
    static const uint8_t pong[] = { 1, 0, 0, 0 };
    static const uint8_t info[] = "vita-adb/0.1 usbserial restricted";

    if (operation == OP_PING) {
        (void)send_frame((uint8_t)(OP_PING | 0x80u), pong, sizeof(pong));
    } else if (operation == OP_INFO) {
        (void)send_frame((uint8_t)(OP_INFO | 0x80u), info, sizeof(info) - 1u);
    } else {
        reply_error(operation);
    }
}

static void parse_frames(void) {
    while (g_rx_len >= FRAME_HEADER_SIZE) {
        uint16_t payload_len;
        unsigned int frame_len;
        if (g_rx[0] != 'V' || g_rx[1] != 'A' || g_rx[2] != 'D' || g_rx[3] != '1') {
            discard(1);
            continue;
        }
        if (g_rx[4] != 1) {
            reply_error(g_rx[5]);
            discard(FRAME_HEADER_SIZE);
            continue;
        }
        payload_len = read_u16(&g_rx[6]);
        if (payload_len > MAX_PAYLOAD) {
            reply_error(g_rx[5]);
            discard(FRAME_HEADER_SIZE);
            continue;
        }
        frame_len = FRAME_HEADER_SIZE + payload_len;
        if (g_rx_len < frame_len) {
            return;
        }
        handle_frame(g_rx[5]);
        discard(frame_len);
    }
}

static int usb_worker(SceSize args, void *argp) {
    (void)args;
    (void)argp;
    while (g_running) {
        unsigned int available = ksceUsbSerialGetRecvBufferSize();
        if (ksceUsbSerialStatus() && available && g_rx_len < RX_CAPACITY) {
            unsigned int free_space = RX_CAPACITY - g_rx_len;
            unsigned int wanted = available < free_space ? available : free_space;
            SceSize received = ksceUsbSerialRecv(&g_rx[g_rx_len], wanted, 0, 0);
            if (received > 0) {
                g_rx_len += (unsigned int)received;
                parse_frames();
            }
        }
        ksceKernelDelayThread(5 * 1000);
    }
    return 0;
}

int _start(SceSize args, void *argp) __attribute__((weak, alias("module_start")));

int module_start(SceSize args, void *argp) {
    int result;
    (void)args;
    (void)argp;

    result = ksceUsbSerialStart();
    if (result < 0) {
        return -1;
    }
    result = ksceUsbSerialSetup(0);
    if (result < 0) {
        ksceUsbSerialClose();
        return -1;
    }
    g_running = 1;
    g_thread = ksceKernelCreateThread("vita-adb-usb", usb_worker, 0x10000100, 0x4000, 0, 0, 0);
    if (g_thread < 0 || ksceKernelStartThread(g_thread, 0, 0) < 0) {
        g_running = 0;
        ksceUsbSerialClose();
        return -1;
    }
    return 0;
}

int module_stop(SceSize args, void *argp) {
    (void)args;
    (void)argp;
    g_running = 0;
    ksceUsbSerialClose();
    return 0;
}
