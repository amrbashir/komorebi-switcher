<p align="center"><img src="./assets/icon.svg" width="125" /></p>

# komorebi-switcher

A minimal workspace switcher for the [Komorebi](https://github.com/LGUG2Z/komorebi/) tiling window manager, seamlessly integrated the Windows 10/11 taskbar or macOS menubar.

![Image showcasing komorebi-switcher in Windows 11 dark mode](assets/screenshots/taskbar-dark.jpg)
![Image showcasing komorebi-switcher in Windows 11 light mode](assets/screenshots/taskbar-light.jpg)
![Image showcasing komorebi-switcher in macOS menubar](assets/screenshots/menubar.png)

## Install

<a href="https://github.com/amrbashir/komorebi-switcher/releases/latest">
  <picture>
    <img alt="Get it on GitHub" src="https://github.com/LawnchairLauncher/lawnchair/blob/7336b4a0481406ff9ddd3f6c95ea05830890b1dc/docs/assets/badge-github.png" height="60">
  </picture>
</a>

Or using scoop (Windows):

```powershell
scoop bucket add amrbashir https://github.com/amrbashir/scoop-bucket
scoop install komorebi-switcher
```

Or using Homebrew (macOS):

```bash
brew install amrbashir/tap/komorebi-switcher
```

## Config

The config is located at `~/.config/komorebi-switcher.toml`. You can edit this file directly
or use the settings window accessible from the context menu.

```toml
# Global settings
show_layout_button    = false
hide_empty_workspaces = false

# Settings for each monitor (Windows only for now)
#   Syntax is [monitors.<id>] where <id> is one of:
#     - serial_number_id
#     - device_id
#     - name
#   The app will try to match in the above order, depending on what info is available,
#   Run `komorebic monitor-information` to get info about your monitors
[monitors.0]
show_layout_button    = false    # Can be removed to use the global setting
hide_empty_workspaces = false    # Can be removed to use the global setting
auto_width            = true
auto_height           = true
x                     = 0
y                     = 0
width                 = 200      # Ignored if `auto_width` is enabled
height                = 40       # Ignored if `auto_height` is enabled
```

## Development

1. Install [Rust](https://rustup.rs/)
2. Run `cargo run`

## LICENSE

[MIT](./LICENSE) License
