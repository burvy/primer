# Puts a new build in place on the server PC. Run in an ADMIN PowerShell,
# from the NEW deploy folder you just copied over:
#
#   .\tools\deploy\update.ps1 -DryRun
#   .\tools\deploy\update.ps1
#
# Order: back up the database, stop the server, swap the files, make any
# new placeholder voices, start the server. The database, logs, key and
# certificate live outside C:\burvy\primer and are never touched. A schema
# change is applied by the server itself at start (sqlx migrations).
param([switch] $DryRun)

$ErrorActionPreference = 'Stop'
. $PSScriptRoot\config.ps1
$source = Resolve-Path (Join-Path $PSScriptRoot '..\..')

Step 'back up the database first' { & (Join-Path $PSScriptRoot 'backup.ps1') }
Step "stop task '$($Primer.Task)'" { Stop-Primer }
Step "mirror $source into $($Primer.AppDir), keeping placeholder voices" {
    # /MIR makes the folder match the new build exactly (old files go).
    # /XD and /XF leave out what the server made itself, so it stays.
    robocopy $source $Primer.AppDir /MIR /XD placeholder /XF Caddyfile RECORDINGS.md /NFL /NDL /NJH /NJS | Out-Null
    if ($LASTEXITCODE -ge 8) { throw 'robocopy failed' }
    $global:LASTEXITCODE = 0
}
Step 'rewrite the Caddyfile: the new web app has new script hashes' { Write-Caddyfile }
Step 'make placeholder voices for any new lines' { Invoke-Voices }
Step "start task '$($Primer.Task)'" { Start-ScheduledTask -TaskName $Primer.Task }
Step "restart task '$($Primer.CaddyTask)' so it reads the new Caddyfile" {
    Stop-ScheduledTask -TaskName $Primer.CaddyTask -ErrorAction SilentlyContinue
    Get-Process caddy -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -eq $Primer.CaddyExe } |
        Stop-Process -Force
    Start-ScheduledTask -TaskName $Primer.CaddyTask
}
Write-Host 'done. The log is in C:\burvy\data\primer\logs\primer-server.log'
