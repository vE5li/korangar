# Ragnarok Packets

A crate that exposes types for Ragnarok Online server-client communication.

## Examples

### Packet capture

An example that uses the `PacketHandler` to deserialize packets captured with `libpcap` and print them to `stdout`.
Since `pcap` requires privileges to monitor your network traffic, the compiled example needs them as well.


The easiest way is to not use `cargo run` and instead build with
```bash
cargo build --example pcap --features unicode
```

##### Hint: Make sure you have `libpcap` installed on your system, otherwise the build will fail.
##### Hint: You can add the `unicode` feature for some slightly nicer output if your system supports it.


And then run the resulting binary in `target/debug/examples/pcap` as root or admin. E.g.
```bash
sudo target/debug/examples/pcap
```
