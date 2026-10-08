; Adds "Crispy" to the Explorer "Send to" menu, so files can be sent to another computer with a
; right-click. Removed again on uninstall.
!macro NSIS_HOOK_POSTINSTALL
  CreateShortCut "$SENDTO\Crispy.lnk" "$INSTDIR\Crispy.exe" "--send" "$INSTDIR\Crispy.exe" 0
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  Delete "$SENDTO\Crispy.lnk"
!macroend
