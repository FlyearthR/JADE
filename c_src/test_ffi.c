#include "rust_lib.h"
#include <CUnit/Basic.h>
#include <stdio.h>


Message stuck_1 = {.tag = Stuck, .stuck = 1};
Message stuck_42 = {.tag = Stuck, .stuck = 42};

Message addStep_1_10 = {.tag = AddStep, .add_step = {._0 = 1, ._1 = 10}};
Message addStep_2_10 = {.tag = AddStep, .add_step = {._0 = 2, ._1 = 10}};
Message addStep_1_20 = {.tag = AddStep, .add_step = {._0 = 1, ._1 = 20}};

Message delStep_1_10 = {.tag = DelStep, .del_step = {._0 = 1, ._1 = 10}};
Message delStep_2_10 = {.tag = DelStep, .del_step = {._0 = 2, ._1 = 10}};
Message delStep_1_20 = {.tag = DelStep, .del_step = {._0 = 1, ._1 = 20}};

Message hasToSend4_1_2_10_0_0_20_30 = {.tag = HasToSend4, .has_to_send4 = {
    ._0 = 1, ._1 = 2, ._2 = {.segments = {10, 0, 0, 20}}, ._3 = 30
}};
Message hasToSend4_2_2_10_0_0_20_30 = {.tag = HasToSend4, .has_to_send4 = {
    ._0 = 2, ._1 = 2, ._2 = {.segments = {10, 0, 0, 20}}, ._3 = 30
}};
Message hasToSend4_1_3_10_0_0_20_30 = {.tag = HasToSend4, .has_to_send4 = {
    ._0 = 1, ._1 = 3, ._2 = {.segments = {10, 0, 0, 20}}, ._3 = 30
}};
Message hasToSend4_1_2_10_0_0_25_30 = {.tag = HasToSend4, .has_to_send4 = {
    ._0 = 1, ._1 = 2, ._2 = {.segments = {10, 0, 0, 25}}, ._3 = 30
}};
Message hasToSend4_1_2_10_0_0_20_40 = {.tag = HasToSend4, .has_to_send4 = {
    ._0 = 1, ._1 = 2, ._2 = {.segments = {10, 0, 0, 20}}, ._3 = 40
}};

Message hasToSend6_1_2_fe80_20_30 = {.tag = HasToSend6, .has_to_send6 = {
    ._0 = 1, ._1 = 2, ._2 = {.segments = {0xfe80, 0, 0, 0, 0, 0, 0, 20}}, ._3 = 30
}};
Message hasToSend6_2_2_fe80_20_30 = {.tag = HasToSend6, .has_to_send6 = {
    ._0 = 2, ._1 = 2, ._2 = {.segments = {0xfe80, 0, 0, 0, 0, 0, 0, 20}}, ._3 = 30
}};
Message hasToSend6_1_3_fe80_20_30 = {.tag = HasToSend6, .has_to_send6 = {
    ._0 = 1, ._1 = 3, ._2 = {.segments = {0xfe80, 0, 0, 0, 0, 0, 0, 20}}, ._3 = 30
}};
Message hasToSend6_1_2_fe80_25_30 = {.tag = HasToSend6, .has_to_send6 = {
    ._0 = 1, ._1 = 2, ._2 = {.segments = {0xfe80, 0, 0, 0, 0, 0, 0, 25}}, ._3 = 30
}};
Message hasToSend6_1_2_fe80_20_40 = {.tag = HasToSend6, .has_to_send6 = {
    ._0 = 1, ._1 = 2, ._2 = {.segments = {0xfe80, 0, 0, 0, 0, 0, 0, 20}}, ._3 = 40
}};

Message send_1 = {.tag = Send, .send = 1};
Message send_42 = {.tag = Send, .send = 42};

Message sent_1_10 = {.tag = Sent, .sent = {._0 = 1, ._1 = 10}};
Message sent_2_10 = {.tag = Sent, .sent = {._0 = 2, ._1 = 10}};
Message sent_1_20 = {.tag = Sent, .sent = {._0 = 1, ._1 = 20}};

Message getTime_1 = {.tag = GetTime, .get_time = 1};
Message getTime_42 = {.tag = GetTime, .get_time = 42};

Message getRand_1 = {.tag = GetRand, .get_rand = {._0 = 1, ._1 = 10}};
Message getRand_42 = {.tag = GetRand, .get_rand = {._0 = 2, ._1 = 10}};
Message getRand_42 = {.tag = GetRand, .get_rand = {._0 = 1, ._1 = 20}};

