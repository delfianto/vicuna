# Show recipes when invoked with no arguments
set default-list

bin := "target/release/vicuna"
prefix := env("HOME") / ".local/bin"

# Build release binary
build:
    cargo build --release

# UPX-compress the release binary
compress: build
    upx --best --lzma {{bin}}

# Build then compress
build-tiny: compress
    @echo "Done! Binary size:"
    @ls -lh {{bin}} | awk '{print $5, $9}'

# Install release binary to ~/.local/bin (overwrites)
install: build
    mkdir -p {{prefix}}
    install -m 755 {{bin}} {{prefix}}/vicuna
    @echo "Installed {{prefix}}/vicuna"
