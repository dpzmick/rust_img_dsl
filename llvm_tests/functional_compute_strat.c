#include <stdio.h>
#include <stdlib.h>

#define width 100
#define height 100
#define index(arr, x,y) (arr[x + y * width])

static const int* global_image = NULL;

int input_image(int x, int y) {
  return index(global_image, x, y);
}

int t1_1(int x, int y) {
  return input_image(x, y) + 1;
}

int kernel_1(int x, int y) {
  int k[][3] = {{-1, 0, 1}, {-2, 0, 2}, {-1, 0, 1}};

  return (input_image(x - 1, y - 1) * k[0][0])
       + (input_image(x - 1, y    ) * k[1][0])
       + (input_image(x - 1, y + 1) * k[2][0])
       + (input_image(x    , y - 1) * k[0][1])
       + (input_image(x    , y    ) * k[1][1])
       + (input_image(x    , y + 1) * k[2][1])
       + (input_image(x + 1, y - 1) * k[0][2])
       + (input_image(x + 1, y    ) * k[1][2])
       + (input_image(x + 1, y + 1) * k[2][2]);
}

int joiner_1(int x, int y) {
  return (t1_1(x, y) + kernel_1(x, y)) / 2;
}

int* jitfunction(const int* base_image) {
  global_image = base_image;

  // joiner(t1, kernel)
  int* out = malloc(width * height * sizeof(*out));
  for (int x = 0; x < width; x++) {
    for (int y = 0; y < width; y++) {
      index(out, x, y) = joiner_1(x,y);
    }
  }

  return out;
}

int main() {
  int* in = malloc(width * height * sizeof(*in));
  for (int x = 0; x < width; x++) {
    for (int y = 0; y < width; y++) {
      index(in, x, y) = x + y;
    }
  }

  int* out = jitfunction(in);
  printf("%d\n", index(out, 1, 1));
  return 0;
}
