#!/bin/bash

jit_out=jit.tsv
native_out=native.tsv

jit=./target/release/examples/sobel
native=./target/release/examples/sobel_native

cargo build --example sobel --release
cargo build --example sobel_native --release

for i in `seq 1 1`;
do
    $jit >> $jit_out
    $native >> $native_out
    echo done with iteration $i
done
