# [pkmc](https://github.com/Vulae/pkmc)

Yet another Rust Minecraft server implementation.

Originally made so I can make a Minecraft server with custom features specifically for parkour.

> [!IMPORTANT]
> This will only ever support the latest Minecraft version (currently 1.21.4)

I will **NOT** make anything use any async code (tokio/futures).
This was originally built to be fully single-threaded, but that _may_ change in the future.

## [Running](#running)

1. Clone repo
2. Start the server `cargo run --release`
3. Join the server (Default IP is `[::1]:52817`)

[pkmc.toml](./pkmc.toml) to configure.

## [Features](#features)

| Feature               | Implemented | Comment                   |
| --------------------- | ----------- | ------------------------- |
| Server List Ping      | ✅          |                           |
| World Loading         | ✅          | (Single-threaded)         |
| World Editing         | ❌          |                           |
| Dimensions/Multiworld | ❌          |                           |
| Entities              | ❌          |                           |
| Inventories           | ❌          |                           |
| Resource Pack         | ❌          |                           |
| Online Mode           | ❌          |                           |
| Packet Compression    | ✅          |                           |
| Players & Chat        | ❌          |                           |
| Commands              | ❌          |                           |
| Cookies 🍪            | ❌          |                           |
| Terminal Interface    | ❌          |                           |
| WASM Plugins          | ❌          | Maybe not [^wasm-plugins] |

[^wasm-plugins]: Still don't know if I want this to be something you can make plugins for, or just a server you can just modify directly instead.

List of features that very likely will never be implemented:

- World Saving
- Command Blocks
- Redstone
- Vanilla World Ticking System
- Liquid Physics
- Entity Behavior / AI / Pathfinding
- Vanilla-like Worldgen

## [Project Layout](#project-layout)

`pkmc-util` Some utility stuff for everything else to use.
`pkmc-generated` Generate some code for `pkmc-defs/src/generated`
`pkmc-defs` Definitions for blocks, packets, & other stuff.
`pkmc-server` General building blocks for a server.
`pkmc` Testing server, that may be used as reference.

## [License](#license)

[`MIT License`](./LICENSE)
License is very likely change to MIT-0 or 0BSD in the future, if I feel like it.

## [Notes](#notes)
