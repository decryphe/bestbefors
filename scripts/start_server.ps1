param(
    [string]$Executable = "$PSScriptRoot\bestbefors-cli.exe"
)

if (-not (Test-Path -Path $Executable)) {
    Write-Error "Unable to locate bestbefors-cli.exe next to this script."
    exit 1
}

Write-Host "Starting Bestbefors server..."
& $Executable start
