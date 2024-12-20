
# [pkmc](https://github.com/Vulae/pkmc)

Yet another Rust Minecraft server implementation.

Originally made so I can make a Minecraft server with custom features specifically for parkour.

I will **NOT** make anything use any async code (tokio/futures).
This was originally built to be fully single-threaded, but that *may* change in the future.

## [Running](#running)

1. Clone repo
2. Start the server `cargo run --release`
3. Join the server (Default IP is `[::1]:52817`)

## [Planned Features](#planned-features)

- [X] [Server list ping](https://wiki.vg/Server_List_Ping)
- [X] Configuration file
- [ ] Console interface for server

- [ ] Registry & tag data
- [ ] Server resource pack
- [ ] World loading
- [ ] Chat messages & basic commands
- [ ] Display players
- [ ] Entities
- [ ] World interactions

- [ ] Plugins via WASM

- [ ] Online mode
- [X] Packet compression

## [License](#license)

[`MIT License`](./LICENSE)
License is very likely change to MIT-0 or 0BSD in the future, if I feel like it.

