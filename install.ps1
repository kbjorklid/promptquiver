$ErrorActionPreference = "Stop"

Write-Host "Compiling Prompt Quiver (release mode)..."
cargo build --release

if ($LASTEXITCODE -ne 0) {
    Write-Error "Compilation failed."
    exit $LASTEXITCODE
}

$exePath = "target\release\promptquiver.exe"
if (-Not (Test-Path $exePath)) {
    Write-Error "Executable not found at $exePath"
    exit 1
}

$targetDir = "$env:USERPROFILE\.local\bin"
if (-Not (Test-Path $targetDir)) {
    Write-Host "Creating directory $targetDir..."
    New-Item -ItemType Directory -Force -Path $targetDir | Out-Null
}

$targetPath = Join-Path $targetDir "quiver.exe"
Write-Host "Moving executable to $targetPath..."
Copy-Item -Path $exePath -Destination $targetPath -Force

Write-Host "Installation complete! You can run the application using 'quiver' (make sure $targetDir is in your PATH)."
