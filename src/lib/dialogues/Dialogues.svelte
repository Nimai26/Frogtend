<script lang="ts">
  // Affiche la pile des fenêtres maison et les toasts. À poser une seule fois, dans la mise en page.
  import { annulerDessus, fermerToast, pile, type Dialogue } from './fenetres.svelte';
  import { retenirFocus } from './focus';
  import { filtrerOptions, SEUIL_FILTRE } from './filtre';

  // Texte en cours de saisie de la fenêtre « Demander », par identifiant de fenêtre.
  let saisies = $state<Record<number, string>>({});
  /** Le texte cherché dans une longue liste de choix (par fenêtre). */
  let filtres = $state<Record<number, string>>({});

  const dessus = $derived(pile.dialogues.at(-1));

  // « Appuie sur une touche » : pourquoi la dernière touche n'est pas possible, par fenêtre.
  let refusTouche = $state<Record<number, string>>({});

  // Fenêtre « touche » au-dessus : le clavier est à elle seule (phase de capture : Échap, Entrée, Tab n'agissent
  // plus ailleurs). La touche est rendue à son RELÂCHEMENT. Une fausse touche de la manette (sans `code`) est ignorée.
  $effect(() => {
    const d = dessus;
    if (!d || d.sorte !== 'touche') return;
    let attendue: { code: string; key: string } | null = null;
    const bas = (e: KeyboardEvent) => {
      if (!e.code) return;
      e.preventDefault();
      e.stopImmediatePropagation();
      if (e.repeat) return;
      // Deux touches à la fois (AltGr = Ctrl + Alt droite sous Windows) : on recommence.
      if (attendue) {
        attendue = null;
        refusTouche[d.id] = 'Une seule touche à la fois : appuie de nouveau.';
        return;
      }
      const refus = d.verifier({ code: e.code, key: e.key });
      if (refus) refusTouche[d.id] = refus;
      else {
        delete refusTouche[d.id];
        attendue = { code: e.code, key: e.key };
      }
    };
    const haut = (e: KeyboardEvent) => {
      if (!e.code) return;
      e.preventDefault();
      e.stopImmediatePropagation();
      if (attendue && e.code === attendue.code) d.fermer(attendue);
    };
    // La fenêtre perd la main entre l'appui et le relâchement : cette touche ne compte plus.
    const oubli = () => (attendue = null);
    window.addEventListener('keydown', bas, true);
    window.addEventListener('keyup', haut, true);
    window.addEventListener('blur', oubli);
    return () => {
      window.removeEventListener('keydown', bas, true);
      window.removeEventListener('keyup', haut, true);
      window.removeEventListener('blur', oubli);
    };
  });

  /** Les cases cochées d'une fenêtre « Cocher » (par fenêtre ; au départ, celles qu'elle a reçues). */
  let cases = $state<Record<number, boolean[]>>({});
  const cochees = (d: Extract<Dialogue, { sorte: 'cocher' }>) => cases[d.id] ?? d.coches;
  const valeursCochees = (d: Extract<Dialogue, { sorte: 'cocher' }>) => d.options.filter((_, i) => cochees(d)[i]).map((o) => o.valeur);
  function basculer(d: Extract<Dialogue, { sorte: 'cocher' }>, i: number) {
    const l = [...cochees(d)];
    l[i] = !l[i];
    cases[d.id] = l;
  }
  /** « Tout cocher » / « Tout décocher ». */
  function tout(d: Extract<Dialogue, { sorte: 'cocher' }>, oui: boolean) {
    cases[d.id] = d.options.map(() => oui);
  }

  function valider(d: Dialogue) {
    if (d.sorte === 'cocher') {
      const v = valeursCochees(d);
      if (v.length && !d.refus?.(v)) d.fermer(v);
    } else if (d.sorte === 'confirmer') d.fermer(true);
    else if (d.sorte === 'demander') d.fermer(saisies[d.id] ?? d.valeur);
    else if (d.sorte === 'informer') d.fermer();
  }

  // Seule la fenêtre du dessus écoute le clavier.
  function surTouche(e: KeyboardEvent) {
    if (!dessus) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      annulerDessus();
    } else if (e.key === 'Enter' && dessus.sorte !== 'choisir') {
      const cible = e.target as HTMLElement;
      if (cible.tagName === 'TEXTAREA' || cible.tagName === 'BUTTON') return;
      e.preventDefault();
      valider(dessus);
    }
  }
