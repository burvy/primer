# Shared settings for the deploy scripts. Every other script in this folder
# dot-sources it (`. $PSScriptRoot\config.ps1`), so a path is written once.
#
# The layout follows the human's other servers: programs and data under
# C:\burvy\, one folder per project, Cloudflare origin certificates under
# C:\burvy\certs\<host>\ (like plgroup-api.burvy.dev).

$Primer = @{
    Host        = 'primer.burvy.dev'
    # The app: primer-server.exe, dist\, content\, assets\, tools\voice\.
    AppDir      = 'C:\burvy\primer'
    # The database, its backups and the logs. Never overwritten by an update.
    DataDir     = 'C:\burvy\data\primer'
    # The Jev key file (only the raw key). Never inside AppDir, never copied.
    KeyFile     = 'C:\burvy\secrets\primer\jev-key.env'
    # Cloudflare Origin Certificate for the host: origin.pem and origin.key.
    CertDir     = 'C:\burvy\certs\primer.burvy.dev'
    CaddyExe    = 'C:\burvy\caddy\caddy.exe'
    # Cloudflare connects to this port (its Origin Rule sends the host here).
    # plgroup-api uses 3120; floret 5002.
    PublicPort  = 3130
    # primer-server listens here, on this machine only. Caddy forwards to it.
    LocalAddr   = '127.0.0.1:3131'
    # Scheduled task names, as shown in Task Scheduler.
    Task        = 'Primer server'
    CaddyTask   = 'Primer Caddy'
    BackupTask  = 'Primer backup'
    # Backups kept (one a day).
    KeepBackups = 14
    # The Google OAuth client id for Google sign-in (docs/DEPLOY.md, step 7).
    # It is not a secret: every visitor's browser sees it. Empty: nobody can
    # sign in yet.
    GoogleClientId = ''
}

# Runs `$Action` unless -DryRun was given to the calling script, and always
# says what it is doing. `$Script:DryRun` is the caller's switch.
function Step {
    param([string] $What, [scriptblock] $Action)
    if ($Script:DryRun) {
        Write-Host "[dry run] $What" -ForegroundColor Yellow
    } else {
        Write-Host "> $What" -ForegroundColor Cyan
        & $Action
    }
}

# The environment primer-server reads (see crates/server/src/main.rs). The
# key file's PATH is set here; its contents are only ever read by the server.
function Set-PrimerEnv {
    $env:PRIMER_ADDR = $Primer.LocalAddr
    # sqlx wants forward slashes: sqlite://C:/burvy/data/primer/primer.db
    $env:DATABASE_URL = 'sqlite://' + (Join-Path $Primer.DataDir 'primer.db').Replace('\', '/')
    $env:PRIMER_DIST = Join-Path $Primer.AppDir 'dist'
    $env:PRIMER_CONTENT = Join-Path $Primer.AppDir 'content'
    $env:PRIMER_ASSETS = Join-Path $Primer.AppDir 'assets'
    $env:PRIMER_JEV_KEY_FILE = $Primer.KeyFile
    $env:PRIMER_GOOGLE_CLIENT_ID = $Primer.GoogleClientId
    # The local test sign-in lets anyone in. It is for a laptop only, so it
    # is cleared here even if this PC has the variable set for some reason
    # (with a client id as well, the server would refuse to start).
    Remove-Item Env:PRIMER_DEV_SIGNIN -ErrorAction SilentlyContinue
}

# Makes any missing placeholder voices on this machine, with its own Windows
# voice (`primer-server voices`). Recordings that exist are never touched.
function Invoke-Voices {
    Set-PrimerEnv
    Push-Location $Primer.AppDir
    try {
        & .\primer-server.exe voices
        if ($LASTEXITCODE -ne 0) { throw 'primer-server voices failed' }
    } finally { Pop-Location }
}

# Stops the server task. Stopping a task ends the PowerShell that run.ps1
# runs in, but not always the primer-server.exe it started, so that is
# stopped too (otherwise its file stays locked and cannot be replaced).
# Only the copy in AppDir is stopped: a primer-server run from anywhere
# else on this PC (a test copy, say) is left alone.
function Stop-Primer {
    Stop-ScheduledTask -TaskName $Primer.Task -ErrorAction SilentlyContinue
    $exe = Join-Path $Primer.AppDir 'primer-server.exe'
    Get-Process primer-server -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -eq $exe } |
        Stop-Process -Force
}

