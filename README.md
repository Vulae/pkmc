
# [pkmc](https://github.com/Vulae/pkmc)

Yet another Rust Minecraft server implementation.

Originally made so I can make a Minecraft server with custom features specifically for parkour.

I will **NOT** make anything use any async code (tokio/futures).
This was originally built to be fully single-threaded, but that *may* change in the future.

## [Running](#running)

1. Clone repo
2. Start the server `cargo run --release`
3. Join the server (Default IP is `[::1]:52817`)

## [Features](#features)

|        Feature        | Implemented |                Comment                |
|-----------------------|-------------|---------------------------------------|
| Server List Ping      | ✅          | [pkmc.toml](./pkmc.toml) to configure |
| World Loading         | ❌          | W.I.P.                                |
| World Editing         | ❌          |                                       |
| Dimensions/Multiworld | ❌          |                                       |
| Entities              | ❌          |                                       |
| Inventories           | ❌          |                                       |
| Resource Pack         | ❌          |                                       |
| Online Mode           | ❌          |                                       |
| Packet Compression    | ✅          | [pkmc.toml](./pkmc.toml) to configure |
| Players & Chat        | ❌          |                                       |
| Commands              | ❌          |                                       |
| Cookies 🍪            | ❌          |                                       |
| Terminal Interface    | ❌          |                                       |
| WASM Plugins          | ❌          | Maybe not [^wasm-plugins]             |

[^wasm-plugins]: Still don't know if I want this to be something you can make plugins for, or just a server you can just modify directly instead.

List of features that very likely will never be implemented:
- World Saving
- Command Blocks
- Redstone
- Vanilla World Ticking System
- Liquid Physics
- Entity Behavior / AI / Pathfinding
- Vanilla-like Worldgen


## [License](#license)

[`MIT License`](./LICENSE)
License is very likely change to MIT-0 or 0BSD in the future, if I feel like it.

## [Notes](#notes)

