FROM registry.opensuse.org/opensuse/leap:15.3 as builder
# zypper ar -p 90 -r https://download.opensuse.org/repositories/devel:/languages:/rust/openSUSE_Leap_15.3/devel:languages:rust.repo
RUN zypper ar -p 90 -r https://download.opensuse.org/repositories/devel:/tools:/compiler/openSUSE_Leap_15.3/devel:tools:compiler.repo \
    && zypper --gpg-auto-import-keys ref \
    && zypper --non-interactive dup --allow-vendor-change
RUN zypper --non-interactive install -t pattern \
    devel_C_C++ \
    devel_basis \
    && zypper --non-interactive install \
    clang \
    curl \
    libelf-devel \
    libopenssl-devel \
    llvm \
    rustup \
    sudo \
    tar \
    xz \
    zlib-devel \
    && zypper clean
RUN rustup-init -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN rustup toolchain install nightly
RUN rustup component add \
    clippy \
    rustfmt
RUN cargo install \
    libbpf-cargo
RUN cargo +nightly install \
    cargo-udeps

FROM builder AS build
WORKDIR /usr/local/src
# Build bpftool from the newest stable kernel sources.
RUN curl -Lso linux.tar.xz \
    $(curl -s https://www.kernel.org/ | grep -A1 "latest_link" | grep -Eo '(http|https)://[^"]+') \
    && tar -xf linux.tar.xz \
    && mv $(find . -maxdepth 1 -type d -name "linux*") linux \
    && cd linux \
    && cd tools/bpf/bpftool \
    && make -j $(nproc)
# Prepare lockc sources and build it.
WORKDIR /usr/local/src/lockc
COPY . ./
ARG profile=release
RUN --mount=type=cache,target=/.root/cargo/registry \
    --mount=type=cache,target=/usr/local/src/lockc/target \
    if [[ "$profile" == "debug" ]]; then cargo build; else cargo build --profile ${profile}; fi \
    && cp target/${profile}/lockcd /usr/local/bin/lockcd

FROM registry.opensuse.org/opensuse/leap:15.3 AS lockcd
# runc links those libraries dynamically
RUN zypper --non-interactive install \
    libseccomp2 \
    libselinux1 \
    && zypper clean
ARG profile=release
# Install rust-gdb and rust-lldb for debug profile
RUN if [[ "$profile" == "debug" ]]; then zypper --non-interactive install gdb lldb python3-lldb rust; fi
COPY --from=build /usr/local/src/linux/tools/bpf/bpftool/bpftool /usr/sbin/bpftool
COPY --from=build /usr/local/bin/lockcd /usr/bin/lockcd
ENTRYPOINT ["/usr/bin/lockcd"]
