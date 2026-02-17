#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * The fixed size of the buffer used for IPC messages via Posix Message Queues.
 */
#define SIZE_BUFFER 27

/**
 * A C-compatible representation of an IPv6 address.
 */
typedef struct Ipv6AddrC {
  /**
   * The eight 16-bit segments of the IPv6 address.
   */
  uint16_t segments[8];
} Ipv6AddrC;

/**
 * A C-compatible representation of an IPv4 address.
 */
typedef struct Ipv4AddrC {
  /**
   * The four octets of the IPv4 address.
   */
  uint8_t segments[4];
} Ipv4AddrC;

/**
 * A fixed-size buffer for IPC messages.
 */
typedef struct Buffer {
  uint8_t buffer[SIZE_BUFFER];
} Buffer;

/**
 * Messages exchanged between the simulator (leader) and the intercepted apps (followers).
 */
typedef enum Message_Tag {
  /**
   * App is blocked on an operation.
   */
  Stuck,
  /**
   * App is requesting to advance time or perform a step.
   */
  AddStep,
  /**
   * App is cancelling a previously requested step.
   */
  DelStep,
  /**
   * App has an IPv4 packet to send.
   */
  HasToSend4,
  /**
   * App has an IPv6 packet to send.
   */
  HasToSend6,
  /**
   * Simulator authorizing an app to send a packet.
   */
  Send,
  /**
   * App confirming a packet has been sent.
   */
  Sent,
  /**
   * App requesting the current simulation time.
   */
  GetTime,
  /**
   * App requesting a random seed from the simulator.
   */
  GetRand,
  /**
   * Simulator waking up an app at a specific time.
   */
  WakeUp,
  /**
   * App has finished its execution.
   */
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

/**
 * Creates a new `Ipv6AddrC` from eight 16-bit segments.
 * This function is exported to C.
 */
struct Ipv6AddrC new(uint16_t a,
                     uint16_t b,
                     uint16_t c,
                     uint16_t d,
                     uint16_t e,
                     uint16_t f,
                     uint16_t g,
                     uint16_t h);

/**
 * Converts a C string to an `Ipv6AddrC`.
 *
 * # Safety
 * This function is unsafe because it dereferences a raw pointer.
 */
struct Ipv6AddrC ip6_from_str(const char *ip);

/**
 * Converts a C string to an `Ipv4AddrC`.
 *
 * # Safety
 * This function is unsafe because it dereferences a raw pointer.
 */
struct Ipv4AddrC ip4_from_str(const char *ip);

/**
 * Serializes a `Message` and returns a pointer to a `Buffer`.
 * The caller is responsible for freeing the memory.
 */
struct Buffer *serialize(struct Message msg);

/**
 * Deserializes a `Buffer` and returns a pointer to a `Message`.
 * The caller is responsible for freeing the memory.
 */
struct Message *deserialize(struct Buffer msg);
