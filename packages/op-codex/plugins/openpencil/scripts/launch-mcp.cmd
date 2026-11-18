@echo off
setlocal DisableDelayedExpansion
rem Resolve the same private Node registration as the POSIX launcher.
set "state_dir=%USERPROFILE%\.codex-openpencil"
set "node_bin="
if exist "%state_dir%\node-path" set /p "node_bin=" < "%state_dir%\node-path"
if not defined node_bin goto path_node
if not exist "%node_bin%" goto path_node
"%node_bin%" -e "process.exit(Number(process.versions.node.split('.')[0]) >= 22 ? 0 : 1)" >nul 2>&1
if errorlevel 1 goto path_node
"%node_bin%" "%~dp0mcp-bridge.mjs" %*
exit /b %errorlevel%
:path_node
node -e "process.exit(Number(process.versions.node.split('.')[0]) >= 22 ? 0 : 1)" >nul 2>&1
if errorlevel 1 goto missing_node
node "%~dp0mcp-bridge.mjs" %*
exit /b %errorlevel%
:missing_node
echo OpenPencil requires Node.js 22+. Run npm run setup from its repository, then reconnect the plugin. >&2
exit /b 1
