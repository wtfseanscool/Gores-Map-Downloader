@echo off
cargo build --profile dev-release
if exist "target\dev-release\gores-map-downloader.exe" (
    copy /Y "target\dev-release\gores-map-downloader.exe" "target\dev-release\Gores Map Downloader.exe"
    echo Copied to "Gores Map Downloader.exe"
)
