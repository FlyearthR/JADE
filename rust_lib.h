#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct Buffer {
  const uint8_t *buffer;
  uint64_t len;
} Buffer;

typedef enum Message_Tag {
  Progressed,
  Stuck,
  AddStep,
  DelStep,
  GetTime,
  WakeUp,
} Message_Tag;

typedef struct AddStep_Body {
  uint8_t _0;
  uint64_t _1;
} AddStep_Body;

typedef struct DelStep_Body {
  uint8_t _0;
  uint64_t _1;
} DelStep_Body;

typedef struct Message {
  Message_Tag tag;
  union {
    struct {
      uint8_t progressed;
    };
    struct {
      uint8_t stuck;
    };
    AddStep_Body add_step;
    DelStep_Body del_step;
    struct {
      uint8_t get_time;
    };
    struct {
      uint64_t wake_up;
    };
  };
} Message;

struct Buffer serialize(struct Message msg);

uint64_t deserialize_u64(const uint8_t (*msg)[8]);
