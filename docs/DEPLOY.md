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
prints its steps. Paths, ports and the Google client id are set once, in
`tools/deploy/config.ps1`.

| What | Where |
|---|---|
| The app (exe, web app, book, assets, scripts) | `C:\burvy\primer\` |
| Database `primer.db`, `backups\`, `logs\` | `C:\burvy\data\primer\` |
| Jev key (raw key only) | `C:\burvy\secrets\primer\jev-key.env` |
| Origin certificate `origin.pem` + `origin.key` | `C:\burvy\certs\primer.burvy.dev\` |
| Caddy | `C:\burvy\caddy\caddy.exe` |
| Public port (Cloudflare → Caddy) / local port (Caddy → server) | `3130` / `127.0.0.1:3131` |

## A. How parents sign in

Children have no login. A parent signs in and gets a session token. The server
offers one of two sign-ins, chosen when it **starts** (there is one kind of
build):

| Settings | Sign-in | Where |
|---|---|---|
| `PRIMER_GOOGLE_CLIENT_ID=<id>` | Google | the server |
| `PRIMER_DEV_SIGNIN=1`, no client id | "Sign in (local test)": anyone who opens the page is let in as the same parent | a laptop only |
| neither | none: nobody can sign in | |
| both | the server refuses to start | |

The deploy scripts set the client id from `config.ps1` and always clear
`PRIMER_DEV_SIGNIN`, so the local test sign-in can never be on at
primer.burvy.dev. `PRIMER_DEV_SIGNIN` must be exactly `1`; any other value
(`true`, `yes`) stops the server with an error instead of being read as "off".

**Running on a laptop** (the prototype): start the server with the flag, for
example `$env:PRIMER_DEV_SIGNIN = '1'; cargo run -p primer-server`. Without it
the sign-in page cannot sign you in.

**Preview, not public, for now.** Keep Cloudflare Access (step 6) on until the
Google client id exists and you have decided to open the site. Without a
client id nobody can sign in at all; with one, Access still limits who sees
the site while it is a preview. Remove Access only when going public (part H).

### The session token: Bearer header, kept in `localStorage`

The web app keeps the token in the browser's `localStorage` and sends it as
`Authorization: Bearer <token>` (Kevin's `crates/web/src/api.rs`). It is not a
cookie. The token is 128 random bits, lasts 30 days, and is deleted on the
server at sign-out or when the account is deleted.

**Judgement: acceptable for v1.**

- What it protects well: no cookie is sent by the browser on its own, so
  another site cannot make a signed-in parent's browser act for them (CSRF).
  Phone apps (`docs/MOBILE.md`) need a Bearer token anyway.
- The weakness: script running in the page can read `localStorage`, so a
  cross-site scripting (XSS) hole would let an attacker copy the token. An
  `HttpOnly` cookie cannot be read by script, but an XSS hole could still act
  as the parent while the page is open, so the cookie only narrows the harm.
- Why the risk is low today: the page runs only our own scripts and Google's
  sign-in script; Leptos escapes text; raw HTML (`inner_html`) is used only for
  our own drawings and maths, never for anything a person types.
- The cheaper, bigger win is a Content-Security-Policy header (follow-up card
  below). A `__Host-session` cookie is not needed for v1.

Follow-up card (not done here; Kevin's area and Caddy):

> **Harden the page against script injection (CSP).** Add a
> `Content-Security-Policy` to the Caddyfile that `install.ps1` writes,
> allowing only `'self'`, `'wasm-unsafe-eval'` for the web app, and
> `https://accounts.google.com/gsi/` for Google's button (script, frame,
> connect, style), with `frame-ancestors 'none'` and `object-src 'none'`. Check
> that Trunk's output has no inline script, or add its hash. Test the sign-in
> page, a lesson with voice and drawings, and the grown-up gate in Chrome and
> Safari with the console open. Also on the sign-in page: show the local test
> button only when `/api/auth/config` says `dev_sign_in: true`, and otherwise
> "Sign-in is not set up on this server" (today the button shows whenever there
> is no Google client id, and the server answers 404).

## B. One-time set-up (Cloudflare, Google, router, server PC)

You do these by hand; no script touches DNS, Cloudflare, Google or the router.

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
   use the preview. Remove it only when going public.
7. **Google client id** (Google Cloud Console → APIs & Services):
   - OAuth consent screen: app name "Primer", your support email, scopes
     `openid` and `email` only.
   - Credentials → Create credentials → OAuth client ID → **Web application**.
     Authorised JavaScript origins: `https://primer.burvy.dev`. No redirect
     URIs are needed.
   - Put the client id (`…apps.googleusercontent.com`) in `GoogleClientId` in
     `tools/deploy/config.ps1` and commit it. It is not a secret: every
     visitor's browser sees it. The client **secret** is not used; do not
     download or store it.
8. **Caddy**: download the Windows amd64 build from
   <https://caddyserver.com/download> and save it as
   `C:\burvy\caddy\caddy.exe`.
