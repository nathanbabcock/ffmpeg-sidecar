# FFmpeg Sidecar ๐

[Github](https://github.com/nathanbabcock/ffmpeg-sidecar) |
[Crates.io](https://crates.io/crates/ffmpeg-sidecar) |
[Docs.rs](https://docs.rs/ffmpeg-sidecar)

> Wrap a standalone FFmpeg binary in an intuitive Iterator interface.

## Features

- โจ Zero dependencies
- โก Automatic FFmpeg CLI download (if needed)
- ๐ค Support for Windows, MacOS, and Linux
- ๐งช Thoroughly unit tested

> ๐ Jump to [Getting Started](#getting-started) ๐

## Motivation

The core goal of this project is to provide a method of interacting with any video **as if it were an
array of raw RGB frames**.

Of course, that's what video _is_, fundamentally, but there is a whole pandora's
box of complexity in terms of receiving and decoding video before you get there.

Using FFmpeg as the core engine provides interoperability between a massive
range of formats, containers, extensions, protocols, encoders, decoders, hardware accelerations, and
more.

## Why CLI?

One method of using FFmpeg is low-level bindings to the code used inside the CLI
itself -- there are [good crates](https://crates.io/crates/ffmpeg-sys-next) in
the Rust ecosystem that do this.

Low level bindings have drawbacks, though:

- Difficult, time-consuming build, toolchain, and dependencies, especially on Windows
- Complexity, especially for beginners
- You end up manually re-implementing a lot of the standard conversions you need
  from scratch

By wrapping the CLI, this crate avoids those downsides, and also solves some of
the pain points that you would encounter if you were to use the CLI directly on
its own:

- Raw data can easily move in and out of FFmpeg instances, or piped between them. Under the hood they
  are moving across stdin and stdout.
- Rich semantic information is recovered from the ffmpeg stderr logs, including:
  - Progress updates (frame #, timestamp, speed, bitrate, ...)
  - Input/output metadata and stream mappings
  - Warnings & errors
- Argument presets and aliases with discoverable names through Intellisense/autocomplete

The only remaining downside is the size of the FFmpeg binary itself, but it's
less than 100MB when zipped. It can be automatically downloaded by the crate, so
you may choose to not even ship it with your own application and instead
download it at runtime.

## Getting Started

### 1. Cargo Install

On the Rust side, it has **zero** Cargo dependencies! ๐

```console
cargo add ffmpeg-sidecar
```

### 2. Download FFmpeg

To automatically download & install a FFmpeg binary for your platform
(Windows, MacOS, and Linux), call this function anywhere in your program:

```rust
ffmpeg_sidecar::download::auto_download().unwrap();
```

You can do this once to set up your dev environment, or include it as a feature
of your client application.

> To customize or extend the download, see [`/examples/download_ffmpeg.rs`](/examples/download_ffmpeg.rs).

## Examples

### Hello world ๐

Read raw video frames.

```rust
use ffmpeg_sidecar::{
  child::FfmpegChild, command::FfmpegCommand, event::FfmpegEvent, iter::FfmpegIterator,
};

/// Iterates over the frames of a testsrc.
fn main() {
  // similar to `std::process::Command`
  let mut command = FfmpegCommand::new();
  command
    .testsrc() // generate a test pattern video
    .rawvideo(); // pipe raw video output

  // similar to `std::process::Child`
  let mut child: FfmpegChild = command.spawn().unwrap();

  // Iterator over all messages and output
  let iter: FfmpegIterator = child.iter().unwrap();
  iter.for_each(|event: FfmpegEvent| {
    match event {
      FfmpegEvent::OutputFrame(frame) => {
        let _pixels = frame.data; // <- raw RGB pixels! ๐จ
      }
      FfmpegEvent::Error(e) => eprintln!("Error: {}", e),
      _ => {}
    }
  });
}
```

Source: [`/examples/hello_world.rs`](/examples/hello_world.rs)

```console
cargo run --example hello-world
```

### H265 Transcoding

Decode H265, modify the decoded frames, and then write back to H265.

Source: [`/examples/h265_transcode.rs`](/examples/h265_transcode.rs)

```console
cargo run --example h265_transcode
```

### FFplay

Pipe an FFmpeg instance to FFplay for debugging purposes.

Source: [`/examples/ffplay_preview.rs`](/examples/ffplay_preview.rs)

```console
cargo run --example ffplay_preview
```

### Others

For a myriad of other examples, check any of the unit tests in
[/src/test.rs](/src/test.rs) in this repo.

## Todo

- [X] Add `/examples`
- [X] Take input from stdin, and pipe between iterators
- [X] Pipe directly to `ffplay` for debugging
- [X] Idiomatic error type instead of `Result<_, String>`
- [X] Handle indeterminate output formats like H264/H265
  - Currently these formats are mutually exclusive with using `iter()` since
    they require consuming `stdout` directly

## See also

Inspired loosely by Node.js
[`fluent-ffmpeg`](https://www.npmjs.com/package/fluent-ffmpeg), which does
something similar in Javascript.

Uses [`setup-ffmpeg`](https://github.com/FedericoCarboni/setup-ffmpeg) for
Github Actions and as a reference for the auto-download behavior.

## ๐ฃ Pull Requests Welcome ๐ฃ
