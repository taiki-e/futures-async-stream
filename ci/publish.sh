#!/bin/bash

set -euo pipefail
IFS=$'\n\t'

cd "$(cd "$(dirname "${0}")" && pwd)"/..

set -x

(
  cd futures-async-stream-macro
  cargo publish
)

sleep 30
cargo publish
