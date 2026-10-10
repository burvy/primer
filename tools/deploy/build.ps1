# Builds a release of Primer into deploy\primer\, ready to copy to the server
# PC as it is (like burvy-dev's `.\build.ps1 -Deploy`). Run on the dev PC,
# from the repo root:
#
#   .\tools\deploy\build.ps1
#   .\tools\deploy\build.ps1 -DryRun    only say what would happen
#
# What goes in deploy\primer\:
#   primer-server.exe   release build
#   dist\               the web app (trunk build --release)
#   content\            the book
#   assets\             drawings and real recordings (placeholder voices are
#                       NOT copied: the server makes its own, see run.ps1)
#   tools\voice\        the placeholder-voice script the server runs
#   tools\deploy\       these scripts, to run on the server
# Never copied: jev-key.env, primer.db, certificates.
#
# There is one kind of build. How parents sign in is set when the server
# starts, not when it is built: run.ps1 gives it the Google client id and
# never turns on the local test sign-in (see Set-PrimerEnv in config.ps1).
param(
    [switch] $DryRun,
    [string] $Out = 'deploy\primer'
)

$ErrorActionPreference = 'Stop'
. $PSScriptRoot\config.ps1
$repo = Resolve-Path (Join-Path $PSScriptRoot '..\..')
Set-Location $repo

Step 'cargo build --release -p primer-server' {
    cargo build --release -p primer-server
    if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }
}
Step 'trunk build --release (crates\web)' {
    Push-Location crates\web
    try {
        trunk build --release
        if ($LASTEXITCODE -ne 0) { throw 'trunk build failed' }
    } finally { Pop-Location }
}
Step "empty $Out" {
    if (Test-Path $Out) { Remove-Item $Out -Recurse -Force }
    New-Item -ItemType Directory -Force $Out | Out-Null
}
Step 'copy the server, web app, book and tools' {
    Copy-Item target\release\primer-server.exe $Out
    Copy-Item crates\web\dist (Join-Path $Out 'dist') -Recurse
    Copy-Item content (Join-Path $Out 'content') -Recurse
    New-Item -ItemType Directory -Force (Join-Path $Out 'tools') | Out-Null
    Copy-Item tools\voice (Join-Path $Out 'tools\voice') -Recurse
    Copy-Item tools\deploy (Join-Path $Out 'tools\deploy') -Recurse
}
Step 'copy assets, leaving out placeholder voices' {
    # robocopy copies a tree and can leave folders out (/XD). Its exit codes
    # 0-7 all mean success; 8 and up are failures.
    robocopy assets (Join-Path $Out 'assets') /E /XD placeholder /NFL /NDL /NJH /NJS | Out-Null
    if ($LASTEXITCODE -ge 8) { throw 'robocopy failed' }
    $global:LASTEXITCODE = 0
}
if ($Primer.GoogleClientId -eq '') {
    Write-Host 'note: GoogleClientId in config.ps1 is empty, so nobody can sign in on the server yet' -ForegroundColor Yellow
}
Write-Host "done: $Out. Copy it to the server PC, then run install.ps1 or update.ps1 there."
