;; BridgeLab NSIS installer hooks (bundle.windows.nsis.installerHooks).
;;
;; After installing, ask whether BridgeLab may look for new versions at
;; startup (default: Yes) and record the answer in
;; %APPDATA%\BridgeLab\installer.json, which the app turns into the
;; "Check for new versions at startup" preference the first time it runs.
;; The user can change it any time under Settings > Privacy.
;;
;; - Asked once: an update over an existing installation that already has
;;   an answer does not ask again.
;; - Silent installs (/S) are never interrupted: no file is written, so
;;   the app's default applies. Managed sites turn the check off
;;   machine-wide with %ProgramData%\BridgeLab\policy.json or the
;;   BRIDGELAB_DISABLE_UPDATE_CHECK environment variable instead.
;; - %APPDATA% is read from the environment, not $APPDATA, so a per-machine
;;   install (shell context "all") still writes into the installing
;;   user's own folder, where the app looks.
;;
;; This file is UTF-8 with a BOM so makensis reads the accented strings
;; correctly whatever the build machine's code page.

!macro BL_WRITE_UPDATE_CHOICE DIR VALUE
  CreateDirectory "${DIR}\BridgeLab"
  FileOpen $R9 "${DIR}\BridgeLab\installer.json" w
  FileWrite $R9 '{"update_check_on_startup": ${VALUE}}$\r$\n'
  FileClose $R9
!macroend

!macro NSIS_HOOK_POSTINSTALL
  Push $R0
  Push $R1
  Push $R9
  IfSilent bl_update_done
  ReadEnvStr $R0 APPDATA
  StrCmp $R0 "" bl_update_done
  IfFileExists "$R0\BridgeLab\installer.json" bl_update_done

  ; Question in the installer's language (English otherwise).
  StrCpy $R1 "Check for new BridgeLab versions automatically at startup?$\n$\nOnce a day BridgeLab asks GitHub (api.github.com) for the number of the latest version. Nothing about you, this computer or your files is sent.$\n$\nYou can change this later in Settings → Privacy."
  StrCmp $LANGUAGE 1040 0 +2
    StrCpy $R1 "Controllare automaticamente la disponibilità di nuove versioni di BridgeLab all'avvio?$\n$\nUna volta al giorno BridgeLab chiede a GitHub (api.github.com) il numero dell'ultima versione. Non viene inviato nulla su di te, sul computer o sui tuoi file.$\n$\nPuoi cambiare scelta in seguito da Impostazioni → Privacy."
  StrCmp $LANGUAGE 1036 0 +2
    StrCpy $R1 "Rechercher automatiquement les nouvelles versions de BridgeLab au démarrage ?$\n$\nUne fois par jour, BridgeLab demande à GitHub (api.github.com) le numéro de la dernière version. Rien sur vous, cet ordinateur ou vos fichiers n'est envoyé.$\n$\nVous pourrez modifier ce choix dans Paramètres → Confidentialité."
  StrCmp $LANGUAGE 1034 0 +2
    StrCpy $R1 "¿Buscar automáticamente nuevas versiones de BridgeLab al iniciar?$\n$\nUna vez al día BridgeLab consulta a GitHub (api.github.com) el número de la última versión. No se envía nada sobre ti, este equipo o tus archivos.$\n$\nPuedes cambiarlo más tarde en Configuración → Privacidad."
  StrCmp $LANGUAGE 3082 0 +2
    StrCpy $R1 "¿Buscar automáticamente nuevas versiones de BridgeLab al iniciar?$\n$\nUna vez al día BridgeLab consulta a GitHub (api.github.com) el número de la última versión. No se envía nada sobre ti, este equipo o tus archivos.$\n$\nPuedes cambiarlo más tarde en Configuración → Privacidad."
  StrCmp $LANGUAGE 1031 0 +2
    StrCpy $R1 "Beim Start automatisch nach neuen BridgeLab-Versionen suchen?$\n$\nEinmal täglich fragt BridgeLab GitHub (api.github.com) nach der Nummer der neuesten Version. Es wird nichts über Sie, diesen Computer oder Ihre Dateien gesendet.$\n$\nSie können dies später unter Einstellungen → Datenschutz ändern."

  MessageBox MB_YESNO|MB_ICONQUESTION|MB_DEFBUTTON1 "$R1" /SD IDYES IDNO bl_update_no
    !insertmacro BL_WRITE_UPDATE_CHOICE "$R0" "true"
    Goto bl_update_done
  bl_update_no:
    !insertmacro BL_WRITE_UPDATE_CHOICE "$R0" "false"
  bl_update_done:
  Pop $R9
  Pop $R1
  Pop $R0
!macroend
