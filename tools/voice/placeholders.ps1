# Makes placeholder recordings: Ada's lines read by Windows' own offline voice.
#
# Usage (normally run for you by `cargo run -p primer-server -- voices`):
#   powershell -File tools/voice/placeholders.ps1 -List lines.tsv -Out assets/voice/placeholder
#
# `-List` is a text file with one line per recording: the recording's name, a
# tab, then the words to read. A WAV file is written to `-Out` for each one.
# A file that already exists is NEVER overwritten, so a placeholder you like
# (or one the script made earlier) stays as it is. To remake one, delete it.
#
# Needs Windows (System.Speech), no internet, no cost. `-Voice` defaults to
# Zira (US English, female). If that voice is not installed it falls back to
# the first female English voice, then to the default voice.

param(
    [Parameter(Mandatory = $true)][string]$List,
    [Parameter(Mandatory = $true)][string]$Out,
    [string]$Voice = "Microsoft Zira Desktop"
)

Add-Type -AssemblyName System.Speech
$speaker = New-Object System.Speech.Synthesis.SpeechSynthesizer

# Choose the voice: the asked-for one, else any female English voice.
$installed = $speaker.GetInstalledVoices() | ForEach-Object { $_.VoiceInfo }
$chosen = $installed | Where-Object { $_.Name -eq $Voice } | Select-Object -First 1
if (-not $chosen) {
    $chosen = $installed |
        Where-Object { $_.Gender -eq "Female" -and $_.Culture.Name -like "en-*" } |
        Select-Object -First 1
}
if ($chosen) { $speaker.SelectVoice($chosen.Name) }
Write-Host "voice: $($speaker.Voice.Name)"

# Small files: 11 kHz, 16 bit, one channel is plenty for a placeholder.
$format = New-Object System.Speech.AudioFormat.SpeechAudioFormatInfo(
    11025,
    [System.Speech.AudioFormat.AudioBitsPerSample]::Sixteen,
    [System.Speech.AudioFormat.AudioChannel]::Mono)

New-Item -ItemType Directory -Force -Path $Out | Out-Null
$made = 0
$kept = 0
foreach ($row in Get-Content -Path $List -Encoding UTF8) {
    if ($row.Trim() -eq "") { continue }
    $name, $words = $row -split "`t", 2
    $file = Join-Path (Resolve-Path $Out) "$name.wav"
    if (Test-Path $file) { $kept++; continue }
    $speaker.SetOutputToWaveFile($file, $format)
    $speaker.Speak($words)
    $speaker.SetOutputToNull()
    $made++
}
$speaker.Dispose()
Write-Host "placeholders: $made made, $kept already there"
