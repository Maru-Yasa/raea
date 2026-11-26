FROM ubuntu:24.04

RUN apt-get update -qq && DEBIAN_FRONTEND=noninteractive apt-get install -y -qq --no-install-recommends \
      ca-certificates ffmpeg libasound2t64 libfontconfig1 libvulkan1 mesa-vulkan-drivers vulkan-tools \
      libxext6 libx11-6 libxkbcommon0 libxkbcommon-x11-0 libgl1 libegl1 \
    && rm -rf /var/lib/apt/lists/*

COPY raea /opt/raea
RUN ln -sf /opt/raea/bin/raea /usr/local/bin/raea \
    && ! ldd /opt/raea/bin/raea /opt/raea/bin/libraea_bundle.so | grep -q 'not found' \
    && RAEA_NO_TREE=1 raea --version

ENV NVIDIA_VISIBLE_DEVICES=all \
    NVIDIA_DRIVER_CAPABILITIES=all \
    WGPU_BACKEND=vulkan

WORKDIR /work
CMD ["raea", "--help"]
