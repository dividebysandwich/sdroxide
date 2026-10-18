# **Does the browser client get its sound back after a long spell hidden?**
#
#   .\tools\web-audio-recovery\run.ps1
#
# The same thing as `run.sh`, for a Windows checkout: the page is driven in a
# headless browser against the *real* bridge scripts, served from a scratch
# directory so nothing is written inside the tree. See `harness.js` for what it
# does and what it asserts; this script only builds the scratch copy, serves it,
# and reports what the page said.
#
# Needs python and a Chromium-family browser (Edge or Chrome).
$ErrorActionPreference = "Stop"

$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$root = (Resolve-Path (Join-Path $here "..\..")).Path
$build = Join-Path $env:TEMP "sdroxide-web-audio-recovery"
$raw = Join-Path $env:TEMP "sdroxide-web-audio-recovery.txt"
$port = if ($env:PORT) { $env:PORT } else { "8731" }

$browser = @(
    "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    "C:\Program Files\Microsoft\Edge\Application\msedge.exe",
    "C:\Program Files\Google\Chrome\Application\chrome.exe",
    "C:\Program Files (x86)\Google\Chrome\Application\chrome.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $browser) {
    Write-Error "no Chromium-family browser found (Edge or Chrome)"
    exit 2
}

Remove-Item $build -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item $raw -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $build | Out-Null
Copy-Item (Join-Path $root "crates\sdroxide-web\assets\audio_bridge.js") $build
Copy-Item (Join-Path $root "crates\sdroxide-web\assets\pcm_worklet.js") $build
Copy-Item (Join-Path $here "page.html") $build
Copy-Item (Join-Path $here "harness.js") $build
Copy-Item (Join-Path $here "serve.py") $build

$server = Start-Process python -ArgumentList "serve.py", $port, $raw `
    -WorkingDirectory $build -PassThru -WindowStyle Hidden
Start-Sleep 2

# A profile of its own per run: a browser left over from an earlier run holds
# the same directory open, and a second launch against it is handed to the
# process that is already there instead of loading the page.
$profile = Join-Path $build ("profile-" + [guid]::NewGuid().ToString("N"))
$started = Get-Date
$client = Start-Process $browser -ArgumentList @(
    "--headless=new", "--disable-gpu", "--no-first-run", "--no-default-browser-check",
    # Edge otherwise opens its own sign-in/sync panel instead of the page.
    "--disable-sync", "--disable-extensions",
    "--autoplay-policy=no-user-gesture-required",
    "--user-data-dir=$profile",
    "http://127.0.0.1:$port/page.html"
) -PassThru

Start-Sleep 22
Stop-Process -Id $client.Id -Force -ErrorAction SilentlyContinue
# Whatever outlived the browser process: ours, and only ours — headless
# processes with no window, started after this run began.
Get-Process msedge, chrome -ErrorAction SilentlyContinue |
    Where-Object { $_.MainWindowHandle -eq 0 -and $_.StartTime -ge $started } |
    Stop-Process -Force -ErrorAction SilentlyContinue
Stop-Process -Id $server.Id -Force -ErrorAction SilentlyContinue

Write-Host "--- the page said ---"
$lines = @(Get-Content $raw -ErrorAction SilentlyContinue)
if ($lines.Count -eq 0) { Write-Host "(nothing)" } else { $lines | ForEach-Object { Write-Host $_ } }
if ($lines -match "^line=PASS ") {
    Write-Host "--- PASS ---"
    exit 0
}
Write-Host "--- FAIL ---" -ForegroundColor Red
exit 1
