; XCopy installer hooks.
;
; Adds two behaviors on top of the default Tauri NSIS installer:
;
;   1. After a successful install (including silent /S and passive installs),
;      enable "launch on Windows startup" by writing the same registry value
;      that the app itself uses (see src/app_settings.rs -> AUTOSTART_VALUE_NAME
;      and the `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` key).
;
;   2. On uninstall, remove that value so no dead startup entry is left behind.
;
; Both ends share the *same* Run-key value name "XCopy", so the in-app
; settings page reads/writes exactly what the installer wrote. A user who
; later turns the toggle off in Settings simply deletes this value; a
; reinstall will re-create it (intended: "default on").
;
; Failures here must never abort the install/uninstall, so registry writes
; are wrapped in ClearErrors and results are discarded.

!define XC_AUTOSTART_RUN_KEY "Software\Microsoft\Windows\CurrentVersion\Run"
!define XC_AUTOSTART_VALUE_NAME "XCopy"

; Path passed to the Run key: the installed exe, quoted so paths with spaces
; survive the Windows command-line parser. Matches the format the app writes
; in app_settings.rs::set_platform_auto_start_enabled:
;   format!("\"{}\"", exe.display())

!macro XC_WriteAutostart
  ClearErrors
  WriteRegStr HKCU "${XC_AUTOSTART_RUN_KEY}" "${XC_AUTOSTART_VALUE_NAME}" `"$INSTDIR\${MAINBINARYNAME}.exe"`
  ClearErrors
!macroend

!macro XC_DeleteAutostart
  ClearErrors
  DeleteRegValue HKCU "${XC_AUTOSTART_RUN_KEY}" "${XC_AUTOSTART_VALUE_NAME}"
  ClearErrors
!macroend

; Runs at the end of the Install section, after files/shortcuts/registry
; are written. Tauri only calls this when the macro is defined.
!macro NSIS_HOOK_POSTINSTALL
  !insertmacro XC_WriteAutostart
!macroend

; Runs at the end of the Uninstall section.
!macro NSIS_HOOK_POSTUNINSTALL
  !insertmacro XC_DeleteAutostart
!macroend
