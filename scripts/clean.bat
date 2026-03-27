@echo off
echo Wait...

rmdir /s /q .svelte-kit 2>nul
rmdir /s /q build 2>nul
rmdir /s /q node_modules\.vite 2>nul
rmdir /s /q src-tauri\target 2>nul

echo Done!
pause
