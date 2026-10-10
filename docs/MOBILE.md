# Primer on phones and tablets

**Recommendation:** ship a **PWA** first (an installable web app: same code, no
store), then wrap that same web build with **Tauri 2** for the App Store and
Google Play once it is proven. Tauri keeps the whole stack in Rust and works
with Trunk, which the human already uses. Capacitor would also work, but it
adds a Node/npm toolchain for no gain here.

Nothing below is built yet. The PWA lives in `crates/web` (Kevin's area), so
this file is the spec for that slice.

## Step 1: PWA (installable, fast, works on a bad connection)

What it needs:

1. **`crates/web/manifest.webmanifest`**, linked from `index.html` with
   `<link rel="manifest">`:
   - `name` "Primer", `short_name` "Primer", `start_url` "/", `scope` "/";
   - `display` "standalone", `orientation` "any";
   - `background_color` and `theme_color` from the Gentle theme;
   - `icons`: 192 px and 512 px PNGs, plus a 512 px `"purpose": "maskable"`.
2. **Icons** in `assets/icons/`, drawn by the human: `icon-192.png`,
   `icon-512.png`, `maskable-512.png`, and `apple-touch-icon.png` at 180 px.
   iOS needs `<link rel="apple-touch-icon">` and
   `<meta name="apple-mobile-web-app-capable" content="yes">`.
3. **Service worker `sw.js`**, registered from `index.html`:
   - **Precache the app shell** on install: `index.html` and Trunk's hashed
     `.wasm`, `.js` and `.css`. Trunk changes the hashes on every build, so a
     Trunk `post_build` hook writes the list into `sw.js` (a version number
     plus the file list). A new version replaces the old cache.
   - **`/assets/*`** (drawings, voices): cache-first, filled as pages use them,
     so a chapter already opened plays its sounds offline.
   - **`/api/*`**: network only, never cached, because answers must reach the
     server. When offline, the play page shows Ada's "We need the internet to
     carry on" line instead of an error.
4. **HTTPS**: already there (Cloudflare and Caddy, see `docs/DEPLOY.md`).
   Service workers need it.
5. **Checks:** Lighthouse "Installable" passes; Add to Home Screen works on
   Android Chrome and iOS Safari; the app opens from the home screen while
   offline and shows the offline line.

Later, true offline play: the core is pure and already builds for wasm, so
the browser can run `policy::next` itself, keep events in IndexedDB and send
them when back online (the server replays the same fold). This is a separate
slice; it needs the content shipped to the browser and a sync endpoint.

## Step 2: store apps with Tauri 2 (later)

What it needs:

1. `src-tauri/` with `tauri.conf.json`. Point `build.frontendDist` at the
   Trunk `dist/`, and `beforeBuildCommand` at `trunk build --release`.
2. **API base URL:** inside the app, the page is served from the app itself,
   not from primer.burvy.dev, so `crates/web/src/api.rs` needs a base URL
   (empty on the web, `https://primer.burvy.dev` in the app), and the server
   needs CORS for the app's origin.
3. **Sign-in token instead of a cookie** for the app build: cookies are
   unreliable on an app's own origin. Store the session token in Tauri's
   secure store and send it as `Authorization: Bearer`.
4. **Tooling:**
   - Android: Android Studio, an SDK and NDK, and
     `rustup target add aarch64-linux-android`.
   - iOS: a Mac with Xcode, and an Apple Developer account ($99 a year).
5. **Store rules for children's apps:**
   - Apple Kids category, Google Play Families policy;
   - no third-party analytics or ads;
   - a grown-up gate before links, settings or purchases (we have one);
   - a privacy policy URL (slice 15).
   - Apple also rejects apps that are "just a website", so the app should
     bundle the web build (as above), not only load the URL.
