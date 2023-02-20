#!/bin/bash

# Copyright 2023 The Malachite Authors.
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

rust_platform_target="x86_64-unknown-linux-gnu"
if [[ "${ARCH}" ]]; then
    if [[ "$ARCH" == "aarch64" ]]; then
        rust_platform_target="aarch64-unknown-linux-gnu"
        echo "[build.sh] will compile aarch64"
    fi
else
    echo "[build.sh] compile x86 by default"
fi

echo $rust_platform_target
RUSTFLAGS='-C target-feature=+crt-static' cargo build --target $rust_platform_target --release

mkdir -p output/bin
cp -r ./server/release/static/ ./output/bin
cp ./target/$rust_platform_target/release/malachite ./output/bin

## copy byteperf & bpf config
mkdir -p output/bin/static/pmu_config
cp -r ./lib/src/ffi/pmu/byteperf/config/* ./output/bin/static/pmu_config/

## copy CHANGELOG
cp -r ./CHANGELOG/ ./output

chmod +x ./output/bin/static/run_agent
chmod +x ./output/bin/malachite