9. **Jev key**: on the server PC, create `C:\burvy\secrets\primer\jev-key.env`
   containing only the raw key, the same as `jev-key.env` in the repo. Without
   it the server runs with the fake judge (it prints
   `no Jev key file: using the fake judge`). The key is never in `C:\burvy\primer\`
   and no script reads or copies it.

## C. Build (on the dev PC, in the repo root)

10. `.\tools\deploy\build.ps1 -DryRun` (check the steps)
11. `.\tools\deploy\build.ps1`

This writes `deploy\primer\`: a release `primer-server.exe`, `dist\` (the web
app, `trunk build --release`), `content\`, `assets\` without placeholder
voices, `tools\voice\` and `tools\deploy\` (with your `config.ps1`). It
contains no key, no database and no certificate. If `GoogleClientId` is empty
it says so: that build cannot be signed into.

12. Copy `deploy\primer\` to the server PC, for example to
    `C:\burvy\incoming\primer\`.

## D. Install (on the server PC, admin PowerShell, first time only)

13. `cd C:\burvy\incoming\primer; Set-ExecutionPolicy -Scope Process Bypass`
    (lets this window run the copied scripts; the setting ends when the window closes)
14. `.\tools\deploy\install.ps1 -DryRun`
15. `.\tools\deploy\install.ps1`

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

The Caddyfile sends these headers on every page: `Strict-Transport-Security`
(https only), `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY` (no
other site may show Primer in a frame) and
`Referrer-Policy: strict-origin-when-cross-origin` (other sites see only the
host name, which Google's sign-in needs).

The environment `run.ps1` sets:

| Variable | Value |
|---|---|
| `PRIMER_ADDR` | `127.0.0.1:3131` (this PC only) |
| `DATABASE_URL` | `sqlite://C:/burvy/data/primer/primer.db` (made on first start) |
| `PRIMER_DIST` | `C:\burvy\primer\dist` |
| `PRIMER_CONTENT` | `C:\burvy\primer\content` |
| `PRIMER_ASSETS` | `C:\burvy\primer\assets` |
| `PRIMER_JEV_KEY_FILE` | `C:\burvy\secrets\primer\jev-key.env` |
| `PRIMER_GOOGLE_CLIENT_ID` | `GoogleClientId` from `config.ps1` |
| `PRIMER_DEV_SIGNIN` | removed, always |

## E. Check it works

16. `Invoke-WebRequest http://127.0.0.1:3131/ -UseBasicParsing` should give 200.
17. `Get-Content C:\burvy\data\primer\logs\primer-server.log -Tail 5` should
    show `sign-in: Google` and end with `primer listening on http://127.0.0.1:3131`.
    `sign-in: none` means `GoogleClientId` was empty in the build.
18. `Invoke-WebRequest http://127.0.0.1:3131/api/auth/dev -Method Post -UseBasicParsing`
    must fail with 404: the local test sign-in is off.
19. Open <https://primer.burvy.dev> from another network (e.g. a phone off
    Wi-Fi). Cloudflare Access asks for your email, then the sign-in page shows
    Google's button; sign in and add a child.

If step 19 fails but 16 works, check in order: the router forward (5), the
Origin Rule (2), Full (strict) (3), the certificate files (4), and
`Get-ScheduledTask 'Primer Caddy'`. If Google's button says the origin is not
allowed, check the JavaScript origin in step 7.

## F. Update to a new version

20. On the dev PC: `.\tools\deploy\build.ps1`, then copy `deploy\primer\` to
    `C:\burvy\incoming\primer\` on the server (replace the old copy).
21. On the server, in an admin PowerShell:
    `cd C:\burvy\incoming\primer; Set-ExecutionPolicy -Scope Process Bypass`
22. `.\tools\deploy\update.ps1 -DryRun`
23. `.\tools\deploy\update.ps1`

It backs up the database, stops the server, mirrors the new files into
`C:\burvy\primer\` (keeping the placeholder voices and the Caddyfile), makes
voices for any new lines, and starts the server. Database changes are applied
by the server itself on start (sqlx migrations). The Caddyfile is kept, so a
change to its headers needs `install.ps1` again (it is safe to re-run).

## G. Backups and restore

- Daily copies: `C:\burvy\data\primer\backups\primer-<date>.db`, the newest 14
  kept. Copy that folder off the PC too (with the plgroup backups).
- A backup now: `C:\burvy\primer\tools\deploy\backup.ps1`
- With `sqlite3.exe` on PATH the copy is made live; without it the server is
  stopped for the moment of the copy.

To restore:

24. `Stop-ScheduledTask 'Primer server'; Get-Process primer-server -ErrorAction SilentlyContinue | Where-Object Path -eq 'C:\burvy\primer\primer-server.exe' | Stop-Process`
25. `Copy-Item C:\burvy\data\primer\backups\primer-<date>.db C:\burvy\data\primer\primer.db -Force`
26. `Start-ScheduledTask 'Primer server'`

## H. Safety checklist

- [x] `primer-server` listens on `127.0.0.1` only (checked: a release build with
      these settings shows `127.0.0.1` as its only listening address).
- [x] Port 3130 is open to Cloudflare's addresses only, and Cloudflare is set to
      Full (strict) with an origin certificate.
- [x] The local test sign-in needs `PRIMER_DEV_SIGNIN=1`, which the scripts
      always clear, and the server refuses to start with it and a Google
      client id together (checked: tests, and a server started each way).
- [x] Google sign-in accepts only tokens made for our client id with a
      verified email.
- [x] The Jev key lives outside the app folder and is never copied or printed.
      `JevConfig` shows `[REDACTED]` in debug output, and the server's Jev log
      lines name only the error.
- [x] A child's typed explanation is sent to Jev and never stored or logged.
- [x] Placeholder voices are made on the server, never shipped.
- [ ] Before going public: the Google client id is set and step 19 passes, the
      CSP card in part A is done, the privacy policy (slice 15) is up, then
      remove Cloudflare Access.

## I. Take it down

27. `Stop-ScheduledTask 'Primer server'; Stop-ScheduledTask 'Primer Caddy'`
28. `Unregister-ScheduledTask 'Primer server','Primer Caddy','Primer backup' -Confirm:$false`
29. `Remove-NetFirewallRule -DisplayName 'Primer (Cloudflare only)'`

The data in `C:\burvy\data\primer\` stays until you delete it.
