#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <assert.h>
#include <math.h>
#include <inttypes.h>
#include <unistd.h>

typedef struct {
  int8_t* ptr;
  int64_t width;
  int64_t height;
} image;

// this function will be provided at runtime
int64_t function(int64_t x, int64_t y, image inputs[], size_t num_inputs);

// only for storing values in output
int8_t* at(int8_t* image, int64_t width, int64_t x, int64_t y) {
  return &image[y*width + x];
}

// core functions
int64_t core_isqrt(int64_t input) {
  return (int64_t) sqrt((double)input);
}

// declare function, then define it inline to encourage inlining
int64_t core_input_at(int64_t x, int64_t y,
    image inputs[], size_t num_inputs, size_t input);

inline int64_t core_input_at(int64_t x, int64_t y,
                             image inputs[], size_t num_inputs, size_t input)
{
  (void)num_inputs;

  if (x >= inputs[input].width || x < 0) {
    return 0;
  }

  if (y >= inputs[input].height || y < 0) {
    return 0;
  }

  image i = inputs[input];
  return (int64_t) i.ptr[y * i.width + x];
}

// "main"
// the ptr arguments will probably be passed as an int64_t since llvm-rs can
// deal with those properly, but nothing should care
void jitfunction(
    int64_t width,
    int64_t height,
    int8_t* output_ptr,
    int64_t fake,
    uint64_t num_inputs)
{
  int8_t** input_ptrs = (int8_t**) fake;

  image output;
  output.ptr    = output_ptr;
  output.width  = width;
  output.height = height;

  image inputs[num_inputs];
  for (size_t i = 0; i < num_inputs; i++) {
    inputs[i].ptr    = input_ptrs[i];
    inputs[i].width  = width;
    inputs[i].height = height;
  }

  for (int x = 0; x < output.width; x++) {
    for (int y = 0; y < output.height; y++) {
      int64_t res = function(x, y, inputs, num_inputs);

      // clamp the pixel
      if (res >= 255) res = 255;
      if (res <= 0)   res = 0;

      *at(output.ptr, output.width, x, y) = res;
    }
  }
}
