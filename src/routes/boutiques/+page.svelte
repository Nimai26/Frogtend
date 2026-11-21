<script lang="ts">
  // « Mes boutiques » (lot 9) : les jeux possédés sur les comptes de la personne (Steam d'abord). Ils restent dans son
  // Frogtend et ne remontent jamais dans Firehouse. Jouer et installer passent par la boutique elle-même.
  import { onMount } from 'svelte';
  import { adresseImageBoutique, adresseImageParUrl, duree, type JeuBoutique } from '$lib/api';
  import { galaxy, importerGalaxy, lireGalaxy, nomPlateforme, ouvrirDansGalaxy } from '$lib/boutiques/galaxy.svelte';
  import { connexion, connexionEpic, verifierEpic, connexionPlaystation, gratuits, lireGratuits, obtenir, obtenirTout, recolterPsPlus, recuperer } from '$lib/boutiques/gratuits.svelte';
  import { etat, reglerProfil } from '$lib/etat.svelte';
  import { connecteASteam, connexionSteam, connexionSteamExpiree, importerSteam, importSteamEnCours, libelleCompteSecours, lireSteam, mettreAJourSteam, oublierSteam, ouvrirDansSteam, steam } from '$lib/boutiques/steam.svelte';
  import { boutiquesPc, lireEtatsPc, relireMaintenant } from '$lib/boutiques/boutiques-pc.svelte';
  import { ligneEtat } from '$lib/boutiques/jeu-boutique';

  let filtre = $state('');
  let seulementInstalles = $state(false);
  onMount(() => {
    lireSteam();
    lireGalaxy();
    lireGratuits();
    verifierEpic();
    lireEtatsPc();
  });

  // GOG Galaxy : filtre par boutique d'origine. Les jeux Steam y sont masqués par défaut si Steam est importé à part.
  let boutiqueGalaxy = $state<string>('tout');
  const plateformesGalaxy = $derived([...new Set(galaxy.jeux.map((j) => j.plateforme ?? ''))].sort());
  const affichesGalaxy = $derived(
    galaxy.jeux.filter(
      (j: JeuBoutique) =>
        (boutiqueGalaxy === 'tout' ? !(steam.jeux.length && j.plateforme === 'steam') : j.plateforme === boutiqueGalaxy) &&
        (!seulementInstalles || j.installe) &&
        j.nom.toLowerCase().includes(filtre.trim().toLowerCase()),
    ),
  );

  // La dernière erreur d'import Steam, seulement celle du profil ouvert.
  const erreurSteam = $derived(steam.erreurProfil === (etat.profilOuvert?.id ?? null) ? steam.erreur : null);

  const affiches = $derived(
    steam.jeux.filter((j) => (!seulementInstalles || j.installe) && j.nom.toLowerCase().includes(filtre.trim().toLowerCase())),
  );
</script>

