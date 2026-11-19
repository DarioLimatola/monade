@echo off
rem Specchio, the interface of the Monade: double click to open it in the browser.
rem Close this window (or press Ctrl+C) to stop it.
cd /d "%~dp0"
cargo run --release -p monade-run -- specchio
pause
