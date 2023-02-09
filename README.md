# FFMPEG Sidecar

> Wrap an FFMPEG binary in Rust language constructs like Iterators and Streams.

## Motivation

- Provide immediate access to raw output frames on stdout
- Most existing Rust FFMPEG crates are system/FFI bindings, rather than spawning
  a child process
- Inspired loosely by Node.js [`fluent-ffmpeg`](https://www.npmjs.com/package/fluent-ffmpeg)
