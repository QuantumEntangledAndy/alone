# Alone Docker Script
# Copyright (c) 2020 George Hilliard
# SPDX-License-Identifier: AGPL-3.0-only

FROM pytorch/pytorch:1.12.0-cuda11.3-cudnn8-devel AS build
LABEL authours="QuantumEntangledAndy <sheepchaan@gmail.com>"

ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update && \
     apt-get install -y --no-install-recommends \
        curl \
        openssl \
        pkg-config \
        libssl-dev \
        unzip \
        build-essential \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

# RUSTUP
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs --output rustup.sh && sh rustup.sh -y
ENV PATH="/root/.cargo/bin:${PATH}"

# ALONE
WORKDIR /usr/local/src/alone

# Build the main program
COPY . /usr/local/src/alone
RUN cargo fetch

# LIBTORCH
ENV LIBTORCH=/opt/conda/lib/python3.7/site-packages/torch
ENV LD_LIBRARY_PATH="${LIBTORCH}/lib:${LD_LIBRARY_PATH}"
ENV LIBTORCH_CXX11_ABI=0

# ALONE
WORKDIR /usr/local/src/alone

# Build the main program
RUN cargo build --release

# Create the release container. Match the base OS used to build
FROM pytorch/pytorch:1.12.0-cuda11.3-cudnn8-runtime
LABEL authours="QuantumEntangledAndy <sheepchaan@gmail.com>"

ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update && \
     apt-get install -y --no-install-recommends \
        openssl \
        curl \
        vim \
        unzip \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

# LIBTORCH
ENV LIBTORCH=/opt/conda/lib/python3.7/site-packages/torch
ENV LD_LIBRARY_PATH=${LIBTORCH}/lib:$LD_LIBRARY_PATH

COPY --from=build \
  /usr/local/src/alone/target/release/alone \
  /usr/local/bin/alone
COPY docker/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

WORKDIR /app

CMD ["/usr/local/bin/alone"]
ENTRYPOINT ["/entrypoint.sh"]
