<p align="center"><img src="./assets/icon.svg" width="125" /></p>

# komorebi-switcher

A minimal workspace switcher for the [Komorebi](https://github.com/LGUG2Z/komorebi/) tiling window manager, seamlessly integrated the Windows 10/11 taskbar.

![Image showcasing komorebi switcher in Windows 11 dark mode](.github/image-1.jpg)
![Image showcasing komorebi switcher in Windows 11 light mode](.github/image-2.jpg)

## Install

[<img src='https://github.com/ycngmn/Nobook/blob/e2a7fa98e460ce8ebb241ea1e7bda2ebb33e05c0/images/get-it-on-github.png' alt='Get it on GitHub' height = "90">](https://github.com/amrbashir/komorebi-switcher/releases/latest)

Or through PowerShell:

```powershell
irm "https://github.com/amrbashir/komorebi-switcher/releases/latest/download/komorebi-switcher-setup.exe" -OutFile "komorebi-switcher-setup.exe"
& "./komorebi-switcher-setup.exe"
```

## Usage

- <kbd>Left Click</kbd> any workspace to switch to it.
- <kbd>Right Click</kbd> to open the context menu:

  - **Move & Resize**: Open the move and resize dialog.

    ![430847504-b1839a40-df9e-4685-aaeb-07410e9c379c](https://github.com/user-attachments/assets/20becf18-7e0c-4b9f-9de6-11ac79ef8408)

  - **Quit**: close the switcher

> [!TIP]
> You can also open the context menu from the tray icon.

## Development

1. Install [Rust](https://rustup.rs/)
2. Run `cargo run`

## LICENSE

[MIT](./LICENSE) License
