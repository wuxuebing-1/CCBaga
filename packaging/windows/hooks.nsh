; File-backed checks avoid NSIS's 1024-character runtime string limit.
; Never stops processes, removes profiles or changes UserChoice.
!include x64.nsh
!define CCBAGA_GUARD_SOURCE "${__FILEDIR__}\browser-guard.ps1"
LangString CCBagaBrowserBusy 1033 "CCBaga-B is still running from this installation. Close its windows and retry. No browser processes were stopped."
LangString CCBagaBrowserBusy 2052 "此安装目录中的 CCBaga-B 仍在运行。请关闭它的浏览器窗口后重试；安装器没有结束任何浏览器进程。"
LangString CCBagaBrowserUnknown 1033 "Could not check this installation's browser (code $0). Close CCBaga-B and retry. No application files were changed by this check."
LangString CCBagaBrowserUnknown 2052 "无法检查此安装目录的浏览器占用（错误码 $0）。请关闭 CCBaga-B 后重试；此检查没有更改应用文件。"

!macro CCBAGA_CHECK_BROWSER tag
  Push $0
  Push $1
  Push $2
  InitPluginsDir
  ; The full script is an installer resource, never a long command-line string.
  File /oname=$PLUGINSDIR\ccbaga-browser-guard.ps1 "${CCBAGA_GUARD_SOURCE}"
  StrCpy $2 "$SYSDIR\WindowsPowerShell\v1.0\powershell.exe"
  ${If} ${RunningX64}
    ; NSIS itself is x86 even for the x64 application. Avoid WOW64 redirection.
    StrCpy $2 "$WINDIR\Sysnative\WindowsPowerShell\v1.0\powershell.exe"
  ${EndIf}
  System::Call 'kernel32::SetEnvironmentVariableW(w "CCBAGA_INSTALL_ENGINE", w "$INSTDIR\browser-runtime\WindowsBrowser") i.r0'
  StrCmp $0 "0" ccbaga_${tag}_unknown
  nsExec::ExecToStack /TIMEOUT=30000 '"$2" -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$PLUGINSDIR\ccbaga-browser-guard.ps1"'
  Pop $0
  Pop $1
  System::Call 'kernel32::SetEnvironmentVariableW(w "CCBAGA_INSTALL_ENGINE", p 0)'
  DetailPrint "CCBaga browser check: $0"
  StrCmp $0 "0" ccbaga_${tag}_done
  StrCmp $0 "12" ccbaga_${tag}_running ccbaga_${tag}_unknown
  ccbaga_${tag}_running:
    MessageBox MB_ICONEXCLAMATION|MB_OK "$(CCBagaBrowserBusy)" /SD IDOK
    SetErrorLevel 12
    Pop $2
    Pop $1
    Pop $0
    Abort
  ccbaga_${tag}_unknown:
    MessageBox MB_ICONEXCLAMATION|MB_OK "$(CCBagaBrowserUnknown)" /SD IDOK
    SetErrorLevel 13
    Pop $2
    Pop $1
    Pop $0
    Abort
  ccbaga_${tag}_done:
    Pop $2
    Pop $1
    Pop $0
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro CCBAGA_CHECK_BROWSER install
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro CCBAGA_CHECK_BROWSER uninstall
!macroend

!macro NSIS_HOOK_POSTINSTALL
  CreateShortCut "$SMPROGRAMS\CCBaga-B.lnk" "$INSTDIR\${MAINBINARYNAME}.exe" "--browser" "$INSTDIR\browser-runtime\browser.ico"
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  Delete "$SMPROGRAMS\CCBaga-B.lnk"
  ; Remove only associations registered by this installation, not another copy.
  ReadRegStr $0 HKCU "Software\Classes\CCBaga.Browser\shell\open\command" ""
  StrCmp $0 '$\"$INSTDIR\${MAINBINARYNAME}.exe$\" --browser $\"%1$\"' 0 +4
    DeleteRegKey HKCU "Software\Classes\CCBaga.Browser"
    DeleteRegKey HKCU "Software\Clients\StartMenuInternet\CCBaga-B"
    DeleteRegValue HKCU "Software\RegisteredApplications" "CCBaga-B"
!macroend
