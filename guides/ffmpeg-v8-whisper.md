# Realtime audio transcription w/ FFmpeg CLI v8 and Whisper

This walkthrough shows how to transcribe mic audio in realtime on the command
line. It takes about 10 minutes to set up.

For a Rust implementation of the same approach, see [`examples/whisper.rs`](../examples/whisper.rs).

## Checking if your FFmpeg binary supports Whisper

Whisper support was added in [FFmpeg 8.0 "Huffman"](https://ffmpeg.org/index.html#pr8.0).

From the official Windows download source, the "essentials" builds doesn't
include it, but "full" does.

Check your FFmpeg version and look for the `--enable-whisper` flag in the
configuration section.

```cli
ffmpeg -version
```

## Downloading the Whisper model

You still need to download a local model to pass into the `whisper` filter in
FFmpeg.

Do that by following the steps in the [`whisper.cpp`
Readme](https://github.com/ggml-org/whisper.cpp?tab=readme-ov-file#quick-start):

First clone the repository:

```cli
git clone https://github.com/ggml-org/whisper.cpp.git
```

Navigate into the directory:

```cli
cd whisper.cpp
```

Then, download one of the Whisper models converted in ggml format. For example:

```cli
sh ./models/download-ggml-model.sh base.en
```

## Invoking the `whisper` filter

You can follow examples from the [`whisper` filter
docs](https://www.ffmpeg.org/ffmpeg-all.html#whisper-1).

Here's a minimal command using a dummy audio source:

```bash
ffmpeg -f lavfi -i "sine=frequency=1000:duration=5" -af "whisper=model=./whisper.cpp/models/ggml-base.en.bin:destination=-:queue=2" -f null -
```

Key parameters:
- `model`: Path to the downloaded ggml model file
- `destination=-`: Use `-` to output transcription to stdout (FFmpeg AVIO syntax)
- `queue`: Seconds to buffer before processing (affects latency vs accuracy trade-off)

## Bonus: Realtime transcribe from mic input

### Find default audio device

Find the device name for your desired input:

```cli
ffmpeg -list_devices true -f dshow -i dummy
```

### Capture and transcribe

```cli
ffmpeg -hide_banner -loglevel error -f dshow -i audio="Microphone (Realtek(R) Audio)" -af "whisper=model=./whisper.cpp/models/ggml-base.en.bin:destination=-:queue=2" -f null -
```

This `dshow` example is for Windows; Max and Linux would use different audio
backends.

## Taking it further

Rather than outputting directly to the command line, this realtime output could
be captured from `stdout` and read programatically; the [Rust demo](../examples/whisper.rs) shows one
example of that.

In a real world use case, you could bundle the appropriate `whisper.cpp` model
alongside a redistributable FFmpeg binary (subject to licensing terms), and
instantly have a realtime transcription service functioning in your app.