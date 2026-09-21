#include "sha256.h"

typedef struct Sha256 {
    uint32_t h[8];
    uint64_t bits;
    uint8_t block[64];
    unsigned int used;
} Sha256;

static const uint32_t k[64] = {
    0x428a2f98u,0x71374491u,0xb5c0fbcfu,0xe9b5dba5u,0x3956c25bu,0x59f111f1u,0x923f82a4u,0xab1c5ed5u,
    0xd807aa98u,0x12835b01u,0x243185beu,0x550c7dc3u,0x72be5d74u,0x80deb1feu,0x9bdc06a7u,0xc19bf174u,
    0xe49b69c1u,0xefbe4786u,0x0fc19dc6u,0x240ca1ccu,0x2de92c6fu,0x4a7484aau,0x5cb0a9dcu,0x76f988dau,
    0x983e5152u,0xa831c66du,0xb00327c8u,0xbf597fc7u,0xc6e00bf3u,0xd5a79147u,0x06ca6351u,0x14292967u,
    0x27b70a85u,0x2e1b2138u,0x4d2c6dfcu,0x53380d13u,0x650a7354u,0x766a0abbu,0x81c2c92eu,0x92722c85u,
    0xa2bfe8a1u,0xa81a664bu,0xc24b8b70u,0xc76c51a3u,0xd192e819u,0xd6990624u,0xf40e3585u,0x106aa070u,
    0x19a4c116u,0x1e376c08u,0x2748774cu,0x34b0bcb5u,0x391c0cb3u,0x4ed8aa4au,0x5b9cca4fu,0x682e6ff3u,
    0x748f82eeu,0x78a5636fu,0x84c87814u,0x8cc70208u,0x90befffau,0xa4506cebu,0xbef9a3f7u,0xc67178f2u
};

static uint32_t rotr(uint32_t value, unsigned int count) { return (value >> count) | (value << (32u - count)); }
static uint32_t choose(uint32_t x, uint32_t y, uint32_t z) { return (x & y) ^ (~x & z); }
static uint32_t majority(uint32_t x, uint32_t y, uint32_t z) { return (x & y) ^ (x & z) ^ (y & z); }

static void transform(Sha256 *ctx) {
    uint32_t w[64];
    uint32_t a,b,c,d,e,f,g,h,t1,t2;
    unsigned int i;
    for (i = 0; i < 16; ++i) w[i] = ((uint32_t)ctx->block[i*4] << 24) | ((uint32_t)ctx->block[i*4+1] << 16) | ((uint32_t)ctx->block[i*4+2] << 8) | ctx->block[i*4+3];
    for (; i < 64; ++i) { uint32_t s0=rotr(w[i-15],7)^rotr(w[i-15],18)^(w[i-15]>>3); uint32_t s1=rotr(w[i-2],17)^rotr(w[i-2],19)^(w[i-2]>>10); w[i]=w[i-16]+s0+w[i-7]+s1; }
    a=ctx->h[0];b=ctx->h[1];c=ctx->h[2];d=ctx->h[3];e=ctx->h[4];f=ctx->h[5];g=ctx->h[6];h=ctx->h[7];
    for (i=0;i<64;++i) { uint32_t s1=rotr(e,6)^rotr(e,11)^rotr(e,25); t1=h+s1+choose(e,f,g)+k[i]+w[i]; uint32_t s0=rotr(a,2)^rotr(a,13)^rotr(a,22); t2=s0+majority(a,b,c); h=g;g=f;f=e;e=d+t1;d=c;c=b;b=a;a=t1+t2; }
    ctx->h[0]+=a;ctx->h[1]+=b;ctx->h[2]+=c;ctx->h[3]+=d;ctx->h[4]+=e;ctx->h[5]+=f;ctx->h[6]+=g;ctx->h[7]+=h;
}

static void init(Sha256 *ctx) { static const uint32_t initial[8]={0x6a09e667u,0xbb67ae85u,0x3c6ef372u,0xa54ff53au,0x510e527fu,0x9b05688cu,0x1f83d9abu,0x5be0cd19u}; unsigned int i; for(i=0;i<8;++i)ctx->h[i]=initial[i];ctx->bits=0;ctx->used=0; }
static void update(Sha256 *ctx, const uint8_t *data, unsigned int len) { unsigned int i; for(i=0;i<len;++i){ctx->block[ctx->used++]=data[i];if(ctx->used==64){transform(ctx);ctx->bits+=512;ctx->used=0;}} }
static void final(Sha256 *ctx, uint8_t out[32]) { unsigned int i; uint64_t bits=ctx->bits+(uint64_t)ctx->used*8u;ctx->block[ctx->used++]=0x80; if(ctx->used>56){while(ctx->used<64)ctx->block[ctx->used++]=0;transform(ctx);ctx->used=0;}while(ctx->used<56)ctx->block[ctx->used++]=0;for(i=0;i<8;++i)ctx->block[63-i]=(uint8_t)(bits>>(i*8));transform(ctx);for(i=0;i<8;++i){out[i*4]=(uint8_t)(ctx->h[i]>>24);out[i*4+1]=(uint8_t)(ctx->h[i]>>16);out[i*4+2]=(uint8_t)(ctx->h[i]>>8);out[i*4+3]=(uint8_t)ctx->h[i];} }

void vita_adbd_hmac_sha256(const uint8_t *key, unsigned int key_len, const uint8_t *data, unsigned int data_len, uint8_t output[32]) {
    uint8_t inner[64], outer[64], digest[32]; Sha256 ctx; unsigned int i;
    for(i=0;i<64;++i){uint8_t v=i<key_len?key[i]:0;inner[i]=v^0x36u;outer[i]=v^0x5cu;}
    init(&ctx);update(&ctx,inner,64);update(&ctx,data,data_len);final(&ctx,digest);
    init(&ctx);update(&ctx,outer,64);update(&ctx,digest,32);final(&ctx,output);
}

int vita_adbd_constant_time_equal(const uint8_t *left, const uint8_t *right, unsigned int length) { uint8_t different=0;unsigned int i;for(i=0;i<length;++i)different|=left[i]^right[i];return different==0; }
