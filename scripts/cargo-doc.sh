#!/bin/bash

# Source: <https://users.rust-lang.org/t/how-to-document-optional-features-in-api-docs/64577>

# First, install nightly toolchain if needed:
# rustup install nightly

RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc --all-features --open
