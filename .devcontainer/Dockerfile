FROM mcr.microsoft.com/devcontainers/base:bookworm

# Install curl and build deps
# Rust and Node.js are installed via devcontainer features (see devcontainer.json),
# not here, to avoid a duplicate/conflicting rustup install.
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl ca-certificates build-essential git pkg-config \
    && rm -rf /var/lib/apt/lists/*

CMD ["sleep", "infinity"]

