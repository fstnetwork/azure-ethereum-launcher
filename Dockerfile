## builder
FROM alpine:edge AS builder

# show backtraces
ENV RUST_BACKTRACE 1

RUN apk add --update --no-cache \
  git \
  build-base \
  cargo \
  cmake \
  eudev-dev \
  libusb-dev \
  libusb \
  openssl-dev \
  clang \
  clang-dev \
  linux-headers \
  rust

WORKDIR /ethereum-launcher

COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./rust-toolchain ./rust-toolchain
COPY ./src ./src

# this build step will cache dependencies
RUN cargo build --release --target x86_64-alpine-linux-musl

# copy binary to /usr/bin
RUN cp target/x86_64-alpine-linux-musl/release/ethereum-launcher /usr/bin/ethereum-launcher

## Ethereum-Launcher
FROM alpine:edge

# show backtraces
ENV RUST_BACKTRACE 1

RUN apk add --no-cache \
  libstdc++ \
  openssl \
  eudev-libs \
  libgcc \
  libusb

COPY --from=builder /usr/bin/ethereum-launcher /usr/bin/ethereum-launcher

ENTRYPOINT [ "/bin/sh" ]
