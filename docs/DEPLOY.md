# Deploying Primer to primer.burvy.dev

This sets Primer up on the home Windows server the same way `plgroup-api` runs:
Cloudflare proxies the host, an Origin Rule sends it to one port on the PC, and
TLS uses a Cloudflare Origin Certificate with "Full (strict)". The one addition
is **Caddy**, a single-file web server that holds the certificate and forwards
to Primer. That keeps `primer-server` listening on `127.0.0.1` only, so nothing
can reach it except through Cloudflare and Caddy.

```text
child's browser ──https──▶ Cloudflare ──https :3130──▶ Caddy (this PC) ──http──▶ primer-server 127.0.0.1:3131
                           (proxied,       (Origin Rule;        (origin cert,          (SQLite in
                            Access while    firewall: Cloudflare  HSTS)                 C:\burvy\data\primer)
                            in preview)     addresses only)
```

Scripts live in `tools/deploy/`. Every script takes `-DryRun`, which only
prints its steps. Paths and ports are set once, in `tools/deploy/config.ps1`.

| What | Where |
|---|---|
| The app (exe, web app, book, assets, scripts) | `C:\burvy\primer\` |
| Database `primer.db`, `backups\`, `logs\` | `C:\burvy\data\primer\` |
| Jev key (raw key only) | `C:\burvy\secrets\primer\jev-key.env` |
| Origin certificate `origin.pem` + `origin.key` | `C:\burvy\certs\primer.burvy.dev\` |
| Caddy | `C:\burvy\caddy\caddy.exe` |
| Public port (Cloudflare → Caddy) / local port (Caddy → server) | `3130` / `127.0.0.1:3131` |

## A. Before you start: preview, not public

There is no sign-in yet (slice 10). Until there is:

- The normal build has the `dev-local` feature: **one built-in child that
  everyone shares**. Deploy it only behind **Cloudflare Access** (step 6), so
  only the emails you list can open the site.
- `build.ps1 -Public` builds without `dev-local`. Today that build has no
  children at all (`/api/child/dev/next` answers "no such child"), so it is
  only for after slice 10.

Before going public, slice 10 must set its session cookie as
`__Host-session; Secure; HttpOnly; SameSite=Lax; Path=/`. Caddy already sends
`Strict-Transport-Security`, so browsers only ever use https.

## B. One-time set-up (Cloudflare, router, server PC)

You do these by hand; no script touches DNS, Cloudflare or the router.

1. **Cloudflare DNS** (burvy.dev zone): add a **proxied** (orange cloud) record
   `primer`, pointing at the same target as `plgroup-api`.
2. **Cloudflare Origin Rule**: hostname equals `primer.burvy.dev` → destination
   port `3130`.
3. **Cloudflare Configuration Rule**: hostname equals `primer.burvy.dev` → SSL
   **Full (strict)**. The zone default stays Flexible, as for plgroup-api.
4. **Origin Certificate**: SSL/TLS → Origin Server → Create Certificate for
   `primer.burvy.dev`. On the server PC, save the two parts as
   `C:\burvy\certs\primer.burvy.dev\origin.pem` and `...\origin.key`.
5. **Router**: forward TCP `3130` to the server PC, as is done for 3120.
6. **Cloudflare Access** (Zero Trust → Access → Applications → Self-hosted):
   application `primer.burvy.dev`, policy "Allow", include the emails that may
   use the preview. Remove it only once sign-in exists.
7. **Caddy**: download the Windows amd64 build from
   <https://caddyserver.com/download> and save it as
   `C:\burvy\caddy\caddy.exe`.
8. **Jev key**: on the server PC, create `C:\burvy\secrets\primer\jev-key.env`
   containing only the raw key, the same as `jev-key.env` in the repo. Without
   it the server runs with the fake judge (it prints
   `no Jev key file: using the fake judge`). The key is never in `C:\burvy\primer\`
   and no script reads or copies it.

## C. Build (on the dev PC, in the repo root)

9. `.\tools\deploy\build.ps1 -DryRun` (check the steps)
10. `.\tools\deploy\build.ps1`

This writes `deploy\primer\`: a release `primer-server.exe`, `dist\` (the web
app, `trunk build --release`), `content\`, `assets\` without placeholder
voices, `tools\voice\` and `tools\deploy\`. It contains no key, no database
and no certificate.

11. Copy `deploy\primer\` to the server PC, for example to
    `C:\burvy\incoming\primer\`.

## D. Install (on the server PC, admin PowerShell, first time only)

12. `cd C:\burvy\incoming\primer; Set-ExecutionPolicy -Scope Process Bypass`
    (lets this window run the copied scripts; the setting ends when the window closes)
13. `.\tools\deploy\install.ps1 -DryRun`
14. `.\tools\deploy\install.ps1`

It makes the folders, copies the app to `C:\burvy\primer\`, writes
`C:\burvy\primer\Caddyfile`, adds the firewall rule "Primer (Cloudflare only)"
(port 3130, Cloudflare's published address ranges only), and registers three
scheduled tasks that run as SYSTEM:

- **Primer server**, at start-up: `run.ps1` sets the environment and runs the
  server. It restarts the server if it stops.
- **Primer Caddy**, at start-up.
- **Primer backup**, daily at 03:30.

Last, it makes the placeholder voices with this PC's Windows voice
(`primer-server voices`) and starts the server and Caddy.

The environment `run.ps1` sets:

| Variable | Value |
|---|---|
| `PRIMER_ADDR` | `127.0.0.1:3131` (this PC only) |
| `DATABASE_URL` | `sqlite://C:/burvy/data/primer/primer.db` (made on first start) |
| `PRIMER_DIST` | `C:\burvy\primer\dist` |
| `PRIMER_CONTENT` | `C:\burvy\primer\content` |
| `PRIMER_ASSETS` | `C:\burvy\primer\assets` |
| `PRIMER_JEV_KEY_FILE` | `C:\burvy\secrets\primer\jev-key.env` |

