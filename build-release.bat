@echo off
setlocal

cargo build --release
if errorlevel 1 (
    echo Build failed.
    exit /b 1
)

set "SRC=target\release\gores-map-downloader.exe"
set "EXE=target\release\Gores Map Downloader.exe"
set "ZIP=target\release\gores-map-downloader-x86_64-pc-windows-msvc.zip"

if not exist "%SRC%" (
    echo ERROR: %SRC% not found.
    exit /b 1
)

copy /Y "%SRC%" "%EXE%"

if exist "%ZIP%" del "%ZIP%"
powershell -NoProfile -Command "Compress-Archive -Path '%EXE%' -DestinationPath '%ZIP%'"

echo.
echo Release built:
echo   EXE: %EXE%
echo   ZIP: %ZIP%
