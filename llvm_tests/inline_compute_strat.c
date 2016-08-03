#include <stdio.h>
#include <stdlib.h>

#define width 100
#define height 100
#define index(arr, x,y) (arr[x + y * width])

int* t1(const int* restrict input) {
  int* out = malloc(width * height * sizeof(*out));
  for (int x = 0; x < width; x++) {
    for (int y = 0; y < width; y++) {
      index(out, x, y) = index(input, x, y) + 1;
    }
  }

  return out;
}

int* kernel(const int* restrict input) {
  int* out = malloc(width * height * sizeof(*out));

  int k[][3] = {{-1, 0, 1}, {-2, 0, 2}, {-1, 0, 1}};

  for (int x = 0; x < width; x++) {
    for (int y = 0; y < width; y++) {
      index(out, x, y) =
          (index(input, x - 1, y - 1) * k[0][0])
        + (index(input, x - 1, y    ) * k[1][0])
        + (index(input, x - 1, y + 1) * k[2][0])
        + (index(input, x    , y - 1) * k[0][1])
        + (index(input, x    , y    ) * k[1][1])
        + (index(input, x    , y + 1) * k[2][1])
        + (index(input, x + 1, y - 1) * k[0][2])
        + (index(input, x + 1, y    ) * k[1][2])
        + (index(input, x + 1, y + 1) * k[2][2]);
    }
  }

  return out;
}

int* joiner(const int* restrict i1, const int* restrict i2) {
  int* out = malloc(width * height * sizeof(*out));
  for (int x = 0; x < width; x++) {
    for (int y = 0; y < width; y++) {
      index(out, x, y) = (index(i1, x, y) + index(i2, x, y)) / 2;
    }
  }

  return out;
}

int* jitfunction(const int* restrict base_image) {
  int* t1_out = t1(base_image);
  int* first_kern = kernel(base_image);
  int* out = joiner(t1_out, first_kern);

  // would copy the result into rust image here

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
