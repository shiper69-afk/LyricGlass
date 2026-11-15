; LyricGlass NSIS hooks — process check only (no Tauri Restart Manager macro).
;
; Tauri's CheckIfAppIsRunning uses Windows Restart Manager on the main binary path.
; During uninstall this can false-positive (ERROR_MORE_DATA) when no lyricglass.exe
; is running — e.g. the uninstaller/setup holds a transient registration on the path,
; or the product name matches the uninstaller window ("LyricGlass"). RM shutdown then
; fails and shows "Failed to kill LyricGlass".
;
; We instead match the exact image name lyricglass.exe via tasklist/taskkill.

!macro LyricGlass_EnsureMainNotRunning
  DetailPrint "LyricGlass: checking for lyricglass.exe before setup..."
  nsExec::ExecToStack 'cmd /c tasklist /FI "IMAGENAME eq lyricglass.exe" 2>nul | find /I "lyricglass.exe"'
  Pop $0
  Pop $1
  StrCmp $1 "" lg_proc_done
    DetailPrint "LyricGlass: stopping lyricglass.exe"
    ExecWait 'taskkill /F /IM lyricglass.exe /T' $R0
    Sleep 500
    nsExec::ExecToStack 'cmd /c tasklist /FI "IMAGENAME eq lyricglass.exe" 2>nul | find /I "lyricglass.exe"'
    Pop $0
    Pop $1
    StrCmp $1 "" lg_proc_done
      DetailPrint "LyricGlass: warning — lyricglass.exe still listed after taskkill"
  lg_proc_done:
    DetailPrint "LyricGlass: lyricglass.exe not running (OK to continue)"
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro LyricGlass_EnsureMainNotRunning
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro LyricGlass_EnsureMainNotRunning
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  DetailPrint "LyricGlass: uninstall finished"
!macroend