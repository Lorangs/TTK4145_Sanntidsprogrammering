#pragma once

void check_if_buttons_pressed(void);

void elevator_order_executed(int level);

void floor_order_executed(int level, int direction);

int find_order_to_execute(void);

int look_for_order_on_the_way(int current_floor, int end_floor);

void execut_a_order(int current_floor, int end_floor, int stop_at);

void clear_que();