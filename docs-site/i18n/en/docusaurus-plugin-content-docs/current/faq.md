---
sidebar_position: 100
---

# FAQ

## Sessions and connections

### SSH works, so why do Local Terminal, Telnet, and Serial behave differently?

Because NyaTerm supports multiple session types, and their capabilities are not identical:

- **SSH** — the most complete workflow, including SFTP, OTP, resource monitoring, proxy, jump host, and tunnels
- **Local Terminal** — local shell workflow only
- **Telnet** — lightweight remote terminal without SSH-specific features
- **Serial** — serial debugging, not an SSH network path

If you need the file explorer, remote resource monitoring, or OTP, make sure the current tab is an **SSH session**.

### Why is the file explorer missing for some sessions?

The file explorer depends on SFTP, so it is only available for **SSH sessions**.

These session types do not provide the remote file explorer:

- Local Terminal
- Telnet
- Serial

### Why can’t I see remote resource monitoring?

Check both of these:

1. The current tab is an **SSH session**
2. **Show Remote Resource Stats** is enabled in **Settings → Terminal**

Resource monitoring is on by default; turning this setting off also hides the Resource Monitor icon in the right activity bar.

### What should I do if the serial port list is empty?

Check that:

- The device is physically connected
- The operating system recognizes the serial port
- Another tool is not already holding the port open

When you reopen the port dropdown on the Serial tab, NyaTerm reloads the available ports.

## Terminal experience

### Why can’t I click action links?

Usually one of these is true:

1. **Action Links** is not enabled in **Settings → Terminal**
2. You are not using **Ctrl / Cmd + click**

Action links are disabled by default, and opening them requires a modifier key to avoid accidental activation.

### Why can’t I see keyword highlighting?

Keyword highlighting is disabled by default. Enable it first in **Settings → Terminal**, then confirm the current output actually matches one of the configured rules.

### Why are line numbers and timestamps not visible?

These are also optional enhancements. Enable them separately in **Settings → Terminal**.

## File transfer

### Why didn’t the auto-upload prompt appear after I opened a remote file?

The auto-upload prompt only appears in this workflow:

1. You choose **Open** on a remote file from the SSH file explorer
2. NyaTerm downloads it into a local temporary directory and starts watching it
3. You save that watched file in your local editor

If you copied the file elsewhere and edited that copy manually, NyaTerm no longer knows it maps back to the remote file.

### Why didn’t the file explorer follow my `cd` command automatically?

Auto-follow depends on terminal path tracking support for the session. If the current session does not support it, automatic sync is disabled and you need to trigger sync manually.

### Where do uploads and downloads go?

That depends on your transfer settings:

- If **ask every time** is enabled, NyaTerm prompts for a destination on each download
- Otherwise it uses the default download directory

You can also change the default download path and the default editor in settings.

## Security and authentication

### Why can I unlock the screen without entering a password?

Because screen lock is enabled, but **no master password is set yet**.

In the current behavior:

- With a master password: unlocking requires the master password
- Without a master password: unlocking can be done directly

### What if I forget the master password?

There is currently no built-in recovery flow for the master password. If your local data is protected by it and you can no longer provide the correct password, those protected sensitive settings cannot continue to be used in the original way.

Before making manual changes, back up `~/.nyaterm/` first, then decide how to rebuild local configuration.

### Where should OTP entries be managed?

Manage them centrally in the **OTP** tab of the **Security/Auth** panel, then bind them to individual SSH connections in the connection form.

## Import and migration

### Which clients can NyaTerm import sessions from?

Current supported imports are:

- Xshell (`.xts`)
- MobaXterm (`.mxtsessions`)
- WindTerm (`.sessions`)

After import, it is a good idea to review the username, port, authentication method, and whether proxy / jump host / OTP still needs to be configured.

### Where are NyaTerm’s config files stored?

Application configuration is stored in `~/.nyaterm/nyaterm.redb`, including settings, connections, keys, OTP data, tunnels, proxies, and history. When upgrading from Dragonfly, NyaTerm copies `~/.dragonfly/dragonfly.redb` on first launch; if the old environment only has `.dragonfly` JSON / text files, they are copied and imported into redb. The old `~/.dragonfly/` directory is kept as a rollback backup.
