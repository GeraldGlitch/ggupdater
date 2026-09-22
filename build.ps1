$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

cargo build --release
New-Item -ItemType Directory -Force -Path dist | Out-Null
Copy-Item target/release/ggupdater.exe dist/ggupdater.exe -Force
Write-Host "OK -> dist/ggupdater.exe"
