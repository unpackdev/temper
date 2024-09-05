# Stage 1: Build the Rust application using Ubuntu
FROM ubuntu:22.04 AS build

# Install required dependencies for Rust
RUN apt-get update && apt-get install -y curl build-essential libssl-dev pkg-config
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH=/root/.cargo/bin:$PATH

WORKDIR /app

# Copy the source code into the container
COPY . .

# Build the Rust project in release mode
RUN cargo build --release

# Stage 2: Use Ubuntu for the final image
FROM ubuntu:22.04

# Copy the binary from the build stage to the final stage
COPY --from=build /app/target/release/enso-temper /enso-temper

# Make the UDS path configurable via the .env file
# Install envsubst to substitute the UDS path from the .env file
RUN apt-get update && apt-get install -y gettext-base

# Expose a mount point for the UDS file (shared socket)
VOLUME ["/tmp"]

# Make sure the UDS path is set via an environment variable (from .env)
COPY .env /app/.env
RUN export $(cat /app/.env | xargs) && echo $UDS_PATH

# Expose the UDS socket file as a volume and the HTTP port
EXPOSE 80

# Command to run the Rust binary
CMD ["sh", "-c", "export $(cat /app/.env | xargs) && ./enso-temper"]
