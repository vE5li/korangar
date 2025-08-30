# Korangar Networking

An opinionated wrapper around the `ragnarok_packets` crate.
This crate exposes a networking system that can run in a separate thread and maintain connections to the login, character, and map servers.

## Examples

### Chat bot

A small example of how you can use the Korangar networking system to implement a small chat bot.
It connects to the server as a client and uses `Ollama` to generate responses to incoming messages.

To run this example you need to set `USERNAME`, `PASSWORD`, and `CHARACTER_NAME` in the source code (you might also have to adjust `OLLAMA_ENDPOINT` and `OLLAMA_MODEL` depending on your setup).
Afterwards you can run it with

```fish
cargo run --example ollama-chat-bot
```

##### Note: Make sure that Ollama is serving and the model specified in `OLLAMA_MODEL` is installed, otherwise you will get a `404`

### Rescue my character

A tool to use when your character is stuck on a map that crashes Korangar. Just run the example, passing your login information and character name as such:

```fish
cargo run --example rescue-my-character -- -u <username> -p <password> -c <character_name>
```

If the provided information is correct you will see the message `[Success] Successfully rescued character` followed by the example terminating.
