# Builds the shipped NRO and the Eden diag NRO into dist/release and dist/diag.
# Run under the memory/time guard; one cargo call per build.
param([ValidateSet('release', 'diag')] [string]$Kind = 'release')
$ErrorActionPreference = 'Stop'
$env:PATH = '/home/intpa/.cargo/bin:' + $env:PATH
$env:CARGO_TERM_COLOR = 'never'
$env:CARGO_TERM_HYPERLINKS = 'false'
$repo = Split-Path -Parent $PSScriptRoot
Set-Location $repo
if ($Kind -eq 'diag') { cargo skyline build --release --features diag } else { cargo skyline build --release }
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$out = Join-Path $repo "dist/$Kind"
New-Item -ItemType Directory -Force $out | Out-Null
$nro = Join-Path $repo 'target/aarch64-skyline-switch/release/libssbu_cpu_pilot.nro'
Copy-Item $nro $out -Force
$hash = (Get-FileHash (Join-Path $out 'libssbu_cpu_pilot.nro')).Hash.ToLower()
"$Kind libssbu_cpu_pilot.nro sha256 $hash"
