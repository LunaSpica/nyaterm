param(
    [ValidateSet("x86_64", "aarch64")]
    [string]$Architecture = "x86_64",
    [string]$Version = "0.0.0",
    [string]$OutputDirectory = "dist",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$target = if ($Architecture -eq "aarch64") { "aarch64-pc-windows-msvc" } else { "x86_64-pc-windows-msvc" }
$workspace = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$stage = Join-Path $workspace "target/package/windows/$target/NyaTerm"
$output = Join-Path $workspace $OutputDirectory

if (-not $SkipBuild) {
    cargo build --locked --release --target $target -p nyaterm-app --bin nyaterm
}
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item $stage -ItemType Directory -Force | Out-Null
Copy-Item (Join-Path $workspace "target/$target/release/nyaterm.exe") (Join-Path $stage "NyaTerm.exe")
Copy-Item (Join-Path $workspace "LICENSE*") $stage -ErrorAction SilentlyContinue
New-Item $output -ItemType Directory -Force | Out-Null

$archive = Join-Path $output "NyaTerm-$Version-windows-$Architecture.zip"
if (Test-Path $archive) { Remove-Item $archive -Force }
Compress-Archive -Path "$stage/*" -DestinationPath $archive -CompressionLevel Optimal

$hash = (Get-FileHash $archive -Algorithm SHA256).Hash.ToLowerInvariant()
Set-Content -Path "$archive.sha256" -Value "$hash  $(Split-Path $archive -Leaf)" -NoNewline
Write-Output $archive
