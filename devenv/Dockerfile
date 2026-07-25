FROM golang:trixie AS go-base

FROM ubuntu:noble AS devenv-base

ENV USER_PATH=""

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    USER_PATH=~/.cargo/bin:/usr/local/cargo/bin:$USER_PATH \
    RUST_VERSION=1.94.0 \
    SWIFT_SIGNING_KEY=52BB7E3DE28A71BE22EC05FFEF80A866B47A981F \
    SWIFT_PLATFORM=ubuntu24.04 \
    SWIFT_BRANCH=swift-6.3.3-release \
    SWIFT_VERSION=swift-6.3.3-RELEASE \
    SWIFT_WEBROOT=https://download.swift.org

RUN set -eux; \
    apt-get update; \
    apt-get -y install --no-install-recommends \
     binutils \
     build-essential \
     curl \
     git \
     gnupg2 \
     # Most of these are for swift
     libedit2 \
     libc6-dev \
     libcurl4-openssl-dev \
     libedit2 \
     libgcc-13-dev \
     libpython3-dev \
     libsqlite3-0 \
     libstdc++-13-dev \
     libxml2-dev \
     libncurses-dev \
     libz3-dev \
     pkg-config \
     less \
     jq \
     man-db \
     nodejs npm \
     python3-pip \
     python3-virtualenv \
     unminimize \
     rsync \
     tzdata \
     unzip \
     vim \
     wget \
     zlib1g-dev; \
    yes | unminimize; \
    rm -rf /var/lib/apt/lists/*; \
    #
    # Install rust from https://github.com/rust-lang/docker-rust/blob/master/stable/bullseye/Dockerfile
    #
    arch="$(dpkg --print-architecture)"; \
    case "$arch" in \
        'amd64') \
            rustArch='x86_64-unknown-linux-gnu'; \
            rustupSha256='20a06e644b0d9bd2fbdbfd52d42540bdde820ea7df86e92e533c073da0cdd43c'; \
            ;; \
        'armhf') \
            rustArch='armv7-unknown-linux-gnueabihf'; \
            rustupSha256='3b8daab6cc3135f2cd4b12919559e6adaee73a2fbefb830fadf0405c20231d61'; \
            ;; \
        'arm64') \
            rustArch='aarch64-unknown-linux-gnu'; \
            rustupSha256='e3853c5a252fca15252d07cb23a1bdd9377a8c6f3efa01531109281ae47f841c'; \
            ;; \
        'i386') \
            rustArch='i686-unknown-linux-gnu'; \
            rustupSha256='a5db2c4b29d23e9b318b955dd0337d6b52e93933608469085c924e0d05b1df1f'; \
            ;; \
        *) \
            echo >&2 "unsupported architecture: $arch"; \
            exit 1; \
            ;; \
    esac; \
    \
    url="https://static.rust-lang.org/rustup/archive/1.28.2/${rustArch}/rustup-init"; \
    wget --progress=dot:giga "$url"; \
    echo "${rustupSha256} *rustup-init" | sha256sum -c -; \
    \
    chmod +x rustup-init; \
    ./rustup-init -y --no-modify-path --profile minimal --default-toolchain $RUST_VERSION --default-host ${rustArch}; \
    rm rustup-init; \
    chmod -R a+w $RUSTUP_HOME $CARGO_HOME; \
    \
    rustup --version; \
    cargo --version; \
    rustc --version; \
    #
    # Install swift from https://github.com/swiftlang/swift-docker/blob/main/6.3/ubuntu/24.04/Dockerfile
    # See vars above
    #
    ARCH_NAME="$(dpkg --print-architecture)"; \
    url=; \
    case "${ARCH_NAME##*-}" in \
        'amd64') \
            OS_ARCH_SUFFIX=''; \
            ;; \
        'arm64') \
            OS_ARCH_SUFFIX='-aarch64'; \
            ;; \
        *) echo >&2 "error: unsupported architecture: '$ARCH_NAME'"; exit 1 ;; \
    esac; \
    SWIFT_WEBDIR="$SWIFT_WEBROOT/$SWIFT_BRANCH/$(echo $SWIFT_PLATFORM | tr -d .)$OS_ARCH_SUFFIX" \
    && SWIFT_BIN_URL="$SWIFT_WEBDIR/$SWIFT_VERSION/$SWIFT_VERSION-$SWIFT_PLATFORM$OS_ARCH_SUFFIX.tar.gz" \
    && SWIFT_SIG_URL="$SWIFT_BIN_URL.sig" \
    # - Grab curl here so we cache better up above
    && export DEBIAN_FRONTEND=noninteractive \
    # - Download the GPG keys, Swift toolchain, and toolchain signature, and verify.
    && export GNUPGHOME="$(mktemp -d)" \
    && curl -fsSL "$SWIFT_BIN_URL" -o swift.tar.gz "$SWIFT_SIG_URL" -o swift.tar.gz.sig \
    && gpg --batch --quiet --keyserver keyserver.ubuntu.com --recv-keys "$SWIFT_SIGNING_KEY" \
    && gpg --batch --verify swift.tar.gz.sig swift.tar.gz \
    # - Unpack the toolchain, set libs permissions, and clean up.
    && tar -xzf swift.tar.gz --directory / --strip-components=1 \
    && chmod -R o+r /usr/lib/swift \
    && rm -rf "$GNUPGHOME" swift.tar.gz.sig swift.tar.gz


# Setup Go
ENV GOLANG_VERSION=1.26.0
ENV GOTOOLCHAIN=local
ENV PATH=~/go/bin:/usr/local/go/bin:$PATH
ENV USER_PATH=~/go/bin:/usr/local/go/bin:$USER_PATH
COPY --from=go-base /usr/local/go/ /usr/local/go/

# Adjust global path
RUN sed "s%PATH=\"%PATH=\"${USER_PATH}%" -i /etc/environment

#
# Ubuntu User Image
#
FROM devenv-base AS devenv-ubuntu

RUN set -eux; \
    apt-get update; \
    apt-get -y install --no-install-recommends \
     sudo; \
    rm -rf /var/lib/apt/lists/*; \
    echo ubuntu ALL=\(root\) NOPASSWD:ALL > /etc/sudoers.d/ubuntu; \
    chmod 0440 /etc/sudoers.d/ubuntu;

USER ubuntu

#
# Codespaces Image
#
FROM devenv-ubuntu AS devenv-cs
LABEL \
    devcontainer.metadata="[{\"containerUser\":\"ubuntu\",\"remoteUser\":\"ubuntu\"}]"

#
# CI Image
#
FROM devenv-ubuntu AS devenv-ci

ENV PATH=/home/ubuntu/.local/bin:$PATH

RUN set -eux; \
    mkdir -p /home/ubuntu/.config/cmd_install /home/ubuntu/.local/bin; \
    curl -fsSL https://raw.githubusercontent.com/kdeal/cs-dotfiles/main/bin/cmd-install \
        -o /home/ubuntu/.local/bin/cmd-install; \
    curl -fsSL https://raw.githubusercontent.com/kdeal/cs-dotfiles/main/xdg_config/cmd_install/config.toml \
        -o /home/ubuntu/.config/cmd_install/config.toml; \
    chmod +x /home/ubuntu/.local/bin/cmd-install; \
    cmd-install just uv jj stylua oxfmt oxlint ty ruff

#
# SSH Image
#
FROM devenv-base AS devenv-ssh

RUN set -eux; \
    apt-get update; \
    apt-get -y install --no-install-recommends \
     openssh-server; \
    rm -rf /var/lib/apt/lists/*; \
    sed -i 's/#PasswordAuthentication yes/PasswordAuthentication no/g' /etc/ssh/sshd_config; \
    # Remove default user to free up 1000 uid in case I want to use it
    userdel ubuntu;

COPY entrypoint.sh /tmp/

EXPOSE 22
CMD ["/tmp/entrypoint.sh"]
