#include "runtime.h"
#include <math.h>
#include <stdint.h>
#include <stdlib.h>

double rt_math_pi(void) { return 3.14159265358979323846; }
double rt_math_e(void) { return 2.71828182845904523536; }
double rt_math_tau(void) { return 6.28318530717958647692; }

double rt_math_sin(double value) { return sin(value); }
double rt_math_cos(double value) { return cos(value); }
double rt_math_tan(double value) { return tan(value); }
double rt_math_sqrt(double value) { return sqrt(value); }
double rt_math_pow(double base, double exponent) { return pow(base, exponent); }
double rt_math_log(double value) { return log(value); }
double rt_math_log10(double value) { return log10(value); }
double rt_math_exp(double value) { return exp(value); }
double rt_math_atan2(double y, double x) { return atan2(y, x); }
double rt_math_asin(double value) { return asin(value); }
double rt_math_acos(double value) { return acos(value); }

int64_t rt_math_round(double value) { return (int64_t)llround(value); }
int64_t rt_math_floor(double value) { return (int64_t)floor(value); }
int64_t rt_math_ceil(double value) { return (int64_t)ceil(value); }

int64_t rt_math_sum_i64(void *list) {
    int64_t len = rt_list_len(list);
    int64_t total = 0;
    for (int64_t i = 0; i < len; i++) {
        total += rt_list_get_i64(list, i);
    }
    return total;
}

int64_t rt_math_min_list_i64(void *list) {
    int64_t len = rt_list_len(list);
    if (len <= 0) return 0;
    int64_t result = rt_list_get_i64(list, 0);
    for (int64_t i = 1; i < len; i++) {
        int64_t value = rt_list_get_i64(list, i);
        if (value < result) result = value;
    }
    return result;
}

int64_t rt_math_max_list_i64(void *list) {
    int64_t len = rt_list_len(list);
    if (len <= 0) return 0;
    int64_t result = rt_list_get_i64(list, 0);
    for (int64_t i = 1; i < len; i++) {
        int64_t value = rt_list_get_i64(list, i);
        if (value > result) result = value;
    }
    return result;
}

double rt_math_mean_i64(void *list) {
    int64_t len = rt_list_len(list);
    if (len <= 0) return 0.0;
    return (double)rt_math_sum_i64(list) / (double)len;
}

double rt_math_variance_i64(void *list) {
    int64_t len = rt_list_len(list);
    if (len <= 0) return 0.0;
    double mean = rt_math_mean_i64(list);
    double total = 0.0;
    for (int64_t i = 0; i < len; i++) {
        double delta = (double)rt_list_get_i64(list, i) - mean;
        total += delta * delta;
    }
    return total / (double)len;
}

double rt_math_stddev_i64(void *list) {
    return sqrt(rt_math_variance_i64(list));
}

static void sort_i64(int64_t *values, int64_t len) {
    for (int64_t i = 1; i < len; i++) {
        int64_t key = values[i];
        int64_t j = i - 1;
        while (j >= 0 && values[j] > key) {
            values[j + 1] = values[j];
            j--;
        }
        values[j + 1] = key;
    }
}

double rt_math_median_i64(void *list) {
    int64_t len = rt_list_len(list);
    if (len <= 0) return 0.0;
    int64_t *copy = (int64_t *)malloc((size_t)len * sizeof(int64_t));
    if (!copy) return 0.0;
    for (int64_t i = 0; i < len; i++) {
        copy[i] = rt_list_get_i64(list, i);
    }
    sort_i64(copy, len);
    double result;
    if ((len & 1) == 1) {
        result = (double)copy[len / 2];
    } else {
        result = ((double)copy[(len / 2) - 1] + (double)copy[len / 2]) / 2.0;
    }
    free(copy);
    return result;
}

void *rt_math_range_i64(int64_t end) {
    return rt_math_range_step_i64(0, end, 1);
}

void *rt_math_range_between_i64(int64_t start, int64_t end) {
    return rt_math_range_step_i64(start, end, start <= end ? 1 : -1);
}

void *rt_math_range_step_i64(int64_t start, int64_t end, int64_t step) {
    void *result = rt_list_create(8, 8);
    if (!result || step == 0) return result;
    if (step > 0) {
        for (int64_t value = start; value < end; value += step) {
            result = rt_list_push_i64(result, value);
        }
    } else {
        for (int64_t value = start; value > end; value += step) {
            result = rt_list_push_i64(result, value);
        }
    }
    return result;
}
