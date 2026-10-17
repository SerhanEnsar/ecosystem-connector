# Benchmarks

Every number and chart in the README comes from a script in this folder. Raw results go
to `bench/out/` (not committed); charts go to `docs/media/` and state the machine and
commit they were measured on.

## Loopback transfer

Both devices run in one process over 127.0.0.1 with the full protocol (QUIC, TLS 1.3,
BLAKE3, disk writes). It measures the software's ceiling, not a network.

```sh
cargo run --release -p ec-transfer --example bench_loopback -- 1024 5   # 1 GiB, 5 runs per setting
python3 bench/plot_loopback.py "Apple M4, macOS 27"                     # writes docs/media/bench-loopback.svg
```

It needs about three times the file size in free disk space.

## Planned

- Wi-Fi and USB cable between real devices, with `ec send` (phase 3).
- Resume cost: time to continue an interrupted transfer.
