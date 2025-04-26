# goose-ffi2

Foreign Function Interface (FFI) bindings for the goose-llm crate, designed for multi-threaded usage in Kotlin and other languages.

## Overview

This crate provides a thread-safe interface for integrating with the `goose-llm` library from languages like Kotlin, Java, C++, etc. The key features include:

- Thread-safe design with a shared Tokio runtime
- Support for concurrent completion requests from multiple threads
- Memory management utilities for C interoperability
- Simplified interface for creating extensions and tools
- Comprehensive error handling and reporting

## Building

To build the library:

```bash
cargo build --release
```

This will generate:
- A dynamic library (`libgoose_ffi2.so`, `libgoose_ffi2.dylib`, or `goose_ffi2.dll` depending on your platform)
- C header file in the `include` directory
