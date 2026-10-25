param(
    [ValidateSet('x86_64-pc-windows-msvc')]
    [string] $Target = 'x86_64-pc-windows-msvc'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ($env:OS -ne 'Windows_NT' -or $env:CI -ne 'true') {
    throw 'Run this verifier only on an isolated Windows CI runner, not a personal desktop.'
}

$root = Split-Path -Parent $PSScriptRoot
$package = Get-Content -LiteralPath (Join-Path $root 'package.json') -Raw | ConvertFrom-Json
$version = $package.version
$release = Join-Path $root "src-tauri/target/$Target/release"
$binary = Join-Path $release 'bytepet.exe'
$out = Join-Path $root 'artifacts/windows-x64'
New-Item -ItemType Directory -Path $out -Force | Out-Null

# Read the real application PE header, not the NSIS bootstrapper (which may be x86).
$stream = [IO.File]::OpenRead($binary)
$reader = [IO.BinaryReader]::new($stream)
try {
    if ($reader.ReadUInt16() -ne 0x5A4D) { throw 'Missing MZ executable header.' }
    $stream.Position = 0x3C
    $peOffset = $reader.ReadInt32()
    if ($peOffset -lt 64 -or $peOffset -gt ($stream.Length - 6)) { throw 'Invalid PE offset.' }
    $stream.Position = $peOffset
    if ($reader.ReadUInt32() -ne 0x00004550 -or $reader.ReadUInt16() -ne 0x8664) {
        throw 'Expected an x64 PE application.'
    }
} finally {
    $reader.Dispose()
}
$fileVersion = [Diagnostics.FileVersionInfo]::GetVersionInfo($binary)
if ($fileVersion.ProductVersion -ne $version) {
    throw "Executable version differs: $($fileVersion.ProductVersion) != $version"
}

$nsis = @(Get-ChildItem -LiteralPath (Join-Path $release 'bundle/nsis') -Filter '*.exe' -File)
$msi = @(Get-ChildItem -LiteralPath (Join-Path $release 'bundle/msi') -Filter '*.msi' -File)
if ($nsis.Count -ne 1 -or $msi.Count -ne 1) { throw 'Expected exactly one EXE and one MSI installer.' }
foreach ($file in @($nsis[0], $msi[0])) {
    if ($file.Length -lt 1024 -or -not $file.Name.Contains("_${version}_")) {
        throw "Invalid or incorrectly versioned installer: $($file.Name)"
    }
}

# CI smoke test only: never install an MSI/EXE, never touch a developer's data.
$process = $null
try {
    $process = Start-Process -FilePath $binary -WorkingDirectory $release -WindowStyle Hidden -PassThru `
        -RedirectStandardOutput (Join-Path $out 'startup.stdout.log') `
        -RedirectStandardError (Join-Path $out 'startup.stderr.log')
    Start-Sleep -Seconds 8
    $process.Refresh()
    if ($process.HasExited) { throw "Packaged application exited early: $($process.ExitCode)" }
} finally {
    if ($null -ne $process) {
        $process.Refresh()
        if (-not $process.HasExited) {
            Stop-Process -Id $process.Id -Force
            $process.WaitForExit()
        }
        $process.Dispose()
    }
}

$outputs = @()
foreach ($file in @($nsis[0], $msi[0])) {
    $name = if ($file.Extension -eq '.exe') { "BytePet_${version}_x64-setup.exe" } else { "BytePet_${version}_x64.msi" }
    $destination = Join-Path $out $name
    Copy-Item -LiteralPath $file.FullName -Destination $destination
    $outputs += $destination
}
$lines = foreach ($path in $outputs) {
    $hash = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    "$hash  $([IO.Path]::GetFileName($path))"
}
[IO.File]::WriteAllLines((Join-Path $out 'SHA256SUMS.txt'), [string[]] $lines, [Text.UTF8Encoding]::new($false))
if ($env:GITHUB_STEP_SUMMARY) {
    @"
### BytePet $version - Windows x64
- EXE/MSI packages, application PE architecture, version and native startup checks passed.
- Download the installers and SHA256SUMS.txt from this run's Artifacts.
- These packages are unsigned. SmartScreen may warn; installation still needs manual acceptance testing.
"@ | Out-File -LiteralPath $env:GITHUB_STEP_SUMMARY -Append -Encoding utf8
}
