$ErrorActionPreference = "Stop"

Write-Host "Aplicando Nexus Arena no diretório atual..." -ForegroundColor Cyan

$patchRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Get-Location

Get-ChildItem -Path $patchRoot -Recurse -File |
    Where-Object {
        $_.Name -ne "apply_patch.ps1" -and
        $_.Name -ne "README_NEXUS_ARENA.md"
    } |
    ForEach-Object {
        $relative = $_.FullName.Substring($patchRoot.Length + 1)
        $target = Join-Path $repoRoot $relative
        $targetDirectory = Split-Path -Parent $target

        New-Item -ItemType Directory -Force -Path $targetDirectory |
            Out-Null

        Copy-Item -Force $_.FullName $target
        Write-Host "  $relative"
    }

Copy-Item -Force `
    (Join-Path $patchRoot "README_NEXUS_ARENA.md") `
    (Join-Path $repoRoot "README_NEXUS_ARENA.md")

Write-Host ""
Write-Host "Arquivos aplicados." -ForegroundColor Green
Write-Host "Execute:"
Write-Host "  cargo fmt --all"
Write-Host "  cargo check --workspace"
