# syntax=docker/dockerfile:1
FROM fedora:34

RUN dnf install -y \
  git \
  openssl \
  openssl-devel \
  cargo \
  rust
RUN git clone https://github.com/tstrempel/scaphandre
WORKDIR /scaphandre
RUN cargo build --release
RUN cp /scaphandre/target/release/scaphandre /usr/local/bin/
ENTRYPOINT ["/usr/local/bin/scaphandre"]
