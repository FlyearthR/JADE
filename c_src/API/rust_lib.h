#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct Buffer {
  uint8_t buffer[10];
} Buffer;

typedef enum Message_Tag {
  Progressed,
  Stuck,
  AddStep,
  DelStep,
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
      uint8_t get_rand;
    };
    struct {
      uint64_t wake_up;
    };
    struct {
      uint8_t finished;
    };
  };
} Message;

struct Buffer serialize(struct Message msg);

struct Message deserialize(struct Buffer msg);
