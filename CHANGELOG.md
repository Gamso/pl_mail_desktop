# Changelog

All notable changes to plMail Desktop are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

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

[0.1.1]: https://github.com/Gamso/pl_mail_desktop/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/Gamso/pl_mail_desktop/releases/tag/v0.1.0
