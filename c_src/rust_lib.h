#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#define SIZE_BUFFER 27

typedef struct Ipv6AddrC {
  uint16_t segments[8];
} Ipv6AddrC;

typedef struct Ipv4AddrC {
  uint8_t segments[4];
} Ipv4AddrC;

typedef struct Buffer {
  uint8_t buffer[SIZE_BUFFER];
} Buffer;

typedef enum Message_Tag {
  Stuck,
  AddStep,
  DelStep,
  HasToSend4,
  HasToSend6,
  Send,
  Sent,
  GetTime,
  GetRand,
  WakeUp,
  Finished,
} Message_Tag;

typedef struct AddStep_Body {
  uint8_t _0;
  uint64_t _1;
} AddStep_Body;

typedef struct DelStep_Body {
  uint8_t _0;
  uint64_t _1;
} DelStep_Body;

typedef struct HasToSend4_Body {
  uint8_t _0;
  uint8_t _1;
  struct Ipv4AddrC _2;
  uint64_t _3;
} HasToSend4_Body;

typedef struct HasToSend6_Body {
  uint8_t _0;
  uint8_t _1;
  struct Ipv6AddrC _2;
  uint64_t _3;
} HasToSend6_Body;

typedef struct Sent_Body {
  uint8_t _0;
  uint64_t _1;
} Sent_Body;

typedef struct GetRand_Body {
  uint8_t _0;
  uint64_t _1;
} GetRand_Body;

typedef struct Message {
  Message_Tag tag;
  union {
    struct {
      uint8_t stuck;
    };
    AddStep_Body add_step;
    DelStep_Body del_step;
    HasToSend4_Body has_to_send4;
    HasToSend6_Body has_to_send6;
    struct {
      uint64_t send;
    };
    Sent_Body sent;
    struct {
      uint8_t get_time;
    };
    GetRand_Body get_rand;
    struct {
      uint64_t wake_up;
    };
    struct {
      uint8_t finished;
    };
  };
} Message;

struct Ipv6AddrC new(uint16_t a,
                     uint16_t b,
                     uint16_t c,
                     uint16_t d,
                     uint16_t e,
                     uint16_t f,
                     uint16_t g,
                     uint16_t h);

struct Ipv6AddrC ip6_from_str(const char *ip);

struct Ipv4AddrC ip4_from_str(const char *ip);

struct Buffer *serialize(struct Message msg);

struct Message *deserialize(struct Buffer msg);
