FROM rustembedded/cross:arm-unknown-linux-gnueabi
ENV DEBIAN_FRONTEND=noninteractive
ENV PKG_CONFIG_PATH=/usr/lib/arm-linux-gnueabi/pkgconfig
ENV RPI_TOOLS=/rpi_tools
ENV MACHINE=armv6
ENV ARCH=armv6
ENV CC=gcc
ENV OPENSSL_DIR=/openssl
ENV INSTALL_DIR=/openssl
ENV CROSSCOMP_DIR=/rpi_tools/arm-bcm2708/arm-bcm2708-linux-gnueabi/bin

RUN apt-get update &&\
    apt-get install -y wget openssl libssl-dev pkg-config libudev-dev lib32z1

# Get Raspberry Pi cross-compiler tools
RUN git -C "/" clone -q --depth=1 https://github.com/raspberrypi/tools.git "${RPI_TOOLS}"

# Manually cross-compile OpenSSL to link with

# 1) Download OpenSSL 1.1.0
RUN mkdir -p $OPENSSL_DIR
RUN cd /tmp && \
    wget --no-check-certificate https://www.openssl.org/source/openssl-1.1.0h.tar.gz && \
    tar xzf openssl-1.1.0h.tar.gz

# 2) Compile
RUN cd /tmp/openssl-1.1.0h && \
    ./Configure linux-generic32 shared \
      --prefix=$INSTALL_DIR --openssldir=$INSTALL_DIR/openssl \
      --cross-compile-prefix=$CROSSCOMP_DIR/arm-bcm2708-linux-gnueabi- && \
      make depend && make && make install

