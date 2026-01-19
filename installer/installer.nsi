Name "komorebi-switcher"
OutFile "komorebi-switcher-setup.exe"
Unicode true
ManifestDPIAware true
ManifestDPIAwareness PerMonitorV2
SetCompressor /SOLID lzma
InstallDir "$LOCALAPPDATA\komorebi-switcher"
Icon "icon.ico"

!include "MUI2.nsh"
!include "FileFunc.nsh"

RequestExecutionLevel user

!define PRODUCTNAME "komorebi-switcher"
!define MAINBINARYNAME "komorebi-switcher.exe"
!define UNINSTKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCTNAME}"
!define VERSION "0.9.2"
!define PUBLISHER "Amr Bashir"

VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "${PRODUCTNAME}"
VIAddVersionKey "FileDescription" "${PRODUCTNAME}"
VIAddVersionKey "FileVersion" "${VERSION}.0"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "CompanyName" "${PUBLISHER}"
VIAddVersionKey "LegalCopyright" "© 2025 ${PUBLISHER}. Licensed under the MIT License."

!define MUI_ICON "icon.ico"
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_NOAUTOCLOSE
!define MUI_FINISHPAGE_SHOWREADME
!define MUI_FINISHPAGE_SHOWREADME_TEXT "Create desktop shortcut"
!define MUI_FINISHPAGE_SHOWREADME_FUNCTION CreateDesktopShortcut
!define MUI_FINISHPAGE_RUN "$INSTDIR\${MAINBINARYNAME}"
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

Section
    SetOutPath "$INSTDIR"

    File "${MAINBINARYNAME}"

    WriteUninstaller "$INSTDIR\uninstall.exe"

    WriteRegStr HKCU "${UNINSTKEY}" "DisplayName" "${PRODUCTNAME}"
    WriteRegStr HKCU "${UNINSTKEY}" "DisplayIcon" "$\"$INSTDIR\${MAINBINARYNAME}$\""
    WriteRegStr HKCU "${UNINSTKEY}" "DisplayVersion" "${VERSION}"
    WriteRegStr HKCU "${UNINSTKEY}" "Publisher" "Amr Bashir"
    WriteRegStr HKCU "${UNINSTKEY}" "InstallLocation" "$\"$INSTDIR$\""
    WriteRegStr HKCU "${UNINSTKEY}" "UninstallString" "$\"$INSTDIR\uninstall.exe$\""
    WriteRegDWORD HKCU "${UNINSTKEY}" "NoModify" "1"
    WriteRegDWORD HKCU "${UNINSTKEY}" "NoRepair" "1"

    ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
    IntFmt $0 "0x%08X" $0
    WriteRegDWORD HKCU "${UNINSTKEY}" "EstimatedSize" "$0"

    CreateShortcut "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}"
SectionEnd

Section Uninstall
    Delete "$INSTDIR\${MAINBINARYNAME}"

    Delete "$INSTDIR\uninstall.exe"

    Delete "$SMPROGRAMS\${PRODUCTNAME}.lnk"

    Delete "$DESKTOP\${PRODUCTNAME}.lnk"

    DeleteRegKey HKCU "${UNINSTKEY}"
SectionEnd

Function CreateDesktopShortcut
  CreateShortcut "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}"
FunctionEnd
