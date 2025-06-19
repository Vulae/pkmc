
# Usage

1. CWD at `pkmc/`
2. Extract minecraft definitions `cargo run -p pkmc-generated-extractor -- --release 1.21.6 --output pkmc-generated/assets/`

This can also be done manually:

1. [Download Minecraft server.jar](https://www.minecraft.net/download/server) for version.
2. Run extraction command `java -DbundlerMainClass=net.minecraft.data.Main -jar server.jar --all --output path/to/pkmc/pkmc-generated/assets/`

# Layout

- `extractor` Binary to download & extract definitions from Minecraft server jar.
- `proc` Procedural macros to convert Minecraft definitions into code.
- `generated` Generated code from the procedural macros.

# TODO

Make pkmc-generated-generated have a build script that automatically downloads & extracts Minecraft server.jar

