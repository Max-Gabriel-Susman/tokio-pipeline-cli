# Writes ..\<folder>-submission.zip: everything in this project except the
# target folder and STARTER.md. .git is kept so the commit history shows.
# Run: powershell -ExecutionPolicy Bypass -File scripts\package.ps1
$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

$project = Split-Path -Parent $PSScriptRoot
$name = Split-Path -Leaf $project
$zipPath = Join-Path (Split-Path -Parent $project) "$name-submission.zip"
$targetDir = (Join-Path $project "target") + [System.IO.Path]::DirectorySeparatorChar
$starterNotes = Join-Path $project "STARTER.md"

if (Test-Path $zipPath) { Remove-Item $zipPath }
$archive = [System.IO.Compression.ZipFile]::Open($zipPath, "Create")
try {
    Get-ChildItem -Path $project -Recurse -File -Force |
        Where-Object { -not $_.FullName.StartsWith($targetDir) -and $_.FullName -ne $starterNotes } |
        ForEach-Object {
            $relative = $_.FullName.Substring($project.Length + 1).Replace("\", "/")
            [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($archive, $_.FullName, "$name/$relative") | Out-Null
        }
} finally {
    $archive.Dispose()
}

$sizeKb = [math]::Round((Get-Item $zipPath).Length / 1KB)
Write-Host "Wrote $zipPath ($sizeKb KB)"
if (-not (Test-Path (Join-Path $project "design\architecture.png"))) {
    Write-Host "Note: design\architecture.png is missing. Export the diagram from Excalidraw first."
}
