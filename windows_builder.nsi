Name "SendingUnicorns"
OutFile "SendingUnicorns Setup.exe"
InstallDir "$PROGRAMFILES\SendingUnicorns"
Unicode true
ShowInstDetails show

!addplugindir ".\target\x86_64-pc-windows-gnu\release"
!addplugindir "$%CARGO_TARGET_DIR%\x86_64-pc-windows-gnu\release"
!addplugindir "$%CARGO_BUILD_TARGET_DIR%\x86_64-pc-windows-gnu\release"

!include "MUI2.nsh"

!define MUI_ICON "src-tauri\icons\icon.ico"
!define MUI_HEADERIMAGE
!define MUI_HEADERIMAGE_RIGHT

!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"


Section

DetailPrint "Installing Sending Unicorns"
SetOutPath $INSTDIR

File src-tauri/target/x86_64-pc-windows-gnu/release/*.*

DetailPrint "Creating shortcut"
CreateShortcut "$SMPROGRAMS\Sending Unicorns.lnk" "$INSTDIR\app.exe"
DetailPrint "Creating uninstaller"
WriteUninstaller $INSTDIR\uninstaller.exe

SectionEnd

Section "Uninstall"
 
# Delete the directory
RMDir /r $INSTDIR
Delete "$SMPROGRAMS\Sending Unicorns.lnk"
SectionEnd