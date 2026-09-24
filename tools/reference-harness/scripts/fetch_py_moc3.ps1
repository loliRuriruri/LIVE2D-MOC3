# Fetch the pinned py-moc3 reference checkout (dev-only, MIT).
# Usage: powershell -File scripts/fetch_py_moc3.ps1
# The checkout lives under target/reference (gitignored), never in the repo tree.

$ErrorActionPreference = "Stop"
$pinned = "2fb112e11a"
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$destination = Join-Path $root "target\reference\py-moc3"

if (Test-Path -LiteralPath $destination) {
    Write-Output "py-moc3 checkout already present: $destination"
} else {
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $destination) | Out-Null
    git clone https://github.com/Ludentes/py-moc3.git $destination
}

Push-Location $destination
try {
    git fetch --all --quiet
    git checkout --quiet $pinned
    $head = git rev-parse --short HEAD
    Write-Output "py-moc3 checked out at $head (pinned $pinned)"
} finally {
    Pop-Location
}

Write-Output ""
Write-Output "Run a differential comparison with:"
Write-Output "  `$env:PY_MOC3_DIR = '$destination'"
Write-Output "  cargo run -p reference-harness -- compare fixtures\synthetic\fixture-010-v53.moc3 --provider ours --provider py-moc3"
