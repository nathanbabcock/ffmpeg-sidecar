# FFmpeg Sidecar 🏍

[Github](https://github.com/nathanbabcock/ffmpeg-sidecar) |
[Crates.io](https://crates.io/crates/ffmpeg-sidecar) |
[Docs.rs](https://docs.rs/ffmpeg-sidecar)

![Github actions](https://github.com/nathanbabcock/ffmpeg-sidecar/actions/workflows/ci.yml/badge.svg)

> Wrap a standalone FFmpeg binary in an intuitive Iterator interface.

## Features

- ✨ Minimal dependencies
- ⚡ Automatic FFmpeg CLI download (if needed)
- 🤗 Support for Windows, MacOS, and Linux
- 🧪 Thoroughly unit tested

> 👉 Jump to [Getting Started](#getting-started) 👈

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

- Raw data can easily move in and out of FFmpeg instances, or pipe between them. Under the hood they
  are moving across stdin and stdout.
- Rich semantic information is recovered from the FFmpeg stderr logs, including:
  - Progress updates (frame #, timestamp, speed, bitrate, ...)
  - Input/output metadata and stream mappings
  - Warnings & errors
- Argument presets and aliases with discoverable names through Intellisense/autocomplete

The only remaining downside is the size of the FFmpeg binary itself, but it's
less than 100MB when zipped. It can be automatically downloaded by the crate, so
you may choose to not even ship it with your own application and instead
download it at runtime.

## Getting Started

> [!TIP]
> Integrating with async Rust or `tokio`? Check out [`async-ffmpeg-sidecar`](https://github.com/dvtkrlbs/async-ffmpeg-sidecar).

### 1. Cargo Install

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

### Hello world 👋

Read raw video frames.

```rust
use ffmpeg_sidecar::command::FfmpegCommand;

fn main() -> anyhow::Result<()> {
  // Run an FFmpeg command that generates a test video
  let iter = FfmpegCommand::new() // <- Builder API like `std::process::Command`
    .testsrc()  // <- Discoverable aliases for FFmpeg args
    .rawvideo() // <- Convenient argument presets
    .spawn()?   // <- Ordinary `std::process::Child`
    .iter()?;   // <- Blocking iterator over logs and output

  // Use a regular "for" loop to read decoded video data
  for frame in iter.filter_frames() {
    println!("frame: {}x{}", frame.width, frame.height);
    let _pixels: Vec<u8> = frame.data; // <- raw RGB pixels! 🎨
  }

  Ok(())
}
```

Source: [`/examples/hello_world.rs`](/examples/hello_world.rs)

```console
cargo run --example hello_world
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

### Named pipes

Pipe multiple outputs from FFmpeg into a Rust program.

Source: [`/examples/named_pipes.rs`](/examples/named_pipes.rs)

```console
cargo run --example named_pipes --features named_pipes
```

### Others

For a myriad of other use cases, check any of the [examples](/examples/), as
well as the unit tests in [/src/test.rs](/src/test.rs).

## See also

Inspired loosely by Node.js
[`fluent-ffmpeg`](https://www.npmjs.com/package/fluent-ffmpeg), which does
something similar in Javascript.

Uses [`setup-ffmpeg`](https://github.com/FedericoCarboni/setup-ffmpeg) for
Github Actions and as a reference for the auto-download behavior.

## 📣 Pull Requests Welcome 📣
