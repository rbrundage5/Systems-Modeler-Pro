; Tauri CLI 2.8.4 includes utils.nsh before installerHooks. Replace the actual
; install/uninstall process check, not a precheck followed by the old kill path.
!ifmacrodef CheckIfAppIsRunning
  !macroundef CheckIfAppIsRunning
!else
  !error "Tauri running-app macro is missing; requalify installer process protection."
!endif

!macro CheckIfAppIsRunning executableName productName
  !if "${INSTALLMODE}" == "currentUser"
    nsis_tauri_utils::FindProcessCurrentUser "${executableName}"
  !else
    nsis_tauri_utils::FindProcess "${executableName}"
  !endif
  Pop $R0
  ${If} $R0 = 0
    DetailPrint "Save and close all copies of ${productName}, then run the installer again."
    IfSilent +2
      MessageBox MB_OK|MB_ICONINFORMATION "Save and close all copies of ${productName}, then run the installer again. No running copy has been closed."
    SetErrorLevel 2
    Quit
  ${EndIf}
!macroend
