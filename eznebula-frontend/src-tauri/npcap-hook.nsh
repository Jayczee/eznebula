; Install Npcap silently during setup
!macro customInstall
  ; Check if Npcap is already installed
  IfFileExists "$SYSDIR\Npcap\wpcap.dll" npcap_done npcap_install
  npcap_install:
    DetailPrint "Installing Npcap driver (one-time setup)..."
    ; Run the bundled Npcap installer silently
    ExecWait '"$INSTDIR\resources\binaries\npcap-installer.exe" /S' $0
    DetailPrint "Npcap installer exit code: $0"
  npcap_done:
!macroend
