@echo off
rem AniKit CEP 插件一键安装（开发期手动用；APP 内一键安装命令随 2b 落地）
rem = 复制到用户级 CEP extensions 目录（PlayerDebugMode 已开免签名）
set "DEST=%APPDATA%\Adobe\CEP\extensions\com.anikit.cep"
if exist "%DEST%" rmdir /S /Q "%DEST%"
xcopy /E /I /Y "%~dp0" "%DEST%" >nul
rem install.bat 自身不带走
del /Q "%DEST%\install.bat" 2>nul
echo installed to %DEST%
