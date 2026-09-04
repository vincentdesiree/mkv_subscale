#!/usr/bin/env bash
set -e

REPO="vincentdesiree/mkv_subscale"
INSTALL_DIR="/usr/local/bin"

# 1. Detect OS
OS="$(uname -s)"
case "${OS}" in
    Linux*)     TARGET_OS="unknown-linux-gnu";;
    Darwin*)    TARGET_OS="apple-darwin";;
    *)          echo "Error: Unsupported OS '${OS}'" && exit 1;;
esac

# 2. Detect architecture
ARCH="$(uname -m)"
case "${ARCH}" in
    x86_64)     TARGET_ARCH="x86_64";;
    arm64|aarch64)
        if [ "${TARGET_OS}" = "apple-darwin" ]; then
            TARGET_ARCH="aarch64"
        else
            echo "Error: Pre-built Linux ARM binaries are not available." && exit 1
        fi
        ;;
    *)          echo "Error: Unsupported architecture '${ARCH}'" && exit 1;;
esac

TARGET="${TARGET_ARCH}-${TARGET_OS}"
TARBALL="mkv_subscale-${TARGET}.tar.gz"

echo "Downloading mkv_subscale for ${TARGET}..."

# 3. Fetch latest release asset from GitHub
RELEASE_URL="https://github.com/${REPO}/releases/latest/download/${TARBALL}"

TEMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TEMP_DIR}"' EXIT

curl -sSL "${RELEASE_URL}" -o "${TEMP_DIR}/${TARBALL}"
tar -xzf "${TEMP_DIR}/${TARBALL}" -C "${TEMP_DIR}"

# 4. Install binary to target directory (elevate if required)
if [ -w "${INSTALL_DIR}" ]; then
    mv "${TEMP_DIR}/mkv_subscale" "${INSTALL_DIR}/mkv_subscale"
else
    echo "Elevated permissions required to install to ${INSTALL_DIR}"
    sudo mv "${TEMP_DIR}/mkv_subscale" "${INSTALL_DIR}/mkv_subscale"
fi

chmod +x "${INSTALL_DIR}/mkv_subscale"
echo "Successfully installed mkv_subscale to ${INSTALL_DIR}/mkv_subscale"
