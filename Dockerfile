# Alone Docker Script
# Copyright (c) 2020 George Hilliard
# SPDX-License-Identifier: AGPL-3.0-only

FROM pytorch/pytorch:1.12.0-cuda11.3-cudnn8-devel AS build
LABEL authours="QuantumEntangledAndy <sheepchaan@gmail.com>"

ARG DEBIAN_FRONTEND=noninteractive

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

WORKDIR /usr/local/src/alone

# Build the main program
COPY . /usr/local/src/alone
RUN cargo build --release

# Create the release container. Match the base OS used to build
FROM pytorch/pytorch:1.12.0-cuda11.3-cudnn8-runtime
LABEL authours="QuantumEntangledAndy <sheepchaan@gmail.com>"

ARG DEBIAN_FRONTEND=noninteractive

COPY --from=build \
  /usr/local/src/alone/target/release/alone \
  /usr/local/bin/alone
COPY docker/entrypoint.sh /entrypoint.sh

CMD ["/usr/local/bin/alone"]
ENTRYPOINT ["/entrypoint.sh"]
EXPOSE 8554
