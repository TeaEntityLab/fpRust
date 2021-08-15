#!/bin/bash
# Usages Ref:
# ./paralleltest.sh 2>&1 | grep failed
# ./paralleltest.sh 2>&1 | grep "assertion failed"
# ./paralleltest.sh 2>&1 | grep "src/actor.rs"

for i in $(seq 1 1000)
do
   ( ./test.sh ) &
   if (( $i % 10 == 0 )); then wait; fi # Limit to 10 concurrent subshells.
done
wait