Message wakeUp_1 = {.tag = WakeUp, .wake_up = 1};
Message wakeUp_42 = {.tag = WakeUp, .wake_up = 42};

Message finished_1 = {.tag = Finished, .finished = 1};
Message finished_42 = {.tag = Finished, .finished = 42};

void print_buffer(uint8_t buffer[SIZE_BUFFER])
{
    for (int i = 0 ; i < SIZE_BUFFER ; i++)
    {
        fprintf(stderr, "%3i ", buffer[i]);
    }
    fprintf(stderr, "\n");
}

void test_ser_stuck()
{
    Buffer* b1 = serialize(stuck_1);
    Buffer* b2 = serialize(stuck_1);
    Buffer* b3 = serialize(stuck_42);

    CU_ASSERT_FALSE(memcmp(b1, b2, SIZE_BUFFER));

    CU_ASSERT(memcmp(b1, b3, SIZE_BUFFER));
}

void test_ser_add_step()
{
    Buffer* b1 = serialize(addStep_1_10);
    Buffer* b2 = serialize(addStep_1_10);
    Buffer* b3 = serialize(addStep_1_20);
    Buffer* b4 = serialize(addStep_2_10);

    CU_ASSERT_FALSE(memcmp(b1->buffer, b2->buffer, SIZE_BUFFER));
    
    CU_ASSERT(memcmp(b1->buffer, b3->buffer, SIZE_BUFFER));
    CU_ASSERT(memcmp(b1->buffer, b4->buffer, SIZE_BUFFER));
}

void test_ser_del_step()
{
    Buffer* b1 = serialize(delStep_1_10);
    Buffer* b2 = serialize(delStep_1_10);
    Buffer* b3 = serialize(delStep_1_20);
    Buffer* b4 = serialize(delStep_2_10);

    CU_ASSERT_FALSE(memcmp(b1->buffer, b2->buffer, SIZE_BUFFER));
    
    CU_ASSERT(memcmp(b1->buffer, b3->buffer, SIZE_BUFFER));
    CU_ASSERT(memcmp(b1->buffer, b4->buffer, SIZE_BUFFER));
}

void test_ser_has_to_send_4()
{
    Buffer* b1 = serialize(hasToSend4_1_2_10_0_0_20_30);
    Buffer* b2 = serialize(hasToSend4_1_2_10_0_0_20_30);
    Buffer* b3 = serialize(hasToSend4_2_2_10_0_0_20_30);
    Buffer* b4 = serialize(hasToSend4_1_3_10_0_0_20_30);
    Buffer* b5 = serialize(hasToSend4_1_2_10_0_0_25_30);
    Buffer* b6 = serialize(hasToSend4_1_2_10_0_0_20_40);

    CU_ASSERT_FALSE(memcmp(b1->buffer, b2->buffer, SIZE_BUFFER));
    
    CU_ASSERT(memcmp(b1->buffer, b3->buffer, SIZE_BUFFER));
    CU_ASSERT(memcmp(b1->buffer, b4->buffer, SIZE_BUFFER));
    CU_ASSERT(memcmp(b1->buffer, b5->buffer, SIZE_BUFFER));
    CU_ASSERT(memcmp(b1->buffer, b6->buffer, SIZE_BUFFER));
}

void test_ser_has_to_send_6()
{
    Buffer* b1 = serialize(hasToSend6_1_2_fe80_20_30);
    Buffer* b2 = serialize(hasToSend6_1_2_fe80_20_30);
    Buffer* b3 = serialize(hasToSend6_2_2_fe80_20_30);
    Buffer* b4 = serialize(hasToSend6_1_3_fe80_20_30);
    Buffer* b5 = serialize(hasToSend6_1_2_fe80_25_30);
    Buffer* b6 = serialize(hasToSend6_1_2_fe80_20_40);

    fprintf(stderr, "b1: %p; b2: %p; b3: %p\n", b1, b2, b3);
    print_buffer(b1->buffer);
    print_buffer(b2->buffer);
    print_buffer(b3->buffer);
    print_buffer(b4->buffer);
    print_buffer(b5->buffer);
    print_buffer(b6->buffer);


    CU_ASSERT_FALSE(memcmp(b1->buffer, b2->buffer, SIZE_BUFFER));
    
    CU_ASSERT(memcmp(b1->buffer, b3->buffer, SIZE_BUFFER));
    CU_ASSERT(memcmp(b1->buffer, b4->buffer, SIZE_BUFFER));
    CU_ASSERT(memcmp(b1->buffer, b5->buffer, SIZE_BUFFER));
    CU_ASSERT(memcmp(b1->buffer, b6->buffer, SIZE_BUFFER));
}

