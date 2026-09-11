@echo off
REM Local mirror of .github\workflows\release.yml: Release build of the single
REM exe into artifacts\ plus a SHA256SUMS manifest in sha256sum format. CI is
REM the shipping build; this exists to reproduce or try it before tagging.
REM MinVer stamps the version from the nearest v* tag, so tag first if the
REM reported version matters.
setlocal

REM ROOT is the project dir without %~dp0's trailing backslash (a trailing "\"
REM before a closing quote escapes the quote).
set "ROOT=%~dp0"
set "ROOT=%ROOT:~0,-1%"
set "OUT=%ROOT%\artifacts"

if exist "%OUT%" rmdir /S /Q "%OUT%"
mkdir "%OUT%"

dotnet build "%ROOT%\Weatherspell.slnx" -c Release -clp:ErrorsOnly
if errorlevel 1 exit /b 1
dotnet test "%ROOT%\Weatherspell.slnx" -c Release --no-build
if errorlevel 1 exit /b 1

copy /Y "%ROOT%\src\Weatherspell\bin\Release\net48\Weatherspell.exe" "%OUT%\" >nul
if errorlevel 1 exit /b 1

REM sha256sum format, LF-terminated, matching what release.yml attaches.
for /f %%H in ('powershell -NoProfile -Command "(Get-FileHash -LiteralPath '%OUT%\Weatherspell.exe' -Algorithm SHA256).Hash.ToLower()"') do (
  powershell -NoProfile -Command "[IO.File]::WriteAllText('%OUT%\SHA256SUMS', '%%H  Weatherspell.exe' + [char]10)"
)
if not exist "%OUT%\SHA256SUMS" (
  echo Could not compute the Weatherspell.exe checksum.
  exit /b 1
)

echo.
echo Built:     %OUT%\Weatherspell.exe
echo Checksums: %OUT%\SHA256SUMS
