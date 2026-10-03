<script lang="ts">
  // ❓ Aide : les réponses aux questions qu'on se pose en s'en servant, pour toute personne qui utilise Frogtend
  // (pas seulement Seb). Les libellés cités sont ceux de l'interface : les garder à jour avec elle.
  import { explicationGalaxy, SOURCES } from '$lib/import/sources';

  const SOMMAIRE = [
    ['importer', '📥 Importer'],
    ['galaxy', '🟣 GOG Galaxy'],
    ['versions', '📀 Versions'],
    ['succes', '🏆 Succès'],
    ['contenus', '📦 Mises à jour et DLC'],
    ['offerts', '🎁 Jeux offerts'],
    ['jouer', '🕹 En jeu'],
    ['taodbox', '🛋 Taodbox'],
    ['rangement', '📁 Rangement'],
  ];
</script>

<div class="aide">
  <h1>❓ Aide</h1>

  <nav class="sommaire" aria-label="Sommaire">
    {#each SOMMAIRE as [ancre, libelle] (ancre)}<a href="#{ancre}">{libelle}</a>{/each}
  </nav>

  <section class="cx-block" id="importer">
    <h2>📥 Importer des jeux</h2>
    <p>Le menu « 📥 Importer » reprend la liste de LaunchBox. Chaque boutique a sa voie :</p>
    <ul>
      {#each SOURCES as s (s.id)}
        <li><a href="/importer?source={s.id}">{s.icone} {s.libelle}</a> — {s.resume}{s.voie === 'bientot' ? ' (prochain lot)' : ''}</li>
      {/each}
    </ul>
    <p>Les jeux importés des boutiques rejoignent ta ludothèque, rubrique Windows ; les filtres « Boutique » et « Installés » les retrouvent. Ils restent dans ton Frogtend et ne remontent jamais dans Firehouse.</p>
  </section>

  <section class="cx-block" id="galaxy">
    <h2>🟣 GOG Galaxy : toutes tes boutiques d’un coup</h2>
    {#each explicationGalaxy() as p (p)}<p>{p}</p>{/each}
    <p>
      Pourquoi passer par Galaxy ? Epic, Amazon, EA, Ubisoft et Xbox n’offrent pas d’API ouverte pour lire ta liste de
      jeux. Galaxy a des accords (« intégrations ») avec elles : c’est la voie la plus sûre, et ton mot de passe ne passe
      jamais par Frogtend. Steam, lui, a une API : tu peux importer par ton compte Steam OU par Galaxy.
    </p>
    <p>Réglages : <a href="/reglages?rubrique=galaxy">⚙ Options ▸ Comptes ▸ GOG Galaxy</a>.</p>
  </section>

  <section class="cx-block" id="versions">
    <h2>📀 Plusieurs versions d’un même jeu</h2>
    <p>
      Une seule fiche par jeu et par console, même si tu as plusieurs ROM (France, Europe, USA…). Frogtend les range
      dans cet ordre : <strong>FR, puis EU, puis US/EN, puis les autres</strong>.
    </p>
    <ul>
      <li>Au moment de jouer, il te demande laquelle, pour cette partie seulement.</li>
      <li>Celle d’habitude (⭐) se choisit dans <strong>⚙ Gérer le jeu ▸ 📀 Version</strong>.</li>
      <li>Les jeux sur plusieurs disques restent ensemble : le disque 2 n’est pas une autre version.</li>
    </ul>
  </section>

  <section class="cx-block" id="succes">
    <h2>🏆 Les succès</h2>
    <ul>
      <li><strong>Steam</strong> : les succès de tes jeux Steam, avec ton compte (<a href="/reglages?rubrique=steam">⚙ Options ▸ Comptes ▸ Steam</a>).</li>
      <li><strong>GOG Galaxy</strong> : les succès que Galaxy connaît (GOG et les boutiques reliées).</li>
      <li>
        <strong>RetroAchievements</strong> pour les jeux émulés : Frogtend calcule l’empreinte de TA version du jeu et
        dit si elle est reconnue. Si elle ne l’est pas, il le dit aussi, pour que tu demandes une version compatible.
        Règle ton compte dans <a href="/reglages?rubrique=retroachievements">⚙ Options ▸ Comptes ▸ 🏆 RetroAchievements</a>,
        puis <strong>🎮 Connecter mes émulateurs</strong> : RetroArch, PCSX2 et DuckStation gagnent alors les succès
        pendant que tu joues.
      </li>
      <li>La première fois, un gros jeu (Wii, GameCube) peut prendre ~30 secondes à vérifier ; ensuite c’est immédiat.</li>
    </ul>
  </section>

  <section class="cx-block" id="contenus">
    <h2>📦 Mises à jour et DLC</h2>
    <p>
      Dans le panneau d’un jeu installé, <strong>📦 Contenus additionnels</strong> montre ce que Frogtend a trouvé sur ton
      disque pour ce jeu. <strong>Rien ne s’installe sans ton oui</strong> : tu coches, tu confirmes.
    </p>
    <ul>
      <li><strong>PS3 (RPCS3)</strong> : les .pkg et leurs licences (.rap) s’installent par RPCS3, dans le compte de ton profil.</li>
      <li><strong>Switch (Eden)</strong> : rien n’est copié ; Frogtend ajoute le dossier à la liste qu’Eden lit. Ferme Eden avant.</li>
      <li><strong>Wii U (Cemu)</strong> : un .wua contient déjà ses mises à jour et DLC ; Frogtend les montre, il n’y a rien à faire.</li>
      <li>À l’import, une mise à jour ou un DLC n’est jamais pris pour un jeu.</li>
    </ul>
  </section>

  <section class="cx-block" id="offerts">
    <h2>🎁 Les jeux offerts</h2>
    <p>
      Page <a href="/boutiques">Boutiques</a> : Epic (chaque semaine), GOG, Prime Gaming et PlayStation Plus (les jeux du
      mois). Connecte-toi une fois à chaque boutique (🔑) ; Frogtend ne voit jamais ton mot de passe. Coche « tout seul »
      pour qu’il les récupère à l’ouverture de ton profil. Il ne peut rien acheter : il clique seulement sur « Obtenir »
      ou « Ajouter à la bibliothèque ».
    </p>
  </section>

  <section class="cx-block" id="jouer">
    <h2>🕹 Pendant une partie</h2>
    <ul>
      <li>
        La touche <strong>Pause/Attn</strong> (réglable dans <a href="/reglages?rubrique=emulateurs">⚙ Options ▸ Émulateurs</a>)
        ouvre le menu de Frogtend par-dessus le jeu : reprendre, sauvegarde rapide, manuel, quitter…
      </li>
      <li><strong>⚙ Gérer le jeu</strong> : changer d’émulateur pour ce jeu seulement, les commandes, ce qui lance le jeu.</li>
      <li>Les triches et les mods se trouvent par jeu ; avant de modder, Frogtend propose une copie du jeu.</li>
      <li>
        Un jeu à disque rangé dans un .zip ou un .7z (PS3, PS2, GameCube…) : l’émulateur ne le lit pas tel quel. Le
        panneau du jeu propose <strong>🗜 Décompresser pour jouer</strong>, avec la taille et la durée annoncées ;
        l’archive est gardée.
      </li>
    </ul>
  </section>

  <section class="cx-block" id="taodbox">
    <h2>🛋 Taodbox : jouer depuis le canapé</h2>
    <p>
      Le mode plein écran, tout à la manette (croix, A, B). Ouvre-le par le menu, ou lance Frogtend avec
      <code>--taodbox</code> : c’est ainsi qu’on l’ajoute comme application dans Sunshine, pour jouer sur la télé avec
      Moonlight. Jouer sur l’écran du PC reste le mode normal.
    </p>
  </section>

  <section class="cx-block" id="rangement">
    <h2>📁 Où Frogtend range les choses</h2>
    <p>Frogtend n’écrit que dans le dossier d’un jeu et dans les dossiers que tu as choisis (<a href="/reglages?rubrique=emplacements">⚙ Options ▸ Emplacements</a>) :</p>
    <ul>
      <li>les images, mods et triches d’un jeu : dans le sous-dossier <code>Frogtend</code> de ce jeu (pour l’afficher même hors ligne) ;</li>
      <li>🧰 <strong>Outils</strong> : Cheat Engine, trainers, gestionnaires de mods ;</li>
      <li>💾 <strong>Abris de parties</strong> : la copie de tes parties, faite avant de retirer ou de réinstaller un jeu ;</li>
      <li>les jeux des boutiques PC (Steam, Epic…) restent dans les dossiers de leur boutique.</li>
    </ul>
    <p>
      Un jeu importé de ton disque n’est jamais effacé : « Retirer de la ludothèque » l’enlève de la liste, ses fichiers
      restent. Tes parties sont copiées à l’abri avant toute désinstallation, et tu peux le faire toi-même :
      <strong>⚙ Gérer le jeu ▸ 💾 Mettre mes parties à l’abri</strong>.
    </p>
  </section>
</div>

<style>
  .aide {
    display: grid;
    gap: calc(14 * var(--u));
    padding: calc(20 * var(--u));
    max-width: calc(900 * var(--u));
  }
  h1,
  h2,
  p {
    margin: 0;
  }
  .cx-block {
    display: grid;
    gap: calc(10 * var(--u));
  }
  .sommaire {
    display: flex;
    flex-wrap: wrap;
    gap: calc(6 * var(--u)) calc(14 * var(--u));
  }
  ul {
    margin: 0;
    display: grid;
    gap: calc(4 * var(--u));
  }
</style>
