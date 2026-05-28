FROM docker.io/paritytech/ci-unified:latest as builder

WORKDIR /hitown
COPY . /hitown

# Fetch dependencies and build
RUN cargo fetch
RUN cargo build --locked --release

# Use a minimal base image
FROM ubuntu:22.04

# Install necessary certificates and curl
RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*

COPY --from=builder /hitown/target/release/solochain-template-node /usr/local/bin/hitown-node

# Create hitown user and isolate data volumes
RUN useradd -m -u 1000 -U -s /bin/sh -d /hitown hitown && \
	mkdir -p /data && \
	chown -R hitown:hitown /data /hitown && \
	# check if executable works in this container
	/usr/local/bin/hitown-node --version

USER hitown
WORKDIR /hitown

# Expose standard Substrate ports
# P2P, RPC, Prometheus
EXPOSE 30333 9944 9615

# Declare data volume for chain state
VOLUME ["/data"]

ENTRYPOINT ["/usr/local/bin/hitown-node"]

