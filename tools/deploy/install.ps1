# First-time set-up on the server PC. Run once, in an ADMIN PowerShell,
# from the copied deploy folder:
#
#   .\tools\deploy\install.ps1 -DryRun    see every step first
#   .\tools\deploy\install.ps1
#
# It: makes the folders, copies the app to C:\burvy\primer, writes the
# Caddyfile, opens the public port to Cloudflare's addresses only, registers
# three scheduled tasks (server, Caddy, daily backup), makes placeholder
# voices, and starts everything. Safe to run again: each step replaces what
# it made last time.
#
# It does NOT touch DNS, Cloudflare or the router (docs/DEPLOY.md lists
# those), and never creates or reads the Jev key or the certificate.
param([switch] $DryRun)

$ErrorActionPreference = 'Stop'
. $PSScriptRoot\config.ps1
$source = Resolve-Path (Join-Path $PSScriptRoot '..\..')

$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).
    IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin -and -not $DryRun) { throw 'run this in an admin PowerShell' }

Step 'make the folders under C:\burvy' {
    foreach ($dir in @($Primer.AppDir, $Primer.DataDir, (Join-Path $Primer.DataDir 'backups'),
            (Join-Path $Primer.DataDir 'logs'), $Primer.CertDir, (Split-Path $Primer.KeyFile))) {
        New-Item -ItemType Directory -Force $dir | Out-Null
    }
}
Step "copy $source to $($Primer.AppDir)" {
    robocopy $source $Primer.AppDir /E /NFL /NDL /NJH /NJS | Out-Null
    if ($LASTEXITCODE -ge 8) { throw 'robocopy failed' }
    $global:LASTEXITCODE = 0
}
Step 'check the certificate and key file are in place (they are copied by hand)' {
    foreach ($file in @((Join-Path $Primer.CertDir 'origin.pem'), (Join-Path $Primer.CertDir 'origin.key'))) {
        if (-not (Test-Path $file)) { throw "missing $file (see docs/DEPLOY.md, part B)" }
    }
    if (-not (Test-Path $Primer.KeyFile)) {
        Write-Host "  no $($Primer.KeyFile): the server will use the fake judge" -ForegroundColor Yellow
    }
}

Step "write $($Primer.AppDir)\Caddyfile (with the page's script hashes)" { Write-Caddyfile }
if ($DryRun) { Write-Host (New-Caddyfile (New-Csp @('sha256-<hash from dist\index.html>'))) -ForegroundColor DarkGray }

Step "allow port $($Primer.PublicPort) in, from Cloudflare's addresses only" {
    $ranges = @('https://www.cloudflare.com/ips-v4', 'https://www.cloudflare.com/ips-v6') |
        ForEach-Object { (Invoke-RestMethod $_) -split "`n" } |
        Where-Object { $_.Trim() }
    Remove-NetFirewallRule -DisplayName 'Primer (Cloudflare only)' -ErrorAction SilentlyContinue
    New-NetFirewallRule -DisplayName 'Primer (Cloudflare only)' -Direction Inbound -Protocol TCP `
        -LocalPort $Primer.PublicPort -RemoteAddress $ranges -Action Allow | Out-Null
}

# Each task runs as SYSTEM at start-up (no sign-in needed) and is restarted
# by Task Scheduler if it stops.
function Register-PrimerTask {
    param([string] $Name, [string] $Exe, [string] $Arguments, $Trigger)
    $action = New-ScheduledTaskAction -Execute $Exe -Argument $Arguments -WorkingDirectory $Primer.AppDir
    $settings = New-ScheduledTaskSettingsSet -RestartCount 999 -RestartInterval (New-TimeSpan -Minutes 1) `
        -ExecutionTimeLimit ([TimeSpan]::Zero) -StartWhenAvailable
    Register-ScheduledTask -TaskName $Name -Action $action -Trigger $Trigger -Settings $settings `
        -User 'SYSTEM' -RunLevel Highest -Force | Out-Null
}
$ps = 'powershell.exe'
$scripts = Join-Path $Primer.AppDir 'tools\deploy'
Step "register task '$($Primer.Task)' (run.ps1 at start-up)" {
    Register-PrimerTask $Primer.Task $ps "-NoProfile -ExecutionPolicy Bypass -File `"$scripts\run.ps1`"" `
        (New-ScheduledTaskTrigger -AtStartup)
}
Step "register task '$($Primer.CaddyTask)' (Caddy at start-up)" {
    if (-not (Test-Path $Primer.CaddyExe)) { throw "missing $($Primer.CaddyExe) (see docs/DEPLOY.md, part B)" }
    Register-PrimerTask $Primer.CaddyTask $Primer.CaddyExe `
        "run --config `"$($Primer.AppDir)\Caddyfile`" --adapter caddyfile" (New-ScheduledTaskTrigger -AtStartup)
}
Step "register task '$($Primer.BackupTask)' (backup.ps1 daily at 03:30)" {
    Register-PrimerTask $Primer.BackupTask $ps "-NoProfile -ExecutionPolicy Bypass -File `"$scripts\backup.ps1`"" `
        (New-ScheduledTaskTrigger -Daily -At 3:30am)
}
Step 'make placeholder voices with this PC''s Windows voice' { Invoke-Voices }
Step 'start the server and Caddy' {
    Start-ScheduledTask -TaskName $Primer.Task
    Start-ScheduledTask -TaskName $Primer.CaddyTask
}
Write-Host "done. Check: Invoke-WebRequest http://$($Primer.LocalAddr)/ -UseBasicParsing"
