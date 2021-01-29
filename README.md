# bitwise-io
A Rust Library that enables you to read/write single bits from/to a stream

## Install
Add this to your `Cargo.toml`
```toml
[dependencies]
bitwise-io = "0.1.2"
```

## Internal
### Reader
The `BitReader` wraps a `BufRead` Trait and a position indicator for the next bit.

### Writer
The `BitWriter` wraps the `Write` Trait and has an internal buffer of 1024 bytes (8192 bits).