void test_ser_send()
{
    Buffer* b1 = serialize(send_1);
    Buffer* b2 = serialize(send_1);
    Buffer* b3 = serialize(send_42);

    CU_ASSERT_FALSE(memcmp(b1->buffer, b2->buffer, SIZE_BUFFER));

    CU_ASSERT(memcmp(b1->buffer, b3->buffer, SIZE_BUFFER));
}

void test_ser_sent()
{
    Buffer* b1 = serialize(sent_1_10);
    Buffer* b2 = serialize(sent_1_10);
    Buffer* b3 = serialize(sent_1_20);
    Buffer* b4 = serialize(sent_2_10);

    CU_ASSERT_FALSE(memcmp(b1->buffer, b2->buffer, SIZE_BUFFER));
    
    CU_ASSERT(memcmp(b1->buffer, b3->buffer, SIZE_BUFFER));
    CU_ASSERT(memcmp(b1->buffer, b4->buffer, SIZE_BUFFER));
}

void test_ser_get_time()
{
    Buffer* b1 = serialize(getTime_1);
    Buffer* b2 = serialize(getTime_1);
    Buffer* b3 = serialize(getTime_42);

    CU_ASSERT_FALSE(memcmp(b1->buffer, b2->buffer, SIZE_BUFFER));

    CU_ASSERT(memcmp(b1->buffer, b3->buffer, SIZE_BUFFER));
}

void test_ser_get_rand()
{
    Buffer* b1 = serialize(getRand_1);
    Buffer* b2 = serialize(getRand_1);
    Buffer* b3 = serialize(getRand_42);

    CU_ASSERT_FALSE(memcmp(b1->buffer, b2->buffer, SIZE_BUFFER));

    CU_ASSERT(memcmp(b1->buffer, b3->buffer, SIZE_BUFFER));
}

void test_ser_wake_up()
{
    Buffer* b1 = serialize(wakeUp_1);
    Buffer* b2 = serialize(wakeUp_1);
    Buffer* b3 = serialize(wakeUp_42);

    CU_ASSERT_FALSE(memcmp(b1->buffer, b2->buffer, SIZE_BUFFER));

    CU_ASSERT(memcmp(b1->buffer, b3->buffer, SIZE_BUFFER));
}

void test_ser_finished()
{
    Buffer* b1 = serialize(finished_1);
    Buffer* b2 = serialize(finished_1);
    Buffer* b3 = serialize(finished_42);

    CU_ASSERT_FALSE(memcmp(b1->buffer, b2->buffer, SIZE_BUFFER));

    CU_ASSERT(memcmp(b1->buffer, b3->buffer, SIZE_BUFFER));
}

/*void test_ser_deser()
{
    Message ret = deserialize(serialize(stuck_42));
    CU_ASSERT_FALSE(memcmp((void*) &stuck_42, (void*) &ret, sizeof(Message)));
}*/

int main()
{
   CU_pSuite pSuite = NULL;

   if (CUE_SUCCESS != CU_initialize_registry())
      return CU_get_error();

   pSuite = CU_add_suite("Serialization", NULL, NULL);
   if (NULL == pSuite) {
      CU_cleanup_registry();
      return CU_get_error();
   }

   if ((NULL == CU_add_test(pSuite, "test of stuck", test_ser_stuck)) || 
       (NULL == CU_add_test(pSuite, "test of add_step", test_ser_add_step)) ||
       (NULL == CU_add_test(pSuite, "test of del_step", test_ser_del_step)) ||
       (NULL == CU_add_test(pSuite, "test of has_to_send4", test_ser_has_to_send_4)) ||
       (NULL == CU_add_test(pSuite, "test of has_to_send6", test_ser_has_to_send_6)) ||
       (NULL == CU_add_test(pSuite, "test of send", test_ser_send)) ||
       (NULL == CU_add_test(pSuite, "test of sent", test_ser_sent)) ||
       (NULL == CU_add_test(pSuite, "test of get_time", test_ser_get_time)) ||
       (NULL == CU_add_test(pSuite, "test of get_rand", test_ser_get_rand)) ||
       (NULL == CU_add_test(pSuite, "test of wake_up", test_ser_wake_up)) ||
       (NULL == CU_add_test(pSuite, "test of finished", test_ser_finished)) )
   {
      CU_cleanup_registry();
      return CU_get_error();
   }

   CU_basic_set_mode(CU_BRM_VERBOSE);
   CU_basic_run_tests();
   CU_cleanup_registry();
   return CU_get_error();
}