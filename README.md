# [pkmc](https://github.com/Vulae/pkmc)

A bunch of stuff to build a Minecraft server.
See [pkmc](./pkmc/) for example server.

> [!IMPORTANT]
> This will only ever support the latest Minecraft version (currently 1.21.4)

I will **NOT** make anything use any async code (tokio/futures).
This was originally built to be fully single-threaded, but that _may_ change in the future.

## [Running](#running)

1. Clone repo
2. Start the server `cargo run --release`
3. Join the server (Default IP is `[::1]:52817`)

[pkmc/pkmc.toml](./pkmc/pkmc.toml) to configure.

## [Features](#features)

| Feature               | Implemented | Comment                                    |
| --------------------- | ----------- | ------------------------------------------ |
| Server List Ping      | ✅          |                                            |
| World Loading         | ✅          | (Single-threaded[^threaded-chunk-loading]) |
| World Editing         | ❌          |                                            |
| World Lighting        | ❌          |                                            |
| World Saving          | ❌          | (Probably never)                           |
| Dimensions/Multiworld | ❌          |                                            |
| Entities              | ❌          |                                            |
| Inventories           | ❌          |                                            |
| Resource Pack         | ❌          |                                            |
| Online Mode           | ❌          |                                            |
| Packet Compression    | ✅          |                                            |
| Players & Chat        | ❌          |                                            |
| Commands Definitions  | ❌          |                                            |
| Cookies 🍪            | ❌          |                                            |

Some extra features may be implemented inside the example server.

## [Project Layout](#project-layout)

- `pkmc-util` Some utility stuff for everything else to use.
- `pkmc-generated` Generate some code for `pkmc-defs/src/generated`
- `pkmc-defs` Definitions for blocks, packets, & other stuff.
- `pkmc-server` General building blocks for a server.
- `pkmc` Example/testing server, that may be used as reference.

## [License](#license)

[`MIT License`](./LICENSE)
License is very likely change to MIT-0 or 0BSD in the future, if I feel like it.

## [Notes](#notes)

[^threaded-chunk-loading]: Currently only single-threaded for world stuff, but definitely going to be multi-threaded in the future.
