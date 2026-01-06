# MiniBit
A full blown Minecraft minigame server written in Rust using [Valence](https://github.com/valence-rs/valence).

![Screenshot](/images/lobby.png)

⚠️ **Warning:** This project is still in very early development and is not ready for production use. Many features are missing and bugs are present.

## Features
- **Ready out of the box** - Run the server with pre-written minigames and a fully functional lobby and proxy server.
- **Performance** - Leveraging the speed of Rust and Bevy ECS, the server can handle hundreds of players with ease.

## Getting Started
1. Clone the repo
2. Run `configure_servers.sh` to initialize base configuration and secrets
3. Run `docker compose up`
4. You're done! You can join the server now at port 25565 and try out the minigames!
