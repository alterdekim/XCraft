# XCraft - A Modern Minecraft Launcher

XCraft is a custom Minecraft launcher written in Rust, designed to provide enhanced flexibility and customization. It supports a custom online mode, skin and cape editing, and seamless integration with MultiMC instances.

## Features

- **Custom Online Mode**: Authenticate and play on your own custom Minecraft servers.
- **Skin & Cape Editing**: Easily customize your in-game appearance.
- **MultiMC Support**: Load and manage MultiMC instances directly from XCraft.
- **One-Click Mod Installation**: Install Forge, Fabric, and Omniarchive versions effortlessly.
- **Portable**: You can run launcher in portable mode from flash drive to play your lovely game everywhere you want.

## Installation

### Download Stable Build
You can download the latest stable build of XCraft from our Jenkins CI server:
[Download from Jenkins](https://jenkins.awain.net/job/XCraft/lastStableBuild/).

### Build from Source

```sh
# Clone the repository
git clone https://github.com/yourusername/XCraft.git
cd XCraft

# Build the launcher
cargo build --release

# Run the launcher
./target/release/xcraft
```

## Usage
1. Launch XCraft.
2. Configure your custom authentication settings.
3. Manage skins and capes within the built-in editor.
4. Load your MultiMC instances for easy access.
5. Enjoy a seamless Minecraft experience!

## For Server Administrators
To make your Minecraft server compatible with XCraft, install the **XCraft-Auth** Spigot plugin. This plugin enables custom authentication and ensures seamless integration with XCraft's custom online mode.

## Roadmap
- [ ] Add support for more mod loaders (Fabric, Forge, etc.)
- [ ] Enhance logging and debugging features
- [ ] Cross-platform support improvements

## License
This project is licensed under the MIT License.