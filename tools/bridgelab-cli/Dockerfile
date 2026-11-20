# syntax=docker/dockerfile:1
#
# bridgelab-cli: the BridgeLab HL7 v2 and FHIR validators, headless, for CI.
# The CLI is built on src-tauri's core, so the build context is the
# repository root (Dockerfile.dockerignore keeps it to what the build needs):
#
#   docker build -f tools/bridgelab-cli/Dockerfile \
#     --build-arg BRIDGELAB_BUILD="$(git log -1 --format=%cd --date=format:%Y%m%d).$(git rev-parse --short=7 HEAD)" \
#     -t bridgelab-cli .
#   docker run --rm -v "$PWD:/work" bridgelab-cli validate "messages/*.hl7"

FROM rust:1.94-slim-trixie AS build
WORKDIR /src
COPY src-tauri ./src-tauri
COPY tools/bridgelab-cli ./tools/bridgelab-cli
# The context has no Git checkout: the build code that --version prints
# comes in as an argument.
ARG BRIDGELAB_BUILD=unknown
ENV BRIDGELAB_BUILD=${BRIDGELAB_BUILD}
RUN cargo build --release --locked --manifest-path tools/bridgelab-cli/Cargo.toml \
 && cp tools/bridgelab-cli/target/release/bridgelab-cli /usr/local/bin/bridgelab-cli

FROM debian:trixie-slim
ARG BRIDGELAB_BUILD=unknown
LABEL org.opencontainers.image.title="bridgelab-cli" \
      org.opencontainers.image.description="BridgeLab HL7 v2 and FHIR validators, headless, for CI" \
      org.opencontainers.image.source="https://github.com/techemv-srl/BridgeLab" \
      org.opencontainers.image.vendor="TECHEMV SRL" \
      org.opencontainers.image.licenses="MIT" \
      org.opencontainers.image.revision="${BRIDGELAB_BUILD}"
COPY --from=build /usr/local/bin/bridgelab-cli /usr/local/bin/bridgelab-cli
COPY LICENSE /usr/share/doc/bridgelab-cli/LICENSE
RUN useradd --uid 10001 --create-home bridgelab
# Plugin packs are read from <config>/BridgeLab/plugins: a fixed config
# folder, so a mounted one is found whatever user the container runs as.
ENV XDG_CONFIG_HOME=/config
USER bridgelab
# Mount the files to check here: docker run -v "$PWD:/work" …
WORKDIR /work
ENTRYPOINT ["bridgelab-cli"]
CMD ["--help"]
