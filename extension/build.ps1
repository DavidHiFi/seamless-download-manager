[CmdletBinding()]
param([switch]$Icons)

$ErrorActionPreference = 'Stop'
$root = $PSScriptRoot
$src = Join-Path $root 'src'
$dist = Join-Path $root 'dist'

function Get-ExtensionId([string]$KeyBase64) {
    $der = [Convert]::FromBase64String($KeyBase64)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try { $hash = $sha.ComputeHash($der) } finally { $sha.Dispose() }
    $hex = ([BitConverter]::ToString($hash) -replace '-', '').ToLower().Substring(0, 32)
    -join ($hex.ToCharArray() | ForEach-Object { [char]([int][char]'a' + [Convert]::ToInt32([string]$_, 16)) })
}

$chromiumManifest = Get-Content (Join-Path $root 'manifest.chromium.json') -Raw -Encoding utf8 | ConvertFrom-Json
$id = Get-ExtensionId $chromiumManifest.key
$idFile = Join-Path $root 'EXTENSION_ID.txt'
$recorded = if (Test-Path $idFile) { (Get-Content $idFile -Raw).Trim() } else { '' }
if ($recorded -ne $id) { throw "EXTENSION_ID.txt says '$recorded' but the manifest key gives '$id'." }

$needIcons = $Icons -or -not (Test-Path (Join-Path $src 'icons\icon-128.png'))
if ($needIcons) {
    $py = Get-Command python -ErrorAction SilentlyContinue
    if (-not $py) { throw 'Icons are missing and python with Pillow and resvg_py is not available to draw them.' }
    & $py.Source (Join-Path $root 'tools\make-icons.py') (Join-Path $src 'icons')
    if ($LASTEXITCODE -ne 0) { throw 'Icon script failed.' }
}

# Empty the folder rather than delete it, so a shell sitting inside it does not stop the build.
if (Test-Path $dist) { Get-ChildItem $dist -Force | Remove-Item -Recurse -Force }
$targets = @{
    chromium = 'manifest.chromium.json'
    firefox  = 'manifest.firefox.json'
}
foreach ($name in $targets.Keys) {
    $out = Join-Path $dist $name
    New-Item -ItemType Directory -Force $out | Out-Null
    Copy-Item (Join-Path $src '*') $out -Recurse -Force
    Copy-Item (Join-Path $root $targets[$name]) (Join-Path $out 'manifest.json') -Force
}
Copy-Item (Join-Path $root 'background.firefox.html') (Join-Path $dist 'firefox\background.html') -Force

Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
$xpi = Join-Path $dist 'seamless-download-manager.xpi'
$fs = [System.IO.File]::Open($xpi, 'Create')
try {
    $zip = New-Object System.IO.Compression.ZipArchive($fs, [System.IO.Compression.ZipArchiveMode]::Create)
    $base = (Resolve-Path (Join-Path $dist 'firefox')).Path
    Get-ChildItem $base -Recurse -File | ForEach-Object {
        $entryName = $_.FullName.Substring($base.Length + 1).Replace('\', '/')
        [void][System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, $_.FullName, $entryName, [System.IO.Compression.CompressionLevel]::Optimal)
    }
    $zip.Dispose()
} finally {
    $fs.Dispose()
}

Write-Host "Chromium folder : $(Join-Path $dist 'chromium')"
Write-Host "Chromium id     : $id"
Write-Host "Firefox folder  : $(Join-Path $dist 'firefox')"
Write-Host "Firefox package : $xpi"
