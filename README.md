# [pkmc](https://github.com/Vulae/pkmc)

Simple modular building blocks for Minecraft Java Edition minigame-like servers.
See [examples/basic](./examples/basic) for example server.

> [!WARNING]
> This project is in early development, expect ALOT of code to change.

> [!IMPORTANT]
> This will only ever support the latest Minecraft Java Edition release version (currently 1.21.5)

This will **NOT** make anything use any async code (tokio/futures).
This was originally built to be fully single-threaded, but that _may_ change in the future.

## [Running Example Server](#running-example-server)

1. Clone repo
2. Extract Minecraft data `cargo run -p pkmc-generated-extractor -- --release 1.21.5 --output pkmc-generated/assets/`
3. Start the server `cargo run -p example-basic --release`
4. Join the server (Default IP is `[::1]:52817`)

[examples/basic/config.toml](./examples/basic/config.toml) to configure.

## [Features](#features)

| Feature               | Implemented | Comment                                    |
| --------------------- | ----------- | ------------------------------------------ |
| Server List Ping      | ✅          |                                            |
| World Loading         | ✅          | (Single-threaded[^threaded-chunk-loading]) |
| World Editing         | ✅          |                                            |
| World Lighting        | ❌          |                                            |
| World Saving          | ❌          | (Probably never)                           |
| Dimensions/Multiworld | ✅          |                                            |
| Entities              | ❌          |                                            |
| Inventories           | ❌          |                                            |
| Resource Pack         | ❌          |                                            |
| Online Mode           | ✅          |                                            |
| Packet Compression    | ✅          |                                            |
| Players & Chat        | ✅          | (Unsigned chat)                            |
| Commands              | ✅          | (Basic implementation)                     |
| Cookies 🍪            | ❌          |                                            |

Some extra features may be implemented inside the example server.
(pkmc will never try to implement many vanilla things, such as: vanilla world gen, redstone, world ticking)

## [Project Layout](#project-layout)

- `pkmc-util` Some utility stuff for everything else to use.
- `pkmc-generated` Extract Minecraft server.jar data & convert to code.
- `pkmc-defs` Definitions for blocks, packets, & other stuff.
- `pkmc-server` General building blocks for a server.
- `examples/*` Some examples & some testing stuff.

## [Goals](#goals)

Make a simple framework for Minecraft minigame servers.

Published as a crate once more features are implemented and things are more fleshed out.

## [License](#license)

[`MIT License`](./LICENSE)
License is very likely change to MIT-0 or 0BSD in the future.

## [Notes](#notes)

[^threaded-chunk-loading]: Currently only single-threaded for world stuff, but definitely going to be multi-threaded in the future.
