# Changelog

## [Unreleased]

### Added

- Remove logs older than a month.

### Fixed

- Fixed incorrect size when monitor DPI changes.
- Fixed dragging mode sometimes not working.

  There is still a bug that entering drag mode consecutively won't work, you need to left-click a workspace at least one time
  before entering drag mode again.

## [0.4.2] - 2025-2-24

### Added

- Add a slightly dark transparent background when in dragging mode.

## [0.4.1] - 2025-2-22

### Fixed

- Reduce the initial memory usage of the app from 430mb to 110mb, idle memory usage should be 10mb, hopefully more improvements in future releases.

## [0.4.0] - 2025-2-20

### Added

- Added animations for active and non-empty workspace indicator.
- Added thin border around active and hovered workspace, in attempt to closely match with taskbar buttons style.

### Fixed

- Fix workspace indicating it is busy when in fact it is empty, like when closing its last window from a different workspace.

## [0.3.1] - 2025-2-19

### Fixed

- Reconnect to komorebi if socket is closed
- Fix Alt+Tab through windows on different workspaces not changing in the switcher.

### Added

- Add logging, saved in `%APPDATA%\komorebi-switcher`

## [0.3.0] - 2025-2-15

### Added

- New look that fits better with Windows 11 style.
- Clamp the switcher position in x direction so it always stays visible within the taskbar.

### Changed

- Changed dragging mode for the switcher to address bugs where `Esc` couldn't exist dragging mode.

  Now after choosing the "Move" context menu item, you need to click and drag the switcher around.
  It will save its position and exit out of dragging mode once you release the mouse click.

## [0.2.0] - 2025-2-12

### Added

- Use system accent Color
- Show context menu when right clicking
- Save and load position on startup

### Removed

- Removed Alt+Click to close the switcher, use the context menu
- Removed Shift+Click to move the switcher around, use the context menu

## [0.1.0] - 2025-2-12

- Inital Release
