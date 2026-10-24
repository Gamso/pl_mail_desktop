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
  message it is about (or the inbox, for a grouped notification).
- **Unread badge**, Thunderbird-style, on the taskbar icon and on the tray icon (the count is also
  in the tray tooltip).
- **Tray icon**: left-click opens the mail, right-click opens the menu (open, sync now, settings,
  quit). Closing a window does not quit the application.
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
| Start with Windows | At logon the application stays in the tray without opening a window |
| Handle `mailto:` links | Registers the application with Windows as a mail app (current user only) |

Where the data lives:

- settings: `%APPDATA%\fr.gamso.plmailclient\settings.json`
- password: Credential Manager, entry `app-password.plmail-client`
- mail window session: `%LOCALAPPDATA%\fr.gamso.plmailclient\webview-plmail`

To start over: quit the application and delete these three items.

## How it works

- The application talks to pl_mail over **JMAP** (`/.well-known/jmap`), authenticated with the app
  password as a `Bearer` token. This is the API pl_mail provides for third-party clients.
- *Sync now* (File menu or tray icon) presses pl_mail's own sync button, which makes the server
  fetch from Gmail, IMAP and the others. The application then re-reads the server to update the
  badge and the notifications.
- Every 60 seconds (adjustable): `Email/changes` lists the mail that appeared since the last
  state, `Email/get` reads its sender, subject and preview, and `Mailbox/get` gives the inbox's
  unread count.
- At startup the current state is simply remembered: mail that is already there does not trigger a
  notification.
- No permanent connection (SSE): each one would hold a PHP worker on the server, which pl_mail's
  client guide advises against on a small personal server.

## Security and privacy

- The mail window loads a remote page: it has **no access** to the application's internal
  commands.
- The Alt key is read from WebView2's keyboard events, only while the mail window has focus. There
  is no global keyboard hook.
- The application sends nothing anywhere but to your server (and opens GitHub if you click the
  link on the About page).

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

The installer is written to `src-tauri\target\release\bundle\nsis\`.

### Tests and continuous integration

```powershell
npm test            # Rust tests: translations, JMAP client (mock server), badge
npm run typecheck   # TypeScript
```

GitHub Actions (`.github/workflows/`):

- `ci.yml`: on every push to `main` and every pull request, builds the frontend, checks formatting
  (`cargo fmt`) and warnings (`cargo clippy`), and runs the tests, on Windows.
- `release.yml`: pushing a `vX.Y.Z` tag (equal to the version in `src-tauri/tauri.conf.json`) runs
  the tests, builds the Windows installer and attaches it to a **draft release** to publish by
  hand.

To cut a release, update the version in `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` and
`package.json`, then:

```powershell
git tag v0.1.0
git push origin v0.1.0
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
