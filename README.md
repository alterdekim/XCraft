<img align="left" width="100" height="100" src="https://w0n.zip/file/bWJVJa">

### XCraft
A Modern Minecraft Launcher

#

<div align="center">
    <img src="https://w0n.zip/file/Xe0zXb">
    <p>
        <small>
            Screenshot from Windows 10
        </small>
    </p>
</div>

XCraft is a custom Minecraft launcher written in Rust, designed to provide enhanced flexibility and customization. It supports a custom online mode, skin and cape editing, and seamless integration with MultiMC instances.

## Features

- **Custom Online Mode**: Authenticate and play on your own custom Minecraft servers.
- **Skin & Cape Editing**: Easily customize your in-game appearance.
- **MultiMC Support**: Load and manage MultiMC instances directly from XCraft.
- **One-Click Forge Installation**: Install Forge, Fabric versions effortlessly.
- **Portable**: You can run launcher in portable mode from flash drive to play your lovely game everywhere you want.

## Installation

### Download Stable Build
You can download the latest stable build of XCraft from releases section:
[Download from Gitea](https://gitea.awain.net/alterwain/XCraft/releases/latest).

### Build from Source

```sh
# Clone the repository
git clone https://gitea.awain.net/alterwain/XCraft.git
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
To make your Minecraft server compatible with XCraft, install the [**XCraftAuth**](https://gitea.awain.net/alterwain/XCraftAuth) Spigot plugin. This plugin enables custom authentication and ensures seamless integration with XCraft's custom online mode.

## Roadmap
- [ ] Cross-platform support improvements
- [ ] Fabric integration

## License
This project is licensed under the MIT License.