</script>

<svelte:window onkeydown={surTouche} />

{#each pile.dialogues as d (d.id)}
  <div class="voile" role="presentation" onclick={(e) => e.target === e.currentTarget && d === dessus && annulerDessus()}>
    <div
      class="boite"
      role={d.sorte === 'confirmer' || d.sorte === 'informer' || d.sorte === 'conflit' || d.sorte === 'cestToi' ? 'alertdialog' : 'dialog'}
      aria-modal="true"
      aria-labelledby="dlg-titre-{d.id}"
      inert={d !== dessus}
      use:retenirFocus
    >
      <h2 id="dlg-titre-{d.id}">{d.titre}</h2>
      {#if d.message}<p class="message">{d.message}</p>{/if}

      {#if d.sorte === 'demander'}
        {#if d.multiligne}
          <textarea rows="5" value={d.valeur} oninput={(e) => (saisies[d.id] = e.currentTarget.value)}></textarea>
        {:else}
          <input
            type={d.masque ? 'password' : 'text'}
            autocomplete="off"
            spellcheck="false"
            value={d.valeur}
            oninput={(e) => (saisies[d.id] = e.currentTarget.value)}
          />
        {/if}
      {/if}

      {#if d.sorte === 'touche'}
        <!-- Le focus va ICI (jamais sur « Annuler » : Entrée y serait prise pour annuler, alors qu'elle est lue). -->
        <p class="attente" tabindex="-1" data-premier aria-live="polite">{refusTouche[d.id] ?? '⌨ En attente d’une touche…'}</p>
      {/if}

      {#if d.sorte === 'choisir'}
        {#if d.options.length === 0}
          <p class="muted">Aucun choix possible pour l'instant.</p>
        {:else}
          {@const montrees = d.options.length > SEUIL_FILTRE ? filtrerOptions(d.options, filtres[d.id] ?? '') : d.options}
          {#if d.options.length > SEUIL_FILTRE}
            <input
              class="filtre"
              type="search"
              placeholder="🔎 Chercher…"
              aria-label="Chercher dans la liste"
              value={filtres[d.id] ?? ''}
              oninput={(e) => (filtres[d.id] = e.currentTarget.value)}
            />
            <p class="muted">{montrees.length ? `${montrees.length} sur ${d.options.length}` : 'Aucun choix ne correspond : essaie un autre mot.'}</p>
          {/if}
          <ul class="choix">
            {#each montrees as o (o)}
              <li>
                <button class="btn option" onclick={() => d.fermer(o.valeur)}>
                  <span>{o.libelle}</span>
                  {#if o.detail}<span class="muted">{o.detail}</span>{/if}
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      {/if}

      {#if d.sorte === 'conflit'}
        <!-- Lot S5 : deux colonnes, jamais un chemin ni une empreinte ; le badge seulement quand c'est sûr. -->
        <div class="conflit">
          {#each d.colonnes as c, i (i)}
            <section class="colonne" aria-label={c.titre}>
              <h3>{c.titre}</h3>
              {#if c.recente}<p class="recente">⭐ la plus récente</p>{/if}
              {#each c.lignes as l, j (j)}<p>{l}</p>{/each}
            </section>
          {/each}
        </div>
      {/if}

      {#if d.sorte === 'cocher'}
        {#if d.toutCocher}
          <div class="actions tout">
            <button class="btn" onclick={() => tout(d, true)}>Tout cocher</button>
            <button class="btn" onclick={() => tout(d, false)}>Tout décocher</button>
          </div>
        {/if}
        <ul class="choix" role="group" aria-labelledby="dlg-titre-{d.id}">
          {#each d.options as o, i (o)}
            <li>
              <!-- Le focus d'office reste sur la première case (pas sur « Tout cocher », au-dessus). -->
              <button class="btn option" role="checkbox" aria-checked={cochees(d)[i]} data-premier={d.toutCocher && i === 0 ? '' : undefined} onclick={() => basculer(d, i)}>
                <span><span aria-hidden="true">{cochees(d)[i] ? '☑' : '☐'}</span> {o.libelle}</span>
                {#if o.detail}<span class="muted">{o.detail}</span>{/if}
              </button>
            </li>
          {/each}
        </ul>
        {#if d.resume}<p class="muted" aria-live="polite">{d.resume(valeursCochees(d))}</p>{/if}
        {#if d.refus?.(valeursCochees(d))}<p class="refus" role="alert">{d.refus(valeursCochees(d))}</p>{/if}
      {/if}

      <div class="actions">
        {#if d.sorte === 'informer'}
          <button class="btn primary" data-valider onclick={() => d.fermer()}>OK</button>
        {:else if d.sorte === 'conflit'}
          <button class="btn primary" data-premier onclick={() => d.fermer('ici')}>Garder celle de ce PC</button>
          <button class="btn" onclick={() => d.fermer('firehouse')}>Reprendre celle de Firehouse</button>
          <button class="btn" onclick={() => d.fermer('plus_tard')}>Décider plus tard</button>
        {:else if d.sorte === 'cestToi'}
          <!-- Lot S7 : « Décider plus tard » a le focus d'office (jamais une réponse pour quelqu'un d'autre par deux appuis). -->
          <button class="btn primary" onclick={() => d.fermer('oui')}>Oui, c’est moi</button>
          <button class="btn" onclick={() => d.fermer('non')}>Non</button>
          <button class="btn" data-premier onclick={() => d.fermer('plus_tard')}>Décider plus tard</button>
        {:else}
          <button class="btn" data-premier={d.sorte === 'confirmer' && d.danger ? '' : undefined} onclick={annulerDessus}>Annuler</button>
          {#if d.sorte === 'confirmer'}
            <button class="btn {d.danger ? 'danger' : 'primary'}" data-valider onclick={() => d.fermer(true)}>
              {d.libelleValider}
            </button>
          {:else if d.sorte === 'demander'}
            <button class="btn primary" data-valider onclick={() => valider(d)}>{d.libelleValider}</button>
          {:else if d.sorte === 'cocher'}
            <button class="btn primary" disabled={valeursCochees(d).length === 0 || !!d.refus?.(valeursCochees(d))} onclick={() => valider(d)}>{d.libelleValider}</button>
          {:else if d.sorte === 'touche'}
            <button class="btn" onclick={() => d.fermer('rien')}>⬜ Aucune touche</button>
            <button class="btn" onclick={() => d.fermer('liste')}>📋 Dans la liste…</button>
          {/if}
        {/if}
      </div>
    </div>
  </div>
{/each}

<div class="toasts" aria-live="polite">
  {#each pile.toasts as t (t.id)}
    {#if t.progres}
      <!-- Un travail en cours : jamais fermé au clic ; seule l'étape est annoncée, les chiffres se lisent à la demande. -->
      <div class="toast progres {t.type}">
        <p class="message">{t.message}</p>
        {#if t.progres.etape}<p class="etape">{t.progres.etape}</p>{/if}
        <div class="bar" role="progressbar" aria-label={t.progres.etape || t.message} aria-valuemin="0" aria-valuemax="100" aria-valuenow={t.progres.pourcent}>
          <span style="width: {t.progres.pourcent}%"></span>
        </div>
        <p class="chiffres muted" aria-live="off">{t.progres.chiffres}</p>
      </div>
    {:else}
      <button class="toast {t.type}" onclick={() => fermerToast(t.id)}>{t.message}</button>
    {/if}
  {/each}
</div>

<style>
  .voile {
    position: fixed;
    inset: 0;
    background: var(--voile);
    display: grid;
    place-items: center;
    z-index: 100;
  }
  .boite {
    background: var(--panel);
    border: 1px solid color-mix(in srgb, var(--ink) 16%, transparent);
    border-radius: calc(14 * var(--u));
    padding: calc(18 * var(--u));
    width: min(calc(440 * var(--u)), 94vw);
    max-height: 86vh;
    overflow: auto;
    box-shadow: 0 calc(20 * var(--u)) calc(60 * var(--u)) var(--ombre-noire);
    backdrop-filter: blur(16px);
  }
  h2 {
    margin: 0 0 calc(10 * var(--u));
    font-size: calc(16 * var(--u));
  }
  .message {
    margin: 0 0 calc(14 * var(--u));
    white-space: pre-line;
    /* Un long chemin de dossier, sans espaces, passe à la ligne au lieu de sortir de la fenêtre. */
    overflow-wrap: anywhere;
  }
  .attente {
    margin: 0 0 calc(14 * var(--u));
    padding: calc(14 * var(--u));
    text-align: center;
    border: 2px dashed var(--accent);
    border-radius: calc(10 * var(--u));
    font-size: calc(15 * var(--u));
  }
  textarea,
  input {
    margin-bottom: calc(14 * var(--u));
  }
  .filtre {
    width: 100%;
    margin-bottom: calc(6 * var(--u));
  }
  .choix {
    list-style: none;
    margin: 0 0 calc(14 * var(--u));
    padding: 0;
    display: grid;
    gap: calc(6 * var(--u));
  }
  .option {
    width: 100%;
    justify-content: space-between;
    text-align: left;
  }
  .conflit {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(calc(160 * var(--u)), 1fr));
    gap: calc(10 * var(--u));
    margin: 0 0 calc(14 * var(--u));
  }
  .colonne {
    border: 1px solid var(--line);
    border-radius: calc(10 * var(--u));
    background: var(--bg2);
    padding: calc(10 * var(--u)) calc(12 * var(--u));
    display: grid;
    gap: calc(4 * var(--u));
    align-content: start;
  }
  .colonne h3 {
    margin: 0;
    font-size: calc(14 * var(--u));
  }
  .colonne p {
    margin: 0;
  }
  .recente {
    font-weight: 600;
    color: var(--ok-texte);
  }
  .refus {
    color: var(--err);
    margin: 0 0 calc(10 * var(--u));
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: calc(8 * var(--u));
    flex-wrap: wrap;
  }
  .actions.tout {
    justify-content: flex-start;
  }
  .toasts {
    position: fixed;
    left: 50%;
    bottom: calc(20 * var(--u));
    transform: translateX(-50%);
    display: grid;
    gap: calc(8 * var(--u));
    z-index: 200;
    width: min(calc(520 * var(--u)), 94vw);
  }
  .toast {
    font: inherit;
    text-align: left;
    background: var(--panel);
    color: var(--ink);
    border: 1px solid var(--line);
    border-left: calc(3 * var(--u)) solid var(--dim);
    border-radius: calc(10 * var(--u));
    padding: calc(10 * var(--u)) calc(14 * var(--u));
    min-height: var(--cible-min);
    box-shadow: 0 calc(8 * var(--u)) calc(24 * var(--u)) var(--ombre-noire);
    backdrop-filter: blur(16px);
    cursor: pointer;
    overflow-wrap: anywhere;
  }
  .toast.progres {
    cursor: default;
    display: grid;
    gap: calc(6 * var(--u));
  }
  .toast.progres p {
    margin: 0;
    overflow-wrap: anywhere;
  }
  .toast.progres .etape {
    font-weight: 600;
  }
  .toast.ok { border-left-color: var(--ok); }
  .toast.alerte { border-left-color: var(--warn); }
  .toast.erreur { border-left-color: var(--err); }
</style>