<div class="boutiques">
  <header>
    <h1>🛒 Mes boutiques</h1>
    <p class="muted">Les jeux installés de tes boutiques (Steam, Epic, GOG, Ubisoft, Amazon, Xbox, Battle.net) entrent seuls dans ta ludothèque. Ils restent dans ton Frogtend.</p>
  </header>

  <section class="cx-block">
    <div class="tete">
      <h2>🎁 Jeux offerts</h2>
      <span class="muted">Epic Games, cette semaine</span>
      <span class="espace"></span>
      {#if etat.profil.gratuits.connectes.epic}
        <span class="tag ok">✅ Connecté à Epic</span>
        <button class="btn petit" onclick={connexionEpic}>Changer de compte</button>
      {:else}
        <button class="btn" onclick={connexionEpic}>🔑 Se connecter à Epic</button>
      {/if}
      <button class="btn primary" onclick={() => obtenirTout()} disabled={gratuits.enCours !== null}>🎁 Tout obtenir</button>
    </div>
    <label class="auto">
      <input
        type="checkbox"
        checked={etat.profil.gratuits.auto}
        onchange={(e) => reglerProfil('gratuits.auto', e.currentTarget.checked)}
      />
      Les obtenir tout seul (une fois par jour, à l’ouverture de mon profil). Si Epic demande une vérification, sa page s’ouvre.
    </label>
    {#if gratuits.liste.length}
      <div class="grille offerts">
        {#each gratuits.liste as j (j.slug)}
          <div class="carte">
            <span class="image">
              <span class="remplacement">{j.titre}</span>
              {#if j.image}<img src={adresseImageParUrl(j.image)} alt="" loading="lazy" onerror={(e) => ((e.currentTarget as HTMLElement).hidden = true)} />{/if}
            </span>
            <span class="nom">{j.titre}</span>
            <span class="muted">jusqu’au {new Date(j.fin).toLocaleDateString('fr-FR', { day: 'numeric', month: 'long' })}</span>
            {#if j.obtenu}
              <span class="tag ok">✅ Dans ta bibliothèque</span>
            {:else}
              <button class="btn petit primary" onclick={() => obtenir(j)} disabled={gratuits.enCours !== null}>
                {gratuits.enCours === j.slug ? 'En cours…' : '🎁 Obtenir'}
              </button>
            {/if}
          </div>
        {/each}
      </div>
    {:else}
      <p class="muted">Aucun jeu offert lu pour l’instant (connexion à Internet ?).</p>
    {/if}

    <div class="tete psplus">
      <h3>🎮 PlayStation Plus</h3>
      <span class="muted">les jeux du mois, ajoutés à ton compte PlayStation (pas à la ludothèque)</span>
      <span class="espace"></span>
      {#if etat.profil.gratuits.connectes.psplus}
        <span class="tag ok">✅ Connecté à PlayStation</span>
        <button class="btn petit" onclick={connexionPlaystation}>Changer de compte</button>
      {:else}
        <button class="btn" onclick={connexionPlaystation}>🔑 Se connecter à PlayStation</button>
      {/if}
      <button class="btn primary" onclick={() => recolterPsPlus()} disabled={gratuits.psplusEnCours}>
        {gratuits.psplusEnCours ? 'En cours… (quelques minutes)' : '🎮 Ajouter les jeux du mois'}
      </button>
    </div>
    <label class="auto">
      <input
        type="checkbox"
        checked={etat.profil.gratuits.psplus}
        onchange={(e) => reglerProfil('gratuits.psplus', e.currentTarget.checked)}
      />
      J’ai PS Plus : les ajouter tout seul (une fois par semaine, à l’ouverture de mon profil). Frogtend ne clique que sur « Ajouter à la bibliothèque » : il ne peut rien acheter.
    </label>

    {#each [{ b: 'gog' as const, nom: '🟣 GOG', detail: 'le jeu offert du moment, ajouté à ton compte GOG', auto: 'Les récupérer tout seul (une fois par jour). Note : GOG t’inscrit à sa lettre d’information quand tu prends un jeu offert ; Frogtend ne change pas tes réglages de compte.' }, { b: 'prime' as const, nom: '📦 Prime Gaming', detail: 'les jeux offerts aux membres Amazon Prime', auto: 'Les récupérer tout seul (une fois par semaine). Les offres d’autres boutiques (code GOG, compte Epic à relier) s’ouvrent pour que tu finisses.' }] as o (o.b)}
      <div class="tete psplus">
        <h3>{o.nom}</h3>
        <span class="muted">{o.detail}</span>
        <span class="espace"></span>
        {#if etat.profil.gratuits.connectes[o.b]}
          <span class="tag ok">✅ Connecté</span>
          <button class="btn petit" onclick={() => connexion(o.b)}>Changer de compte</button>
        {:else}
          <button class="btn" onclick={() => connexion(o.b)}>🔑 Se connecter</button>
        {/if}
        <button class="btn primary" onclick={() => recuperer(o.b)} disabled={gratuits.boutique !== null}>
          {gratuits.boutique === o.b ? 'En cours…' : '🎁 Récupérer'}
        </button>
      </div>
      <label class="auto">
        <input type="checkbox" checked={etat.profil.gratuits[o.b]} onchange={(e) => reglerProfil(`gratuits.${o.b}`, e.currentTarget.checked)} />
        {o.auto}
      </label>
    {/each}

    <!-- Seb 21/11 : le compte Steam AVEC les autres comptes (Epic, PS Plus, GOG, Prime), pas sous « Mes jeux installés ». -->
    <!-- Lot C3 : oublier le compte ET le déconnecter du navigateur du profil (pour en changer) ; les jeux restent. Le
         bouton garde le focus pendant l'oubli (aria-disabled ; le gardien est dans oublierSteam). -->
    {#snippet boutonOublierSteam()}
      <button class="btn petit" aria-disabled={steam.oubliEnCours} onclick={oublierSteam}>{steam.oubliEnCours ? 'Déconnexion…' : 'Oublier ce compte'}</button>
    {/snippet}
    <!-- Lot C1 : la connexion à Steam par sa page officielle (clavier et souris : pas dans Taodbox). -->
    <div class="tete psplus">
      <h3>🎮 Mon compte Steam</h3>
      <span class="muted">sur sa page officielle ; ton mot de passe reste chez Steam</span>
      <span class="espace"></span>
      {#if connecteASteam(steam.etat) && connexionSteamExpiree()}
        <!-- Lot C2 : la connexion du navigateur du profil a expiré ; les jeux déjà importés restent. -->
        <span class="tag warn">🔑 Connexion expirée</span>
        <button class="btn primary" onclick={connexionSteam} disabled={steam.connexionEnCours || steam.oubliEnCours}>{steam.connexionEnCours ? 'Ouverture…' : '🔑 Me reconnecter à Steam'}</button>
        {@render boutonOublierSteam()}
      {:else if connecteASteam(steam.etat)}
        <span class="tag ok">✅ Connecté à Steam</span>
        <!-- Lot C2 : les jeux possédés du compte, sans clé d'API (aussi relus à chaque ouverture de Frogtend). Le bouton
             garde le focus pendant la lecture (aria-disabled, et le gardien ignore le clic). -->
        <button class="btn primary" aria-disabled={importSteamEnCours()} onclick={() => !importSteamEnCours() && mettreAJourSteam('main')}>
          {importSteamEnCours() ? 'Lecture…' : '🔄 Mettre à jour mes jeux Steam'}
        </button>
        <button class="btn petit" onclick={connexionSteam} disabled={steam.connexionEnCours || steam.oubliEnCours}>Changer de compte / se reconnecter</button>
        {@render boutonOublierSteam()}
      {:else}
        <button class="btn" onclick={connexionSteam} disabled={steam.connexionEnCours || steam.oubliEnCours}>{steam.connexionEnCours ? 'Ouverture…' : '🔑 Se connecter à Steam'}</button>
      {/if}
    </div>
    {#if connecteASteam(steam.etat)}
      <!-- La dernière erreur d'import reste affichée ici (une erreur au démarrage ne fait pas de message). -->
      <p class="muted ligne-steam" class:alerte={!!erreurSteam} role="status">
        {erreurSteam ??
          (steam.etat?.nb_jeux ? `${steam.etat.nb_jeux} jeu(x) de ton compte Steam dans ta ludothèque, relus à chaque ouverture de Frogtend.` : 'Tes jeux Steam sont relus à chaque ouverture de Frogtend.')}
      </p>
    {/if}
  </section>

  <section class="cx-block">
    <div class="tete">
      <h2>🎮 Mes jeux installés</h2>
      <span class="muted">ils entrent seuls dans ta ludothèque, relus à chaque ouverture de ton profil</span>
      <span class="espace"></span>
      <button class="btn primary" onclick={relireMaintenant} disabled={boutiquesPc.enCours}>{boutiquesPc.enCours ? 'Lecture…' : '🔄 Relire maintenant'}</button>
    </div>
    {#if !etat.profil.boutiquesPc.auto}
      <p class="muted">⚠ L’ajout tout seul est coupé pour ton profil (⚙ Options ▸ Comptes ▸ Mes jeux installés).</p>
    {/if}
    <ul class="etats">
      {#each boutiquesPc.etats as e (e.boutique)}
        <li class:absent={!e.lu}>{ligneEtat(e)}</li>
      {:else}
        <li class="muted">{boutiquesPc.erreur ?? 'Lecture des boutiques de ce PC…'}</li>
      {/each}
    </ul>
  </section>

  <details class="cx-block secours">
    <summary>🛟 Secours : ma clé Steam et GOG Galaxy</summary>
    <p class="muted">Si la connexion à Steam ne marche pas, ta clé d’API Steam reste possible (⚙ Options ▸ Comptes ▸ Steam) ; GOG Galaxy montre aussi les jeux de tes comptes qu’il relie. Tes jeux installés, eux, entrent toujours seuls.</p>
    <section class="cx-block">
      <div class="tete">
        <h2>Steam</h2>
        <span class="muted">{libelleCompteSecours(steam.etat)}</span>
        <span class="espace"></span>
        <!-- Connecté par la page : le même import sans clé (jamais la clé demandée à un novice connecté). -->
        <button class="btn primary" onclick={importerSteam} disabled={importSteamEnCours()}>
          {importSteamEnCours() ? 'Import…' : steam.jeux.length ? '🔄 Mettre à jour' : '⬇ Importer mes jeux Steam'}
        </button>
      </div>

      {#if steam.jeux.length}
        <div class="outils">
          <input type="search" bind:value={filtre} placeholder="Chercher un jeu" aria-label="Chercher un jeu Steam" />
          <label><input type="checkbox" bind:checked={seulementInstalles} /> installés sur ce PC</label>
          <span class="muted">{affiches.length} / {steam.jeux.length}</span>
        </div>
        <div class="grille">
          {#each affiches as j (j.id)}
            <button
              class="carte"
              title={j.installe ? `▶ Jouer à « ${j.nom} » (Steam)` : `⬇ Installer « ${j.nom} » (Steam)`}
              onclick={() => ouvrirDansSteam(j, j.installe ? 'jouer' : 'installer')}
            >
              <span class="image">
                <span class="remplacement">{j.nom}</span>
                <img src={adresseImageBoutique('steam', j.id)} alt="" loading="lazy" onerror={(e) => ((e.currentTarget as HTMLElement).hidden = true)} />
              </span>
              <span class="nom">{j.nom}</span>
              <span class="muted">{j.installe ? '✅ installé · ▶ Jouer' : '⬇ Installer'}{j.minutes ? ` · ${duree(j.minutes * 60)}` : ''}</span>
            </button>
          {/each}
        </div>
      {:else if !importSteamEnCours()}
        <p class="muted">Aucun jeu Steam importé pour l’instant.</p>
      {/if}
    </section>

    <section class="cx-block">
      <div class="tete">
        <h2>GOG Galaxy</h2>
        <span class="muted">{galaxy.installe ? 'GOG et les boutiques que tu y as reliées' : '⚠ GOG Galaxy n’est pas trouvé sur ce PC'}</span>
        <span class="espace"></span>
        <button class="btn primary" onclick={importerGalaxy} disabled={galaxy.enCours || !galaxy.installe}>
          {galaxy.enCours ? 'Lecture…' : galaxy.jeux.length ? '🔄 Mettre à jour' : '⬇ Lire mes jeux GOG Galaxy'}
        </button>
      </div>
      {#if galaxy.jeux.length}
        <div class="outils">
          <input type="search" bind:value={filtre} placeholder="Chercher un jeu" aria-label="Chercher un jeu" />
          <select bind:value={boutiqueGalaxy} aria-label="Boutique">
            <option value="tout">Toutes les boutiques</option>
            {#each plateformesGalaxy as p (p)}<option value={p}>{nomPlateforme(p)}</option>{/each}
          </select>
          <label><input type="checkbox" bind:checked={seulementInstalles} /> installés sur ce PC</label>
          <span class="muted">{affichesGalaxy.length} / {galaxy.jeux.length}</span>
        </div>
        <div class="grille">
          {#each affichesGalaxy as j (j.id)}
            <button class="carte" title={`Ouvrir « ${j.nom} » dans GOG Galaxy`} onclick={() => ouvrirDansGalaxy(j)}>
              <span class="image">
                <span class="remplacement">{j.nom}</span>
                {#if j.image}<img src={adresseImageParUrl(j.image)} alt="" loading="lazy" onerror={(e) => ((e.currentTarget as HTMLElement).hidden = true)} />{/if}
              </span>
              <span class="nom">{j.nom}</span>
              <span class="muted">{nomPlateforme(j.plateforme ?? '')}{j.installe ? ' · ✅ installé' : ''}{j.minutes ? ` · ${duree(j.minutes * 60)}` : ''}</span>
            </button>
          {/each}
        </div>
      {/if}
    </section>
  </details>
</div>

<style>
  .etats {
    margin: 0;
    padding-left: calc(18 * var(--u));
    display: grid;
    gap: calc(4 * var(--u));
  }
  .etats .absent {
    color: var(--dim);
  }
  .ligne-steam {
    margin: 0;
  }
  .ligne-steam.alerte {
    color: var(--warn-texte);
  }
  .secours > summary {
    cursor: pointer;
    font-weight: 600;
  }
  .secours[open] {
    display: grid;
    gap: calc(14 * var(--u));
  }
  .boutiques {
    display: grid;
    gap: calc(14 * var(--u));
    padding: calc(20 * var(--u));
    max-width: calc(900 * var(--u));
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
    align-content: start;
    overflow: auto;
    height: 100%;
  }
  h1,
  h2 {
    margin: 0;
  }
  header p {
    margin: calc(4 * var(--u)) 0 0;
  }
  .tete,
  .outils {
    display: flex;
    align-items: center;
    gap: calc(10 * var(--u));
    flex-wrap: wrap;
  }
  .espace {
    flex: 1;
  }
  .outils {
    margin: calc(10 * var(--u)) 0;
  }
  .outils input[type='search'] {
    flex: 1;
    min-width: calc(180 * var(--u));
  }
  .boutiques {
    max-width: none;
  }
  .auto {
    display: flex;
    gap: calc(8 * var(--u));
    align-items: center;
    margin: calc(8 * var(--u)) 0;
  }
  .offerts .carte {
    cursor: default;
  }
  .psplus {
    margin-top: calc(14 * var(--u));
  }
  .psplus h3 {
    margin: 0;
  }
  .grille {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(calc(150 * var(--u)), 1fr));
    gap: calc(14 * var(--u));
  }
  .carte {
    display: grid;
    gap: calc(4 * var(--u));
    padding: calc(4 * var(--u));
    background: none;
    border: calc(2 * var(--u)) solid transparent;
    border-radius: calc(8 * var(--u));
    color: var(--ink);
    text-align: left;
    cursor: pointer;
  }
  .carte:hover,
  .carte:focus-visible {
    border-color: var(--accent);
  }
  /* Le titre est DERRIÈRE l'image : visible tant qu'elle n'est pas là (ou si Steam n'en a pas). */
  .image {
    position: relative;
    aspect-ratio: 2 / 3;
    border-radius: calc(6 * var(--u));
    overflow: hidden;
    background: color-mix(in srgb, var(--ink) 8%, transparent);
  }
  .image img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .remplacement {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    padding: calc(8 * var(--u));
    text-align: center;
    color: var(--dim);
  }
  .nom {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
