# Install Android SDK command-line tools + packages required by apps/web/android (Capacitor).
# Uses the JDK bundled with Android Studio. Run once after winget install Google.AndroidStudio.

$ErrorActionPreference = 'Stop'

$SdkRoot = Join-Path $env:LOCALAPPDATA 'Android\Sdk'
$JavaHome = 'C:\Program Files\Android\Android Studio\jbr'
$StudioBin = 'C:\Program Files\Android\Android Studio\bin\studio64.exe'
$CmdlineZip = Join-Path $env:TEMP 'commandlinetools-win.zip'
$CmdlineUrl = 'https://dl.google.com/android/repository/commandlinetools-win-13114758_latest.zip'
$RepoRoot = Split-Path $PSScriptRoot -Parent
$LocalProps = Join-Path $RepoRoot 'apps\web\android\local.properties'

if (-not (Test-Path $JavaHome)) {
    Write-Error "Android Studio JDK not found at $JavaHome. Install with: winget install Google.AndroidStudio"
}

New-Item -ItemType Directory -Force -Path $SdkRoot | Out-Null

if (-not (Test-Path (Join-Path $SdkRoot 'cmdline-tools\latest\bin\sdkmanager.bat'))) {
    Write-Host "Downloading Android command-line tools..."
    Invoke-WebRequest -Uri $CmdlineUrl -OutFile $CmdlineZip -UseBasicParsing
    $ExtractRoot = Join-Path $env:TEMP 'android-cmdline-tools'
    if (Test-Path $ExtractRoot) { Remove-Item $ExtractRoot -Recurse -Force }
    Expand-Archive -Path $CmdlineZip -DestinationPath $ExtractRoot -Force
    $LatestDir = Join-Path $SdkRoot 'cmdline-tools\latest'
    New-Item -ItemType Directory -Force -Path (Split-Path $LatestDir) | Out-Null
    if (Test-Path $LatestDir) { Remove-Item $LatestDir -Recurse -Force }
    Move-Item (Join-Path $ExtractRoot 'cmdline-tools') $LatestDir
}

$env:JAVA_HOME = $JavaHome
$env:ANDROID_HOME = $SdkRoot
$env:ANDROID_SDK_ROOT = $SdkRoot
$env:Path = "$JavaHome\bin;$SdkRoot\platform-tools;$SdkRoot\cmdline-tools\latest\bin;" + $env:Path

Write-Host "Accepting SDK licenses..."
$yes = ('y' + [Environment]::NewLine) * 200
$yes | & sdkmanager.bat --licenses | Out-Null

Write-Host "Installing platform-tools, Android 35, build-tools 35.0.0..."
& sdkmanager.bat 'platform-tools' 'platforms;android-35' 'build-tools;35.0.0'

$sdkDirEscaped = ($SdkRoot -replace '\\', '\\')
@("sdk.dir=$sdkDirEscaped") | Set-Content -Path $LocalProps -Encoding ascii

[Environment]::SetEnvironmentVariable('ANDROID_HOME', $SdkRoot, 'User')
[Environment]::SetEnvironmentVariable('ANDROID_SDK_ROOT', $SdkRoot, 'User')
[Environment]::SetEnvironmentVariable('JAVA_HOME', $JavaHome, 'User')
if (Test-Path $StudioBin) {
    [Environment]::SetEnvironmentVariable(
        'CAPACITOR_ANDROID_STUDIO_PATH',
        $StudioBin,
        'User'
    )
}

Write-Host ""
Write-Host "Done. SDK: $SdkRoot"
Write-Host "Wrote $LocalProps"
Write-Host "Set user env: ANDROID_HOME, JAVA_HOME, CAPACITOR_ANDROID_STUDIO_PATH"
Write-Host "Restart terminals / Android Studio, then: npm run android:sync && npm run android:open"
