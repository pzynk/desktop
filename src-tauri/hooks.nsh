!macro NSIS_HOOK_POSTINSTALL
  RegDLL "$INSTDIR\resources\obs-virtualcam-module64.dll"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  UnRegDLL "$INSTDIR\resources\obs-virtualcam-module64.dll"
!macroend
