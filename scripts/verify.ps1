param([switch]$CoreOnly, [switch]$Offline)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$command = Get-Command cargo -ErrorAction SilentlyContinue
if ($command) { $cargo = $command.Source } else { $cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe' }
$python = (Get-Command python -ErrorAction Stop).Source
$cargoOptions = @('--locked')
if ($Offline) { $cargoOptions += '--offline' }
Push-Location $root
try {
    & $python -I scripts/check_repository.py
    if ($LASTEXITCODE -ne 0) { throw 'Publication inventory failed' }
    & $python -I scripts/check_pins.py
    if ($LASTEXITCODE -ne 0) { throw 'Pin/fixture checks failed' }
    & $python -I -B -m unittest discover -s scripts/tests -v
    if ($LASTEXITCODE -ne 0) { throw 'Release-tool tests failed' }
    & $cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { throw 'Rust formatting failed' }
    & $cargo test @cargoOptions -p vivecraft-core
    if ($LASTEXITCODE -ne 0) { throw 'Core tests failed' }
    & $cargo clippy @cargoOptions -p vivecraft-core --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw 'Core Clippy failed' }
    if (-not $CoreOnly) {
        & $cargo clippy @cargoOptions -p vivecraft-pumpkin --target wasm32-wasip2 -- -D warnings
        if ($LASTEXITCODE -ne 0) { throw 'Adapter Clippy failed' }
        & $cargo build @cargoOptions --release -p vivecraft-pumpkin --target wasm32-wasip2
        if ($LASTEXITCODE -ne 0) { throw 'WASM build failed' }
    }
} finally { Pop-Location }