## E. Check it works

15. `Invoke-WebRequest http://127.0.0.1:3131/ -UseBasicParsing` should give 200.
16. `Get-Content C:\burvy\data\primer\logs\primer-server.log -Tail 5` should end
    with `primer listening on http://127.0.0.1:3131`.
17. Open <https://primer.burvy.dev> from another network (e.g. a phone off
    Wi-Fi). Cloudflare Access asks for your email; then the book opens.

If step 17 fails but 15 works, check in order: the router forward (5), the
Origin Rule (2), Full (strict) (3), the certificate files (4), and
`Get-ScheduledTask 'Primer Caddy'`.

## F. Update to a new version

18. On the dev PC: `.\tools\deploy\build.ps1`, then copy `deploy\primer\` to
    `C:\burvy\incoming\primer\` on the server (replace the old copy).
19. On the server, in an admin PowerShell:
    `cd C:\burvy\incoming\primer; Set-ExecutionPolicy -Scope Process Bypass`
20. `.\tools\deploy\update.ps1 -DryRun`
21. `.\tools\deploy\update.ps1`

It backs up the database, stops the server, mirrors the new files into
`C:\burvy\primer\` (keeping the placeholder voices and the Caddyfile), makes
voices for any new lines, and starts the server. Database changes are applied
by the server itself on start (sqlx migrations).

## G. Backups and restore

- Daily copies: `C:\burvy\data\primer\backups\primer-<date>.db`, the newest 14
  kept. Copy that folder off the PC too (with the plgroup backups).
- A backup now: `C:\burvy\primer\tools\deploy\backup.ps1`
- With `sqlite3.exe` on PATH the copy is made live; without it the server is
  stopped for the moment of the copy.

To restore:

22. `Stop-ScheduledTask 'Primer server'; Get-Process primer-server -ErrorAction SilentlyContinue | Stop-Process`
23. `Copy-Item C:\burvy\data\primer\backups\primer-<date>.db C:\burvy\data\primer\primer.db -Force`
24. `Start-ScheduledTask 'Primer server'`

## H. Safety checklist

- [x] `primer-server` listens on `127.0.0.1` only (checked: a release build with
      these settings shows `127.0.0.1` as its only listening address).
- [x] Port 3130 is open to Cloudflare's addresses only, and Cloudflare is set to
      Full (strict) with an origin certificate.
- [x] The Jev key lives outside the app folder and is never copied or printed.
      `JevConfig` shows `[REDACTED]` in debug output, and the server's Jev log
      lines name only the error.
- [x] A child's typed explanation is sent to Jev and never stored or logged.
- [x] Placeholder voices are made on the server, never shipped.
- [ ] Before going public: sign-in (slice 10) with the cookie in part A, a
      `-Public` build, then remove Cloudflare Access.

## I. Take it down

25. `Stop-ScheduledTask 'Primer server'; Stop-ScheduledTask 'Primer Caddy'`
26. `Unregister-ScheduledTask 'Primer server','Primer Caddy','Primer backup' -Confirm:$false`
27. `Remove-NetFirewallRule -DisplayName 'Primer (Cloudflare only)'`

The data in `C:\burvy\data\primer\` stays until you delete it.
