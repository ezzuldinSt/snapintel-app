; How SnapIntel's Windows installer looks, and what its first and last pages say.
;
; Tauri's installer script includes this file before it sets up its pages, so the
; Modern UI settings below take effect. The pictures are sidebar.bmp and header.bmp
; (drawn from the SVG files beside them by render.ps1); tauri.conf.json names them.

; The header of every page, and the whole first and last pages, in the app's colours.
!define MUI_BGCOLOR "16161A"
!define MUI_TEXTCOLOR "F3F3F6"
!define MUI_HEADERIMAGE_RIGHT
; Without this, Windows draws the last page's checkbox labels in black, whatever colour is asked for.
!define MUI_FORCECLASSICCONTROLS

; The progress bar and the list of installed files, in the same colours.
!define MUI_INSTFILESPAGE_PROGRESSBAR "smooth colored"
!define MUI_INSTFILESPAGE_COLORS "FFDF3D 16161A"

; Windows ignores a progress bar's colours while it draws the bar in the system style,
; so the bar is switched to the plain style when the work starts.
!macro SNAPINTEL_PLAIN_PROGRESSBAR
  FindWindow $0 "#32770" "" $HWNDPARENT
  GetDlgItem $0 $0 1004
  System::Call 'UXTHEME::SetWindowTheme(p r0, w " ", w " ")'
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro SNAPINTEL_PLAIN_PROGRESSBAR
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro SNAPINTEL_PLAIN_PROGRESSBAR
!macroend

; First page.
!define MUI_WELCOMEPAGE_TITLE "Welcome to SnapIntel"
!define MUI_WELCOMEPAGE_TEXT "See what a public Snapchat profile shares, and keep a copy you can verify.$\r$\n$\r$\nSetup installs SnapIntel for your Windows account only. It needs no administrator rights, and the app needs no Snapchat account.$\r$\n$\r$\nChoose Next to continue."

; Last page.
!define MUI_FINISHPAGE_TITLE "SnapIntel is ready"
!define MUI_FINISHPAGE_TEXT "SnapIntel is installed, and you will find it in the Start menu.$\r$\n$\r$\nTo begin, type a username or paste a profile link and press Enter."
!define MUI_FINISHPAGE_LINK "Open the user guide"
!define MUI_FINISHPAGE_LINK_LOCATION "https://github.com/ezzuldinSt/snapintel-app/blob/main/docs/README.md"
!define MUI_FINISHPAGE_LINK_COLOR "FFDF3D"
