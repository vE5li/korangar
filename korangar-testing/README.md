# Korangar Testing

A harness for driving the Korangar client through scripted scenarios against a live server.
This crate exposes a `TestManager` that implements `ClientHooks` and advances through a sequence of `WorkStep`s, injecting input events and asserting on network events and client state.

## Tests

### Smoke test

To run this example you need a local rAthena server reachable on `127.0.0.1` (login on port `6900`, character on port `6121`). You can run it with

```fish
cargo run -p korangar-testing --bin smoke
```
