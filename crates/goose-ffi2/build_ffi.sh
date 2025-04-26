#!/bin/bash

# Build script for goose-ffi2 library
# Generates header files and builds for the target platform

set -e

# Detect platform
PLATFORM=$(uname -s)
ARCH=$(uname -m)

case "$PLATFORM" in
  Darwin*)  
    TARGET="darwin-$ARCH"
    LIB_EXT="dylib"
    ;;
  Linux*)   
    TARGET="linux-$ARCH"
    LIB_EXT="so"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    TARGET="windows"
    LIB_EXT="dll"
    ;;
  *)
    echo "Unsupported platform: $PLATFORM"
    exit 1
    ;;
esac

# Build the library
echo "Building for $TARGET..."
cargo build --release

# Copy the library to a convenient location
mkdir -p target/ffi/include
mkdir -p target/ffi/lib/$TARGET

# Copy header files
cp include/goose_ffi2.h target/ffi/include/

# Copy the library
if [ "$PLATFORM" == "Darwin" ]; then
  cp target/release/libgoose_ffi2.$LIB_EXT target/ffi/lib/$TARGET/
elif [ "$PLATFORM" == "Linux" ]; then
  cp target/release/libgoose_ffi2.$LIB_EXT target/ffi/lib/$TARGET/
else
  cp target/release/goose_ffi2.$LIB_EXT target/ffi/lib/$TARGET/
fi

echo "Build complete. Libraries and headers are in target/ffi/"