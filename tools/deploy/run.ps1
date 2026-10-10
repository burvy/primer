# Starts primer-server with the production settings and keeps a log.
# The "Primer server" scheduled task runs this at start-up (install.ps1 sets
# that up); Task Scheduler restarts it if it stops. You can also run it by
# hand to watch it: .\run.ps1
#
# The server listens on 127.0.0.1 only. Nothing outside this PC can reach it
# except through Caddy, and Caddy only through Cloudflare.
#
# The log holds what the server prints: start-up, "not written yet" notes and
# Jev errors (status numbers only). It never holds the Jev key or a child's
# typed explanation; the server does not print either.
param([switch] $DryRun)

$ErrorActionPreference = 'Stop'
. $PSScriptRoot\config.ps1

$logDir = Join-Path $Primer.DataDir 'logs'
$log = Join-Path $logDir 'primer-server.log'
Step "set PRIMER_* and DATABASE_URL, listen on $($Primer.LocalAddr)" { Set-PrimerEnv }
Step 'start a fresh log if the old one passed 10 MB' {
    New-Item -ItemType Directory -Force $logDir | Out-Null
    if ((Test-Path $log) -and (Get-Item $log).Length -gt 10MB) {
        Move-Item $log "$log.old" -Force
    }
}
Step "run primer-server.exe in $($Primer.AppDir), output to $log" {
    Set-Location $Primer.AppDir
    "---- start $(Get-Date -Format s)" | Add-Content $log
    # `*>>` appends every output stream (normal and error) to the log.
    & .\primer-server.exe *>> $log
    exit $LASTEXITCODE
}
