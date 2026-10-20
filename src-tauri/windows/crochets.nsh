; Crochets de l'installeur de Frogtend (Tauri, bundle > windows > nsis > installerHooks).
;
; Le 03/10/2026, une mise à jour de 0.9.1 vers 0.43.1 a effacé les données de Seb (profil, ludothèque) : l'installeur
; propose de désinstaller l'ancienne version, et la case « Supprimer les données de l'application » de la
; désinstallation efface %APPDATA%\fr.hikari-no-sekai.frogtend. Règle de Frogtend : rien ne s'efface sans preuve.
; La case est respectée, mais une copie des données est d'abord mise de côté, à côté :
; %APPDATA%\fr.hikari-no-sekai.frogtend.avant-desinstallation (jamais écrasée : une copie plus ancienne est gardée).
; Le jeton, lui, est dans le coffre de Windows : la désinstallation n'y touche pas.

; Les liens nxm:// de Nexus Mods (lot 0.82.0 ; ✅ Seb 19/10 : par l'INSTALLEUR ; 20/10 : SEULEMENT à la 1re
; installation, jamais repris par une mise à jour ; rendus à la désinstallation au programme qui les gérait avant,
; Vortex ou Mod Organizer, s'il existe encore). Tout dans HKCU (Frogtend s'installe pour l'utilisateur) : une
; association de tout le PC (HKLM) n'est jamais touchée, et redevient visible quand la nôtre est retirée.
; Les clés sont écrites en toutes lettres dans les macros : ce fichier est inclus AVANT la définition de BUNDLEID.

!macro NSIS_HOOK_POSTINSTALL
  ReadRegStr $0 HKCU "Software\${BUNDLEID}" "nxm_pris"
  ${If} $0 == ""
    ; La commande de celui qui gérait nxm:// avant (vide : personne, ou seulement pour tout le PC).
    ReadRegStr $1 HKCU "Software\Classes\nxm\shell\open\command" ""
    WriteRegStr HKCU "Software\${BUNDLEID}" "nxm_avant" "$1"
    WriteRegStr HKCU "Software\Classes\nxm" "" "URL:NXM Protocol"
    WriteRegStr HKCU "Software\Classes\nxm" "URL Protocol" ""
    WriteRegStr HKCU "Software\Classes\nxm\DefaultIcon" "" "$INSTDIR\${MAINBINARYNAME}.exe,0"
    WriteRegStr HKCU "Software\Classes\nxm\shell\open\command" "" '"$INSTDIR\${MAINBINARYNAME}.exe" "%1"'
    WriteRegStr HKCU "Software\${BUNDLEID}" "nxm_pris" "1"
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    ; Seulement si les liens sont encore à Frogtend (quelqu'un a pu les reprendre depuis : on n'y touche pas).
    ReadRegStr $1 HKCU "Software\Classes\nxm\shell\open\command" ""
    ${If} $1 == '"$INSTDIR\${MAINBINARYNAME}.exe" "%1"'
      DeleteRegKey HKCU "Software\Classes\nxm"
      ReadRegStr $2 HKCU "Software\${BUNDLEID}" "nxm_avant"
      ${If} $2 != ""
        ; Le programme d'avant existe-t-il encore ? (son chemin : entre les premiers guillemets, sinon jusqu'à
        ; la première espace)
        StrCpy $3 $2 1
        ${If} $3 == '"'
          StrCpy $4 $2 "" 1
          StrCpy $5 0
          ${Do}
            StrCpy $3 $4 1 $5
            ${If} $3 == '"'
            ${OrIf} $3 == ""
              ${ExitDo}
            ${EndIf}
            IntOp $5 $5 + 1
          ${Loop}
          StrCpy $4 $4 $5
        ${Else}
          StrCpy $4 $2
        ${EndIf}
        ${If} ${FileExists} "$4"
          WriteRegStr HKCU "Software\Classes\nxm" "" "URL:NXM Protocol"
          WriteRegStr HKCU "Software\Classes\nxm" "URL Protocol" ""
          WriteRegStr HKCU "Software\Classes\nxm\shell\open\command" "" "$2"
        ${EndIf}
      ${EndIf}
    ${EndIf}
    DeleteRegValue HKCU "Software\${BUNDLEID}" "nxm_avant"
    DeleteRegValue HKCU "Software\${BUNDLEID}" "nxm_pris"
    DeleteRegKey /ifempty HKCU "Software\${BUNDLEID}"
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    SetShellVarContext current
    ${If} ${FileExists} "$APPDATA\${BUNDLEID}\*.*"
    ${AndIfNot} ${FileExists} "$APPDATA\${BUNDLEID}.avant-desinstallation\*.*"
      CreateDirectory "$APPDATA\${BUNDLEID}.avant-desinstallation"
      CopyFiles /SILENT "$APPDATA\${BUNDLEID}\*.*" "$APPDATA\${BUNDLEID}.avant-desinstallation"
    ${EndIf}
  ${EndIf}
!macroend
