# MiniBit
A full blown Minecraft minigame server written in Rust using [Valence](https://github.com/valence-rs/valence).

![Screenshot](/images/lobby.png)

⚠️ **Warning:** This project is still in very early development and is not ready for production use. Many features are missing and bugs are present.

## Features
- **Ready to use** - Run the server with pre-written a functional lobby, minigames, and proxy server.
- **Performance** - Leveraging the speed of Rust and Bevy ECS, the server can handle hundreds of players with ease.

## Getting Started
1. Clone the repo and navigate to the project directory.
2. Start up the proxy server with `gradlew runVelocity` in the `velocity` directory.
3. Configure each minigame to use the proxy server by changing their `server.json` file. Make sure to give them each a unique port!
4. Configure the proxy to use the minigame servers by modifying the `velocity.toml` file.
5. Run the lobby with `cargo run --bin lobby` in the `run/lobby` directory.
6. Run any minigame with `cargo run --bin <minigame>` in the `run/<minigame>` directory.

## FAQ
- **Why is the getting started process so complicated?** - The getting started process is mainly geared towards developers as the project is still in early development. In the future, we plan to provide a more user-friendly experience.