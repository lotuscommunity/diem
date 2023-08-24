FROM ubuntu:20.04@sha256:a06ae92523384c2cd182dcfe7f8b2bf09075062e937d5653d7d0db0375ad2221 AS ubuntu-base

## get rust build environment ready
FROM rust:1.66.1-buster@sha256:e518dbab65069f4869f0159a460a989161ed277913c03427ed8b84542b771f7e AS rust-base

WORKDIR /diem
RUN apt-get update && apt-get install -y cmake curl clang git pkg-config libssl-dev libpq-dev lld

### Build Rust code ###
FROM rust-base as builder

ARG GIT_REPO=https://github.com/aptos-labs/diem-core.git
ARG GIT_REF

RUN git clone $GIT_REPO ./ && git reset $GIT_REF --hard
RUN --mount=type=cache,target=/diem/target --mount=type=cache,target=$CARGO_HOME/registry \
  cargo build --release \
  -p diem-rosetta \
  && mkdir dist \
  && cp target/release/diem-rosetta dist/diem-rosetta

### Create image with diem-node and diem-rosetta ###
FROM ubuntu-base AS rosetta

RUN apt-get update && apt-get install -y libssl-dev ca-certificates && apt-get clean && rm -r /var/lib/apt/lists/*

COPY --from=builder /diem/dist/diem-rosetta /usr/local/bin/diem-rosetta

# Rosetta API
EXPOSE 8082

# Capture backtrace on error
ENV RUST_BACKTRACE 1

WORKDIR /opt/diem/data

ENTRYPOINT ["/usr/local/bin/diem-rosetta"]
CMD ["online", "--config /opt/diem/fullnode.yaml"]
