# Korangar Networking

An opinionated wrapper around the `ragnarok_packets` crate.
This crate exposes a networking system that can run in a separate thread and maintain connections to the login, character, and map servers.

## Examples

### Chat bot

A small example of how you can use the Korangar networking system to implement a small chat bot.
It connects to the server as a client and uses `Ollama` to generate responses to incoming messages.

To run this example you need to set `USERNAME`, `PASSWORD`, and `CHARACTER_NAME` in the source code (you might also have to adjust `OLLAMA_ENDPOINT` and `OLLAMA_MODEL` depending on your setup).
Afterwards you can run it with
```bash
cargo run --example ollama-chat-bot --features=example
```

##### Note: Make sure that Ollama is serving and the model specified in `OLLAMA_MODEL` is installed, otherwise you will get a `404`
