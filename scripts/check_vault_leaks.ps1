param([string]$ProjectRoot = (Split-Path -Parent $PSScriptRoot))
$ErrorActionPreference = 'Stop'
$ProjectRoot = (Resolve-Path -LiteralPath $ProjectRoot).Path
& python -B (Join-Path $PSScriptRoot 'check_vault_leaks.py') --root $ProjectRoot
if ($LASTEXITCODE -ne 0) { throw 'Hardware vault validation failed.' }
