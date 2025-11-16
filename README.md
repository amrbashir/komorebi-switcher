<p align="center"><img src="./assets/icon.svg" width="125" /></p>

# komorebi-switcher

A minimal workspace switcher for the [Komorebi](https://github.com/LGUG2Z/komorebi/) tiling window manager, seamlessly integrated the Windows 10/11 taskbar.

![Image showcasing komorebi switcher in Windows 11 dark mode](assets/screenshots/taskbar-dark.jpg)
![Image showcasing komorebi switcher in Windows 11 light mode](assets/screenshots/taskbar-light.jpg)

## Install

<a href="https://github.com/amrbashir/komorebi-switcher/releases/latest">
  <picture>
    <img alt="Get it on GitHub" src="https://github.com/LawnchairLauncher/lawnchair/blob/7336b4a0481406ff9ddd3f6c95ea05830890b1dc/docs/assets/badge-github.png" height="60">
  </picture>
</a>

Or through PowerShell:

```powershell
irm "https://github.com/amrbashir/komorebi-switcher/releases/latest/download/komorebi-switcher-setup.exe" -OutFile "komorebi-switcher-setup.exe"
& "./komorebi-switcher-setup.exe"
```

## Usage

- <kbd>Left Click</kbd> any workspace to switch to it.
- <kbd>Right Click</kbd> to open the context menu:

  - **Move & Resize**: Open the move and resize dialog.

    ![Move and Resize panel](assets/screenshots/move-resize-panel.png)

  - **Quit**: close the switcher

> [!TIP]
> You can also open the context menu from the tray icon.

## Development

1. Install [Rust](https://rustup.rs/)
2. Run `cargo run`

## LICENSE

[MIT](./LICENSE) License
