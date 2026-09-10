param(
    [Parameter(Mandatory=$true)][string]$Version
)
$ErrorActionPreference = 'Stop'
# Expected-negative native verifier exits are checked explicitly below.
$PSNativeCommandUseErrorActionPreference = $false
if ($Version -notmatch '^0\.\d+\.\d+$') { throw 'Invalid application version' }
if ([string]::IsNullOrWhiteSpace($env:SMP_UPDATER_PUBLIC_KEY) -or
    [string]::IsNullOrWhiteSpace($env:TAURI_SIGNING_PRIVATE_KEY)) {
    throw 'Updater public key and private signing key are required'
}
$root = (git rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Not in the repository checkout' }
$cli = Join-Path $env:RUNNER_TEMP 'tauri-cli/node_modules/.bin/tauri.cmd'
if (-not (Test-Path $cli)) {
    npm install --prefix "$env:RUNNER_TEMP/tauri-cli" --no-save --package-lock=false @tauri-apps/cli@2.8.4
    if ($LASTEXITCODE -ne 0) { throw 'Tauri CLI installation failed' }
}
$configPath = Join-Path $env:RUNNER_TEMP 'desktop-release-config.json'
@{ version=$Version; bundle=@{ createUpdaterArtifacts=$true } } |
    ConvertTo-Json -Depth 4 | Set-Content $configPath -Encoding utf8
Push-Location "$root/apps/desktop/src-tauri"
try {
    & $cli build --config $configPath --bundles nsis --target x86_64-pc-windows-msvc -- --locked
    if ($LASTEXITCODE -ne 0) { throw 'Signed Windows build failed' }
} finally { Pop-Location }
git diff --exit-code -- Cargo.lock
if ($LASTEXITCODE -ne 0) { throw 'Build modified Cargo.lock' }
$installers = @(Get-ChildItem "$root/target/x86_64-pc-windows-msvc/release/bundle/nsis/*-setup.exe" -File)
if ($installers.Count -ne 1) { throw 'Expected exactly one installer' }
$installer = $installers[0]
$signature = $installer.FullName + '.sig'
if (-not (Test-Path $signature -PathType Leaf)) { throw 'Updater signature is missing' }
cargo run --locked --release --target x86_64-pc-windows-msvc -p systems-modeler-desktop --example verify_update_signature -- $installer.FullName $signature
if ($LASTEXITCODE -ne 0) { throw 'Installer signature does not match the embedded public key' }
$verifier = "$root/target/x86_64-pc-windows-msvc/release/examples/verify_update_signature.exe"
$corrupted = Join-Path $env:RUNNER_TEMP 'corrupted-update.exe'
$bytes = [System.IO.File]::ReadAllBytes($installer.FullName)
$bytes[0] = $bytes[0] -bxor 1
[System.IO.File]::WriteAllBytes($corrupted, $bytes)
& $verifier $corrupted $signature
if ($LASTEXITCODE -eq 0) { throw 'Verifier accepted corrupted update bytes' }
Remove-Item $corrupted
$wrongKeyPath = Join-Path $env:RUNNER_TEMP 'negative-test-signing.key'
& $cli signer generate --ci --write-keys $wrongKeyPath
if ($LASTEXITCODE -ne 0) { throw 'Negative signature test key creation failed' }
$publicKey = $env:SMP_UPDATER_PUBLIC_KEY
try {
    $env:SMP_UPDATER_PUBLIC_KEY = (Get-Content "$wrongKeyPath.pub" -Raw).Trim()
    & $verifier $installer.FullName $signature
    if ($LASTEXITCODE -eq 0) { throw 'Verifier accepted a different signing identity' }
} finally {
    $env:SMP_UPDATER_PUBLIC_KEY = $publicKey
    Remove-Item $wrongKeyPath, "$wrongKeyPath.pub"
}
$installDir = Join-Path $env:RUNNER_TEMP 'smp-signed-install-smoke'
$setup = Start-Process -FilePath $installer.FullName -ArgumentList "/S /D=$installDir" -Wait -PassThru
if ($setup.ExitCode -ne 0) { throw "Installer exited with $($setup.ExitCode)" }
$appPath = Join-Path $installDir 'systems-modeler-desktop.exe'
if (-not (Test-Path $appPath -PathType Leaf)) { throw 'Installed executable is missing' }
$app = Start-Process -FilePath $appPath -WorkingDirectory $installDir -PassThru
try {
    if ($app.WaitForExit(15000)) { throw "Signed application exited at startup: $($app.ExitCode)" }
} finally {
    if (-not $app.HasExited) { Stop-Process -Id $app.Id -Force }
}
$dist = Join-Path $root 'dist'
New-Item -ItemType Directory -Path $dist -Force | Out-Null
Copy-Item $installer.FullName, $signature $dist
@{
    source_commit=(git rev-parse HEAD).Trim()
    application_version=$Version
    target='x86_64-pc-windows-msvc'
    tauri_cli='2.8.4'
    installer=$installer.Name
    sha256=(Get-FileHash $installer.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    signature_verified=$true
    run_url="$env:GITHUB_SERVER_URL/$env:GITHUB_REPOSITORY/actions/runs/$env:GITHUB_RUN_ID"
} | ConvertTo-Json | Set-Content "$dist/BUILD_INFO.json" -Encoding utf8
# Expected-negative verifier exits above must not become this step's exit status.
$global:LASTEXITCODE = 0
