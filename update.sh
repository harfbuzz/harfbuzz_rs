#!/bin/bash

VERSION="8.0.1"

wget https://github.com/harfbuzz/harfbuzz/releases/download/$VERSION/harfbuzz-$VERSION.tar.xz
rm -rf harfbuzz
tar xvf harfbuzz-$VERSION.tar.xz
rm harfbuzz-$VERSION.tar.xz
mv harfbuzz-$VERSION harfbuzz
bindgen --no-prepend-enum-name --allowlist-function hb_.\* --allowlist-type hb_.\* wrapper.h > src/bindings.rs
