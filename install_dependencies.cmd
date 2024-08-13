@echo off
echo %PATH% | find /C /I "C:\ProgramData\chocolatey\bin" >nul
set is_choco=%ERRORLEVEL%
if %is_choco%==1 (
    echo "Chocolatey installation found, downloading dependencies."

    echo "Installing cmake:"
    choco install cmake -y - set path=C:\Program Files\CMake\bin;%path%

    echo "Installing pkgconfig:"
    ::We manually define the said path because we define the version
    choco install pkgconfiglite -y --version 0.28.0 - set path=C:\ProgramData\chocolatey\lib\pkgconfiglite\tools\pkg-config-lite-0.28-1\bin;%path%

    echo "Installing opencv:"
    choco install opencv -y --version 4.10.0

    ::Set environment variables, hopefully these will work
    setx %OPENCV_INCLUDE_PATHS% "C:\tools\opencv\build\include"
    setx %OPENCV_LINK_LIBS% "opencv_world4100"
    setx %OPENCV_LINK_PATHS% "C:\tools\opencv\build\x64\vc16\lib"

    echo "All packages installed successfully! Please check environment variables and config the rest"
) else (
    echo "Chocolatey is not installed, you will have to install chocolatey first to install the dependencies automatically."
)

pause
