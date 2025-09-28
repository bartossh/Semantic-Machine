FROM rust:1.90 as builder

WORKDIR /usr/src/app

ENV CARGO_INCREMENTAL=0
ENV CARGO_BUILD_JOBS=4
ENV CARGO_PROFILE_RELEASE_LTO=fat
ENV CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
ENV CARGO_PROFILE_RELEASE_OPT_LEVEL=3

COPY Cargo.toml Cargo.lock ./
COPY crates crates/
COPY apps apps/

ARG BIN_NAME=api-service
ENV BIN_NAME=${BIN_NAME}

RUN cargo clean
RUN cargo build --release --bin ${BIN_NAME}

FROM gcr.io/distroless/cc-debian12

ARG BIN_NAME=api-service
ARG SERVER_HOST=0.0.0.0
ARG SERVER_PORT=8080

ENV BIN_NAME=${BIN_NAME}
ENV SERVER_HOST=${SERVER_HOST}
ENV SERVER_PORT=${SERVER_PORT}

WORKDIR /app

COPY --from=builder /usr/src/app/target/release/${BIN_NAME} /app/executable
COPY --from=builder /usr/src/app/apps/api-service/migrations /app/migrations

EXPOSE ${SERVER_PORT}

ENTRYPOINT ["/app/executable"]
