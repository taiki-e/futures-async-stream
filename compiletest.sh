#!/bin/bash

# A script to run compile tests with the same condition of the checks done by CI.
#
# Usage
#
# ```sh
# . ./compiletest.sh
# ```

TRYBUILD=overwrite cargo +nightly test -p futures-async-stream --all-features --test compiletest -- --ignored
# cargo +nightly test -p futures-async-stream --all-features --test compiletest -- --ignored