# The Content-Security-Policy: which places the page may load code, styles,
# frames and data from. If someone ever slipped a <script> into the page, the
# browser would refuse to run it, because it is not on this list.
#   script-src  our own files; 'wasm-unsafe-eval' lets the browser compile
#               the web app's WebAssembly; the hashes allow Trunk's small
#               inline start-up script (and nothing else inline); Google's
#               sign-in script.
#   style-src   'unsafe-inline' is needed for the style="" attributes the
#               drawings use; styles cannot run code.
#   frame-src, connect-src   Google's sign-in button and its calls.
#   frame-ancestors 'none'   no other site may show Primer in a frame.
# `$ScriptHashes` are 'sha256-...' sources from Get-InlineScriptHashes.
function New-Csp {
    param([string[]] $ScriptHashes)
    $gsi = 'https://accounts.google.com/gsi/'
    $hashes = ($ScriptHashes | ForEach-Object { "'$_'" }) -join ' '
    @(
        "default-src 'self'"
        "script-src 'self' 'wasm-unsafe-eval' $hashes ${gsi}client"
        "style-src 'self' 'unsafe-inline' ${gsi}style"
        "frame-src $gsi"
        "connect-src 'self' $gsi"
        "img-src 'self' data:"
        "media-src 'self'"
        "object-src 'none'"
        "base-uri 'none'"
        "form-action 'self'"
        "frame-ancestors 'none'"
    ) -join '; '
}

# The hash of each inline <script> in the built page, as CSP wants it:
# sha256 of the script's text, in base64. Trunk writes one inline script and
# its text changes with every build (it names the hashed .wasm file), so the
# hash is worked out from the page actually installed, on every install and
# update. Browsers read "\r\n" as "\n" before hashing, so we do too.
function Get-InlineScriptHashes {
    param([string] $IndexHtml)
    $html = (Get-Content $IndexHtml -Raw) -replace "`r`n", "`n"
    $sha = [Security.Cryptography.SHA256]::Create()
    [regex]::Matches($html, '(?s)<script(?![^>]*\bsrc=)[^>]*>(.*?)</script>') | ForEach-Object {
        $bytes = [Text.Encoding]::UTF8.GetBytes($_.Groups[1].Value)
        'sha256-' + [Convert]::ToBase64String($sha.ComputeHash($bytes))
    }
}

# The Caddyfile. Caddy holds the Cloudflare origin certificate and forwards
# to the server; `auto_https off`: no Let's Encrypt, Cloudflare's
# certificate is used. The headers: https only; files are what they say they
# are; no framing; other sites see only our host name, never a page address
# (Google sign-in needs the host name); and the CSP above.
function New-Caddyfile {
    param([string] $Csp)
    $cert = $Primer.CertDir.Replace('\', '/')
    @"
{
	auto_https off
}

$($Primer.Host):$($Primer.PublicPort) {
	tls $cert/origin.pem $cert/origin.key
	encode zstd gzip
	header {
		Strict-Transport-Security "max-age=31536000"
		X-Content-Type-Options "nosniff"
		X-Frame-Options "DENY"
		Referrer-Policy "strict-origin-when-cross-origin"
		Content-Security-Policy "$Csp"
	}
	reverse_proxy $($Primer.LocalAddr)
}
"@
}

# Writes AppDir\Caddyfile for the web app now in AppDir\dist.
function Write-Caddyfile {
    $hashes = @(Get-InlineScriptHashes (Join-Path $Primer.AppDir 'dist\index.html'))
    Set-Content (Join-Path $Primer.AppDir 'Caddyfile') (New-Caddyfile (New-Csp $hashes))
}
