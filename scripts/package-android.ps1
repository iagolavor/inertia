# Package inertia-api (jniLibs) + web (assets) for on-device Android. Mirrors Windows zip layout.

# Usage: powershell -ExecutionPolicy Bypass -File scripts/package-android.ps1



$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot



$apiBin = Join-Path $root 'dist/android-arm64/inertia-api'

$webBuild = Join-Path $root 'apps/web/build'

$assetsRoot = Join-Path $root 'apps/web/android/app/src/main/assets/inertia'

$jniLibs = Join-Path $root 'apps/web/android/app/src/main/jniLibs/arm64-v8a'

$jniLibName = 'libinertia_api.so'



if (-not (Test-Path $apiBin)) {

    throw "Missing $apiBin - run: npm run android:api:build"

}

if (-not (Test-Path (Join-Path $webBuild 'index.html'))) {

    throw "Missing apps/web/build - run: npm run web:build"

}



if (Test-Path $assetsRoot) {

    Remove-Item $assetsRoot -Recurse -Force

}

New-Item -ItemType Directory -Path $assetsRoot -Force | Out-Null

Copy-Item $webBuild (Join-Path $assetsRoot 'web') -Recurse

$bundleId = (Get-FileHash (Join-Path $webBuild 'index.html') -Algorithm SHA256).Hash.Substring(0, 12)
# No leading dot — aapt ignoreAssetsPattern `.*` drops dotfiles from the APK.
Set-Content -Path (Join-Path $assetsRoot 'web/bundle-id') -Value $bundleId -NoNewline

New-Item -ItemType Directory -Path $jniLibs -Force | Out-Null

Copy-Item $apiBin (Join-Path $jniLibs $jniLibName) -Force



Write-Host "Packaged web assets at $assetsRoot"

Write-Host "Packaged API binary at $jniLibs\$jniLibName"

Write-Host "Next: npm run android:sync (or rebuild/install from Android Studio)"


