@echo off
setlocal
cd /d "%~dp0"
chcp 65001 >nul

rem 版本号统一写在 src-tauri\tauri.conf.json，打包时自动带入产物文件名
rem cargo 未加入系统 PATH 时补上默认安装位置，否则 tauri 找不到构建工具
if exist "%USERPROFILE%\.cargo\bin\cargo.exe" set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
where cargo >nul 2>nul
if errorlevel 1 (
  echo [错误] 没有找到 cargo，请先安装 Rust 工具链 https://rustup.rs
  goto fail
)

if not exist "node_modules" (
  echo [1/3] 安装前端依赖
  call npm install
  if errorlevel 1 goto fail
)

echo [2/3] 编译前端与 Rust 程序，打包安装包
call npm run tauri build
if errorlevel 1 goto fail

echo [3/3] 完成
echo.
echo 程序本体目录：%CD%\src-tauri\target\release
dir /b "src-tauri\target\release\*.exe"
echo 安装包目录：%CD%\src-tauri\target\release\bundle\nsis
dir /b "src-tauri\target\release\bundle\nsis\*.exe"
echo.
pause
exit /b 0

:fail
echo.
if exist "src-tauri\target\release\*.exe" (
  echo 程序本体已生成：%CD%\src-tauri\target\release
  echo 安装包没有生成，原因见上面的报错。常见原因是下载 NSIS 打包工具失败。
) else (
  echo 打包失败，原因见上面的报错。
)
pause
exit /b 1
