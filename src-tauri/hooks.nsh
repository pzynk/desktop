!macro NSIS_HOOK_POSTINSTALL
  RegDLL "$INSTDIR\resources\obs-virtualcam-module64.dll"

  ; Set registry values for 64-bit applications
  SetRegView 64
  WriteRegStr HKLM "SOFTWARE\Classes\CLSID\{A3FCE0F5-3493-419F-958A-ABA1250EC20B}" "" "Sync Camera"
  WriteRegStr HKLM "SOFTWARE\Classes\CLSID\{860BB310-5D01-11d0-BD3B-00A0C911CE86}\Instance\{A3FCE0F5-3493-419F-958A-ABA1250EC20B}" "FriendlyName" "Sync Camera"
  WriteRegStr HKLM "SOFTWARE\Classes\CLSID\{860BB310-5D01-11d0-BD3B-00A0C911CE86}\Instance\{A3FCE0F5-3493-419F-958A-ABA1250EC20B}" "" "Sync Camera"

  ; Set registry values for 32-bit applications
  SetRegView 32
  WriteRegStr HKLM "SOFTWARE\Classes\CLSID\{A3FCE0F5-3493-419F-958A-ABA1250EC20B}" "" "Sync Camera"
  WriteRegStr HKLM "SOFTWARE\Classes\CLSID\{860BB310-5D01-11d0-BD3B-00A0C911CE86}\Instance\{A3FCE0F5-3493-419F-958A-ABA1250EC20B}" "FriendlyName" "Sync Camera"
  WriteRegStr HKLM "SOFTWARE\Classes\CLSID\{860BB310-5D01-11d0-BD3B-00A0C911CE86}\Instance\{A3FCE0F5-3493-419F-958A-ABA1250EC20B}" "" "Sync Camera"

  ; Restore default registry view
  SetRegView Default
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  UnRegDLL "$INSTDIR\resources\obs-virtualcam-module64.dll"
!macroend
