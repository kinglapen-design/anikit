@echo off
rem AniKit CEP spike —— 安装到用户级 CEP extensions 目录（PlayerDebugMode 本机已开）
set "DEST=%APPDATA%\Adobe\CEP\extensions\com.anikit.cep"
if exist "%DEST%" rmdir /S /Q "%DEST%"
xcopy /E /I /Y "%~dp0extension" "%DEST%" >nul
echo installed to %DEST%
