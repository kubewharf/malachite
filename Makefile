# Copyright 2023 The Malachite Authors.

# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
# 
#     http://www.apache.org/licenses/LICENSE-2.0

# Unless required by applicable law or agreed to in writing, software
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

build:
	@echo "start building"
	@bash ./build/docker_build.sh
run:
	@echo "start running"
	@./output/bin/malachite
clean:
	@echo "start cleaning"
	@rm -rf output/
all: build
.PHONY: all build run clean
