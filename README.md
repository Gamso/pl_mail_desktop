# plMail Desktop

[![CI](https://github.com/Gamso/pl_mail_desktop/actions/workflows/ci.yml/badge.svg)](https://github.com/Gamso/pl_mail_desktop/actions/workflows/ci.yml)

**English** | [Français](README.fr.md)

A Windows desktop client for [pl_mail](https://github.com/karatektus/pl_mail), the self-hosted
mail client. It shows your pl_mail inbox in a window of its own and adds what a browser tab does
poorly: Windows notifications, an unread count on the icon, start with Windows, and a tray icon.

> Working name: "plMail Desktop". This project is independent of pl_mail.

## Features

- **Mail window**: your pl_mail server's web UI, with its own session (2FA and SSO work as they do
  in a browser).
- **Windows notifications** for each new unread message in the inbox, across all accounts. More
  than three at once are grouped into a single notification. Clicking a notification opens the
  message it is about (or the inbox, for a grouped notification). Each notification names the
  account that received the mail, the subject and the sender, and has **Mark read** and **Delete**
  buttons (delete moves the message to the Trash, as in pl_mail).
- **Unread badge**, Thunderbird-style, on the taskbar icon (the count is also in the tray icon's
  tooltip).
- **Tray icon**: left-click opens the mail, right-click opens the menu (open, settings, quit). Closing a window does not quit the application.
- **Menu bar on Alt**: a bare press of `Alt` shows or hides the bar, as in Thunderbird. It closes
  again after each choice.
  - *File*: new message, sync now, print, close window, quit.
  - *View*: reload, back and forward, zoom (50 % to 300 %), full screen.
  - *Go*: inbox, starred, snoozed, drafts, sent, archive, spam, trash, all mail, calendar,
    contacts.
  - *Tools*: filters, templates, integrations, app passwords, plMail settings, administration
    (administrators only: an ordinary account is refused by the server) and client settings.
  - *Help*: pl_mail's keyboard shortcuts and what's new, open the data folder, about.
  - Thunderbird's *Message* menu (reply, archive, tag…) is not here: it acts on the selected
    message, which lives inside the web page. Use pl_mail's own buttons and shortcuts.
- **"Server unreachable" page** with the cause and *Try again* / *Edit settings* buttons, instead of
  the browser's error page.
- **Four languages**, the same as plMail: English, Deutsch, Français and Pirate English. The
  application follows the Windows language by default; change it in the settings.
- **Links in mails** open in your default browser (links to your own pl_mail server stay in the
  window, and `mailto:` links open a new message).
- **Modal dialogs**: settings, about and error windows lock the mail window while they are open.
- **Updates**: the application looks for a newer release on GitHub at launch (at most once a day)
  and from *Help → Check for updates…*. Pre-releases are never offered. An installed copy
  downloads the signed installer and updates in place, keeping its shortcuts, taskbar pin and
  settings, then restarts; you can also skip a version.
- **`mailto:` links** (optional): once enabled in the settings, the application is offered to
  Windows as a mail app. Choose it once in *Settings → Apps → Default apps*, and a click on a
  `mailto:` link anywhere opens a new message in pl_mail. pl_mail only reads the recipient for now
  (`/compose/new?to=…`); the subject and body are passed along and will be used as soon as the
  server reads them.

## Requirements

- Windows 10 or 11 with WebView2 (already part of Windows 11).
- A reachable pl_mail server (HTTPS, or HTTP on a trusted network).
- A pl_mail **app password**: *Settings → App passwords*. Create one and copy it right away, it is
  shown only once.

## Installation

Run the installer `plMail Desktop_<version>_x64-setup.exe` (see [Building](#building)). It installs
for the current user, without administrator rights, and adds a Start menu shortcut.

On first launch, the welcome page asks for the server address and the app password. The connection
is tested before anything is saved.

## Settings

| Setting | Meaning |
|---|---|
| Server address | `https://mail.example.com` |
| App password | Stored in the Windows Credential Manager, never in a file. Leave empty to keep the stored one. |
| Language | Automatic (Windows), English, Deutsch, Français, Pirate English |
| Check frequency | 60 s by default, 15 s minimum |
| Self-signed certificate | Only enable for a server you trust (local network) |
| Start with Windows | The application starts at logon and opens the mail window (it waits up to a minute for the network) |
| Start in the tray only | With *Start with Windows*: stay in the tray at logon instead of opening the window |
| Handle `mailto:` links | Registers the application with Windows as a mail app (current user only) |
| Check for updates automatically | On by default; contacts github.com once a day |

Where the data lives:

- settings: `%APPDATA%\fr.gamso.plmailclient\settings.json`
- password: Credential Manager, entry `app-password.plmail-client`
- mail window session: `%LOCALAPPDATA%\fr.gamso.plmailclient\webview-plmail`

To start over: quit the application and delete these three items.

## How it works

- The application talks to pl_mail over **JMAP** (`/.well-known/jmap`), authenticated with the app
  password as a `Bearer` token. This is the API pl_mail provides for third-party clients.
- *Sync now* (File menu) presses pl_mail's own sync button, which makes the server
  fetch from Gmail, IMAP and the others. The application then re-reads the server to update the
  badge and the notifications.
- Every 60 seconds (adjustable): `Email/changes` lists the mail that appeared since the last
  state, `Email/get` reads its sender, subject and preview, and `Mailbox/get` gives the inbox's
  unread count.
- At startup the current state is simply remembered: mail that is already there does not trigger a
  notification.
- No permanent connection (SSE): each one would hold a PHP worker on the server, which pl_mail's
  client guide advises against on a small personal server.

## Updates

- The check reads `latest.json` from the newest GitHub release that is neither a draft nor a
  pre-release (`…/releases/latest/download/latest.json`), so betas are never offered.
- The installer is signed with the project's updater key and the application refuses an update whose
  signature does not match the public key built into it.
- The update runs the installer in update mode: the previous version is **not** uninstalled first,
  so your shortcuts, taskbar pin, settings and start-with-Windows entry stay as they are.
- A **portable** copy (the `…-portable.exe` file, or a development build) cannot install an update
  itself: it tells you a new version exists and opens the release page.
- Versions before the first one with the updater cannot update themselves; install that one by hand.

## Security and privacy

- The mail window loads a remote page: it has **no access** to the application's internal
  commands.
- The Alt key is read from WebView2's keyboard events, only while the mail window has focus. There
  is no global keyboard hook.
- The application talks to your server, and to github.com to look for updates (turn that off in the
  settings). It opens GitHub in your browser if you click the link on the About page.

## Known limitations

- Windows only.
- No offline mode: the mail window is the web UI.
- A `mailto:` link fills in the recipient only: pl_mail does not read the subject, body, Cc or Bcc yet.
- If the server becomes unreachable while the mail window is already open, the browser's own error
  page is shown.
- New mail is detected by polling: up to a minute of delay.

## Development

Requirements: Node.js LTS, Rust (MSVC toolchain) and the Visual Studio C++ Build Tools.

```powershell
npm install
npm run tauri dev
```

### Building

```powershell
npm run tauri build
```

The installer is written to `src-tauri\target\release\bundle\nsis\`. The build also signs it for
the updater, which needs the private key in the environment (`TAURI_SIGNING_PRIVATE_KEY`, and
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`). For a local build without the key:

```powershell
npm run tauri build -- --config '{"bundle":{"createUpdaterArtifacts":false}}'
```

### Tests and continuous integration

```powershell
npm test            # Rust tests: translations, JMAP client (mock server), badge
npm run typecheck   # TypeScript
```

GitHub Actions (`.github/workflows/`):

- `ci.yml`: on every push to `main` and every pull request, builds the frontend, checks formatting
  (`cargo fmt`) and warnings (`cargo clippy`), and runs the tests, on Windows.
- `release.yml`: pushing a `vX.Y.Z` tag runs the tests, builds the Windows installer, signs it for
  the updater, and attaches the installer, its signature, `latest.json` and a portable exe to a
  **draft release** to publish by hand. It stops early if the tag differs from the version in
  `src-tauri/tauri.conf.json`, if the signing key is missing, or if `CHANGELOG.md` has no
  `## [X.Y.Z]` section: that section becomes the release notes, which the updater shows to users.
  A tag with a dash (`v0.3.0-beta.1`) is published as a pre-release and never offered by the updater.

One-time setup: generate the updater key pair, put the public key in
`plugins.updater.pubkey` of `src-tauri/tauri.conf.json`, and store the private key and its password
as the repository secrets `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

```powershell
npm run tauri signer generate -- -w "$HOME/.tauri/plmail-desktop.key"
```

Keep the private key out of the repository, and keep a backup: **without it, installed copies can
no longer be updated**.

To cut a release, add a section to `CHANGELOG.md`, update the version in
`src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` and `package.json`, merge, then:

```powershell
git tag -a v0.2.0 -m "plMail Desktop 0.2.0"
git push origin v0.2.0
```

The graphical interface (windows, tray, menu bar) is not tested automatically: check it by hand.

### Layout

| Path | Contents |
|---|---|
| `index.html`, `src/main.ts`, `src/styles.css` | Settings, about and error window |
| `src/i18n.ts` | Text of those windows (en, de, fr, pirate) |
| `src-tauri/src/lib.rs` | Application: windows, tray, menus, settings, check loop |
| `src-tauri/src/jmap.rs` | Minimal JMAP client |
| `src-tauri/src/i18n.rs` | Rust-side text: tray, menus, notifications, errors |
| `src-tauri/src/badge.rs` | Drawing of the unread badge |
| `src-tauri/src/altkey.rs` | Alt key detection inside the mail window |
| `src-tauri/src/*_tests.rs` | Tests (translations, JMAP client against a mock server) |
| `assets-src/` | Source logo |

To rename the application: `APP_NAME` in `src-tauri/src/lib.rs` and `productName` in
`src-tauri/tauri.conf.json`.

## Credits and license

- Built with [Tauri](https://tauri.app/) 2.
- The logo is pl_mail's, distributed under the AGPL-3.0 license.
- License: [AGPL-3.0-or-later](LICENSE), the same as pl_mail.
