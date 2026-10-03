; Crochets de l'installeur de Frogtend (Tauri, bundle > windows > nsis > installerHooks).
;
; Le 03/10/2026, une mise à jour de 0.9.1 vers 0.43.1 a effacé les données de Seb (profil, ludothèque) : l'installeur
; propose de désinstaller l'ancienne version, et la case « Supprimer les données de l'application » de la
; désinstallation efface %APPDATA%\fr.hikari-no-sekai.frogtend. Règle de Frogtend : rien ne s'efface sans preuve.
; La case est respectée, mais une copie des données est d'abord mise de côté, à côté :
; %APPDATA%\fr.hikari-no-sekai.frogtend.avant-desinstallation (jamais écrasée : une copie plus ancienne est gardée).
; Le jeton, lui, est dans le coffre de Windows : la désinstallation n'y touche pas.

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
