#!/bin/bash

VERSION="3.2.0"

wget https://github.com/harfbuzz/harfbuzz/releases/download/$VERSION/harfbuzz-$VERSION.tar.xz
rm -rf harfbuzz
tar xvf harfbuzz-$VERSION.tar.xz
rm harfbuzz-$VERSION.tar.xz
mv harfbuzz-$VERSION harfbuzz
bindgen --no-prepend-enum-name --whitelist-function hb_.\* --whitelist-type hb_.\* wrapper.h > src/bindings.rs
