default: build-tiny

build:
    cargo build --release

compress:
    upx --best --lzma target/release/vicuna

build-tiny: build compress
    @echo "Done! Binary size:"
    @ls -lh target/release/vicuna | awk '{print $5, $9}'
