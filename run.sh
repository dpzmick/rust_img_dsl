#!/bin/bash

jit_out=jit.tsv
native_out=native.tsv

jit=./target/release/examples/sobel
native=./target/release/examples/sobel_native

for i in `seq 1 100`;
do
    $jit >> $jit_out
    $native >> $native_out
    echo done with iteration $i
done
