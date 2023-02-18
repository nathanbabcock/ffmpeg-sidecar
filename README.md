# FFmpeg Sidecar 🏍

> Wrap a standalone FFmpeg binary in an intuitive Iterator interface.

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
less than 100MB when zipped.

## Getting started

### 1. Download FFmpeg

First you need an FFmpeg binary. If you don't already have one, head to
<https://ffmpeg.org>. Either install it globally (e.g. add to `PATH` on windows),
or simply place the executable adjacent to your Rust binary target. When you
package and distribute

### 2. Cargo install

On the Rust side, it has **zero** Cargo dependencies! 🎉

```console
cargo add ffmpeg-sidecar
```

### 3. Import and use

```rust
use ffmpeg_sidecar::{FfmpegCommand, FfmpegChild, FfmpegEvent};

fn main() {
  // similar to `std::process::Command`
  let command = FfmpegCommand::new()
      .testsrc() // generate a test pattern video
      .rawvideo(); // pipe raw video output

  // similar to `std::process::Child`
  let child: FfmpegChild = command
    .spawn()
    .unwrap();

  // Iterator over all messages and output
  let mut iter: FfmpegIterator = child.events_iter();
  iter.for_each(|event: FfmpegEvent| {
    match event {
      FfmpegEvent::OutputFrame(frame) => {
        let _pixels = frame.data; // <- raw RGB pixels! 🎨
      },
      FfmpegEvent::LogInfo(string) => {
        println!("ffmpeg log info: {}", string);
      },
      _ => {
        // many other kinds of events, including:
        // - parsed inputs, outputs, streams, and metadata
        // - errors and warnings
        // - and more
      }
    }
  });
}
```

## Examples

### H265 Transcoding

Decode H265, modify the decoded frames, and then write back to H265.

Source: [/examples/h265_transcode.rs](/examples/h265_transcode.rs)

```console
cargo run --example h265_transcode
```

### Others

> For a myriad of other examples, check any of the unit tests in
> [/src/test.rs](/src/test.rs) in this repo.

## Todo

- [X] Add `/examples` (WIP)
- [X] Take input from stdin, and pipe between iterators
- [ ] Handle indeterminate output formats like H264/H265
- [ ] Pipe directly to `ffplay` for debugging
- [ ] Check code coverage
- [ ] Idiomatic error type instead of `Result<_, String>`

## See also

Inspired loosely by Node.js
[`fluent-ffmpeg`](https://www.npmjs.com/package/fluent-ffmpeg), which does
something similar in Javascript.
