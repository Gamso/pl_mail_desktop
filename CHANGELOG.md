# Changelog

All notable changes to plMail Desktop are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- **Notifications with buttons**, laid out like Thunderbird's: the account that received the mail
  ("you@example.com received 1 new message"), the subject, the sender, and **Reply**, **Mark
  read** and **Delete** buttons. Reply opens the reply form in pl_mail; Delete moves the message to
  the Trash, as in pl_mail. A click on the text still opens the message. A grouped notification
  (more than three at once) has no buttons.

### Fixed

- **The application did not start with Windows after a new version was installed by hand.** The
  "uninstall first" choice of the installer deletes Windows' autostart entry. It is now put back at
  launch whenever "Start with Windows" is on.

## [0.2.1] - 2026-10-25

### Fixed

- **Checking for updates failed** with "Could not fetch a valid release JSON from the remote", even
  though a release was published: GitHub answers 404 to the JSON-only request the updater sent for
  `latest.json`. The check now accepts any content type, and the download keeps its own.
  Version 0.2.0 cannot update itself because of this: install 0.2.1 by hand once.
- **The unread badge disappeared from the taskbar button** after the window was closed to the tray
  and opened again, and stayed away until the number changed. It is now painted again at every
  check and whenever the window is shown.

## [0.2.0] - 2026-10-25

### Added

- **Updates.** The application looks for a newer release on GitHub at launch (at most once a day,
  switchable in the settings) and from *Help → Check for updates…*. A dialog shows the release
  notes and offers to install, remind later or skip that version. Pre-releases are never offered.
  An installed copy downloads the installer, checks its signature against the key built into the
  application, and updates in place, keeping its shortcuts, taskbar pin and settings, then
  restarts. A portable copy only points to the release page. Versions before this one cannot
  update themselves: install this one by hand once.
- A portable `plMail-Desktop_X.Y.Z_x64-portable.exe` is attached to each release.

### Changed

- **At Windows logon the mail window now opens**, like in any mail client; before, only the tray icon
  appeared and the taskbar button stayed idle. The old behaviour is a new option, "Start in the tray
  only". If the network is not up yet, the application keeps trying for a minute before reporting
  that the server is unreachable.
- The unread count is no longer drawn on the tray icon, where it was too small to read. It stays on
  the taskbar icon and in the tray icon's tooltip.
- "Sync now" is no longer in the tray menu; it is still in the File menu of the mail window.

### Fixed

- The settings, about and error dialogs open centred over the mail window, and closing one gives the
  focus back to the mail window instead of leaving it behind other applications.

## [0.1.1] - 2026-10-24

### Fixed

- **Links in mails did nothing.** Clicking "Accept", "View profile" or any other link in a message
  had no effect: WebView2 does not report clicks on `target="_blank"` links, which pl_mail puts on
  every link of a mail. They now behave as expected:
  - a link to another site opens in your default browser;
  - a link to your own pl_mail server stays in the mail window;
  - a `mailto:` link opens a new message in pl_mail;
  - any other kind of link (`file:`, `tel:`, `javascript:`…) is ignored, since the content of a mail
    is untrusted.

### Added

- AGPL-3.0-or-later licence (`LICENSE`), the same as pl_mail.
- Tests for the link handling.

### Changed

- Updated `winreg` to 0.56.

## [0.1.0] - 2026-10-24

First release.

### Added

- Mail window showing your pl_mail server's web UI, with its own session.
- Windows notifications for new unread mail in the inbox, across all accounts; a click opens the
  message.
- Unread count badge on the taskbar and tray icons.
- Tray icon with a menu (open, sync now, settings, quit); closing a window keeps the application
  running.
- Thunderbird-style menu bar shown with `Alt`: File, View, Go, Tools and Help.
- "Server unreachable" page with the cause and a retry button.
- English, Deutsch, Français and Pirate English, following the Windows language by default.
- Optional handling of `mailto:` links, through Windows' default mail app setting.
- Start with Windows, and a setting for self-signed certificates.
- Windows installer for the current user, no administrator rights needed.

[0.2.1]: https://github.com/Gamso/pl_mail_desktop/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/Gamso/pl_mail_desktop/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/Gamso/pl_mail_desktop/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/Gamso/pl_mail_desktop/releases/tag/v0.1.0
