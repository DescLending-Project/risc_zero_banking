FROM rust:1.86 AS builder

# Install musl tools
RUN apt-get update && apt-get install -y musl-tools && \
    rustup target add x86_64-unknown-linux-musl

WORKDIR /app
COPY . .

# Build statically linked binary
RUN cargo build --release --target x86_64-unknown-linux-musl

# Use scratch image for minimal size and no glibc
FROM scratch

# Copy statically built binary
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/tlsn-verifier /tlsn-verifier

EXPOSE 8080
CMD ["/tlsn-verifier"]
