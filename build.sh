 #!/bin/zsh


# get the native library and generate bindings
cargo build --release
cargo run --bin uniffi-bindgen generate --library target/release/libraytracer.so --language kotlin --out-dir bindings

# compile for android (only 64 bit)
cargo ndk -t arm64-v8a -t x86_64 -o ./jniLibs build --release --link-libcxx-shared
