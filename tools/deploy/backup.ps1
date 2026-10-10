# Copies the database to DataDir\backups\primer-<date>.db and keeps the
# newest 14. The "Primer backup" task runs it daily; run it by hand before
# anything risky (update.ps1 does).
#
# SQLite must not be copied mid-write. With sqlite3.exe on PATH, its
# `.backup` command makes a safe copy while the server runs. Without it, the
# server is stopped for the second the copy takes, then started again.
param([switch] $DryRun)

$ErrorActionPreference = 'Stop'
. $PSScriptRoot\config.ps1

$db = Join-Path $Primer.DataDir 'primer.db'
$dir = Join-Path $Primer.DataDir 'backups'
$copy = Join-Path $dir ("primer-{0}.db" -f (Get-Date -Format 'yyyy-MM-dd-HHmm'))

if (-not (Test-Path $db) -and -not $DryRun) {
    Write-Host "no database yet at ${db}: nothing to back up"
    exit 0
}
Step "back up $db to $copy" {
    New-Item -ItemType Directory -Force $dir | Out-Null
    if (Get-Command sqlite3 -ErrorAction SilentlyContinue) {
        sqlite3 $db ".backup '$copy'"
        if ($LASTEXITCODE -ne 0) { throw 'sqlite3 .backup failed' }
    } else {
        $running = (Get-ScheduledTask -TaskName $Primer.Task -ErrorAction SilentlyContinue).State -eq 'Running'
        if ($running) { Stop-Primer }
        try { Copy-Item $db $copy } finally {
            if ($running) { Start-ScheduledTask -TaskName $Primer.Task }
        }
    }
}
Step "keep the newest $($Primer.KeepBackups) backups" {
    Get-ChildItem $dir -Filter 'primer-*.db' |
        Sort-Object Name -Descending |
        Select-Object -Skip $Primer.KeepBackups |
        Remove-Item
}
