#Requires -Version 7.0
$ErrorActionPreference = 'Continue'

# Discard stdin (required by the hook protocol; not used).
$null = [Console]::In.ReadToEnd()

if ($env:CLAUDE_PROJECT_DIR) {
    Set-Location $env:CLAUDE_PROJECT_DIR
}

# Auto-fix formatting — no need to block for this.
$null = cargo fmt --all 2>&1

# Run clippy and capture combined stdout+stderr.
$clippyOutput = cargo clippy --all --all-targets --all-features -- -D warnings 2>&1 | Out-String
$clippyExit = $LASTEXITCODE

if ($clippyExit -ne 0) {
    [PSCustomObject]@{
        decision      = 'block'
        reason        = 'Clippy found errors — fix them before finishing.'
        systemMessage = "Fix these clippy errors, then wrap up:`n$($clippyOutput.Trim())"
    } | ConvertTo-Json -Compress | Write-Output
    exit 0
}

Write-Output '{"decision":"approve"}'
exit 0
