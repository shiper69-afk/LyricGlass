; Custom NSIS hooks for LyricGlass (Tauri bundler).
; Mitigate false-positive "Failed to kill LyricGlass" when no process is running
; (stale PID / TOCTOU in older nsis-process helpers).

!macro NSIS_HOOK_PREUNINSTALL
  DetailPrint "LyricGlass: checking for lyricglass.exe before uninstall..."
  nsExec::ExecToStack 'cmd /c tasklist /FI "IMAGENAME eq lyricglass.exe" 2>nul | find /I "lyricglass.exe"'
  Pop $0
  Pop $1
  StrCmp $1 "" not_running
    DetailPrint "LyricGlass: stopping running lyricglass.exe"
    ExecWait 'taskkill /F /IM lyricglass.exe /T' $R0
    Sleep 500
  not_running:
    DetailPrint "LyricGlass: no lyricglass.exe process found"
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  DetailPrint "LyricGlass: uninstall finished"
!macroend