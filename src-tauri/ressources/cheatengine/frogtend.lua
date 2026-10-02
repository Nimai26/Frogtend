-- Frogtend : branche Cheat Engine sur le jeu lancé par Frogtend et charge sa table, sans passer par « Ouvrir un
-- processus ». Posé par Frogtend dans autorun\ ; il lit le fichier frogtend-lancement.txt (écrit par Frogtend juste
-- avant de lancer Cheat Engine), puis l'efface. Sans ce fichier, il ne fait rien.
-- Fonctions de Cheat Engine (celua.txt) : getCheatEngineDir, fileExists, openProcess, loadTable, createTimer.
local fichier = getCheatEngineDir() .. 'frogtend-lancement.txt'
if fileExists(fichier) then
  local pid, tableCT
  for ligne in io.lines(fichier) do
    local cle, valeur = ligne:match('^(%w+)=(.*)$')
    if cle == 'pid' then
      pid = tonumber(valeur)
    elseif cle == 'table' and valeur ~= '' then
      tableCT = valeur
    end
  end
  os.remove(fichier)
  if pid then
    -- Un court délai : la fenêtre principale de Cheat Engine doit être prête.
    createTimer(800, function()
      openProcess(pid)
      if tableCT and fileExists(tableCT) then
        loadTable(tableCT)
      end
    end)
  end
end
