# Runs the Primer prototype on this PC. The one command to try it, from the
# repo root:
#
#   .\tools\run-local.ps1          on this PC only: http://127.0.0.1:3000
#   .\tools\run-local.ps1 -Lan     also from a phone or tablet on the same Wi-Fi
#   .\tools\run-local.ps1 -DryRun  only say what would happen
#
# What it does, in order:
#   1. stops a primer-server left running from an earlier run-local (only one
#      listening on this port and not the deployed copy in C:\burvy\primer);
#   2. builds the web app (trunk build in crates\web);
#   3. makes any missing placeholder voices (real recordings are never touched);
#   4. starts the server with the local test sign-in on (PRIMER_DEV_SIGNIN=1)
#      and prints the addresses to open. Ctrl+C stops it.
#
# The local test sign-in lets anyone who can open the page in as the same
# parent. That is fine on a laptop. With -Lan, anyone on your Wi-Fi can open
# it, so use -Lan only on a network you trust. This script is never used on
# the server (see docs/DEPLOY.md).
#
# The database is primer.db in the repo root, and the Jev key is read by the
# server from jev-key.env if it is there (otherwise the fake judge is used).
# This script never reads the key.
param(
    [switch] $Lan,
    [int] $Port = 3000,
    [switch] $DryRun
)

$ErrorActionPreference = 'Stop'
$repo = Resolve-Path (Join-Path $PSScriptRoot '..')
Set-Location $repo

# Runs `$Action` unless -DryRun, and always says what it is doing.
function Step {
    param([string] $What, [scriptblock] $Action)
    if ($DryRun) {
        Write-Host "[dry run] $What" -ForegroundColor Yellow
    } else {
        Write-Host "> $What" -ForegroundColor Cyan
        & $Action
    }
}

$hostIp = if ($Lan) { '0.0.0.0' } else { '127.0.0.1' }

Step "stop an earlier local primer-server on port $Port" {
    $listening = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue |
        Select-Object -ExpandProperty OwningProcess -Unique
    foreach ($id in $listening) {
        $process = Get-Process -Id $id -ErrorAction SilentlyContinue
        if (-not $process) { continue }
        if ($process.ProcessName -ne 'primer-server') {
            throw "port $Port is used by $($process.ProcessName) (process $id); choose another with -Port"
        }
        if ($process.Path -like 'C:\burvy\*') {
            throw "port $Port is used by the deployed Primer ($($process.Path)); choose another with -Port"
        }
        Stop-Process -Id $id -Force
        Write-Host "  stopped primer-server (process $id)"
    }
}
Step 'build the web app (trunk build in crates\web)' {
    Push-Location crates\web
    try {
        trunk build
        if ($LASTEXITCODE -ne 0) { throw 'trunk build failed' }
    } finally { Pop-Location }
}

# The server's settings. Google sign-in is cleared: with it set as well, the
# server would refuse to start (see SignIn::decide in crates/server).
$env:PRIMER_DEV_SIGNIN = '1'
$env:PRIMER_ADDR = "${hostIp}:$Port"
Remove-Item Env:PRIMER_GOOGLE_CLIENT_ID -ErrorAction SilentlyContinue

Step 'make any missing placeholder voices' {
    cargo run -q -p primer-server -- voices
    if ($LASTEXITCODE -ne 0) { throw 'making the placeholder voices failed' }
}

Write-Host ''
Write-Host "Open: http://127.0.0.1:$Port" -ForegroundColor Green
if ($Lan) {
    # This PC's addresses on the home network: IPv4, not the loopback and not
    # the 169.254 address Windows gives itself when there is no network.
    Get-NetIPAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue |
        Where-Object { $_.IPAddress -ne '127.0.0.1' -and $_.IPAddress -notlike '169.254.*' } |
        ForEach-Object { Write-Host "  on the Wi-Fi: http://$($_.IPAddress):$Port  ($($_.InterfaceAlias))" -ForegroundColor Green }
    Write-Host '  (if a phone cannot connect, allow primer-server through Windows Firewall for Private networks)'
}
Write-Host 'Sign in with "Sign in (local test)". Ctrl+C stops the server.'
Write-Host ''
Step "run primer-server on ${hostIp}:$Port" {
    cargo run -q -p primer-server
}
