<script lang="ts">
  // La sauvegarde du profil : ce qu'elle contient, où elle va, quand elle se fait.
  import { onMount } from 'svelte';
  import { api, taille, type ConflitRestaure } from '$lib/api';
  import { etat, reglerPc, reglerProfil } from '$lib/etat.svelte';
  import { depuis } from '$lib/ludotheque/ludotheque.svelte';
  import { lireDerniereSauvegarde, placeTexte, proposerRemplacements, restaurer, sauvegarde, sauvegarder } from '$lib/sauvegarde.svelte';
  import { libelleSauvegarde } from '$lib/sauvegardeTaille';

  // Les parties restaurées encore en double : une proposition fermée sans répondre se retrouve ici.
  let enDouble = $state<ConflitRestaure[]>([]);
  const relireDoubles = async () => {
    enDouble = await api.restaurationConflits().catch(() => []);
  };
  // Lot S4 : la place chez Firehouse (tous profils et PC de la personne) ; inconnue hors ligne ou en mode simulé.
  let place = $state<{ utilise: number; quota: number } | null>(null);
  const relirePlace = async () => {
    place = etat.pc.firehouse.simule ? null : await api.sauvegardePlace().catch(() => null);
  };
  onMount(() => {
    lireDerniereSauvegarde();
    relireDoubles();
    relirePlace();
  });
  const d = $derived(sauvegarde.derniere);
  // Relue après chaque sauvegarde (elle a pu changer la place utilisée).
  let vue = '';
  $effect(() => {
    const date = sauvegarde.derniere?.date ?? '';
    if (date && date !== vue) {
      vue = date;
      relirePlace();
    }
  });
  const pourcent = $derived(place && place.quota > 0 ? Math.min(100, Math.round((place.utilise / place.quota) * 100)) : 0);
</script>

<div class="sauvegarde">
  <p class="muted">
    Tout ce qui ne se retélécharge pas : tes réglages, la liste de tes jeux, les parties et codes de triche de
    tous tes jeux (jeux PC comme émulateurs), et tes captures d’écran. Elle part chez Firehouse, dans ton dossier, par Internet : ça marche à
    la maison comme ailleurs. Firehouse garde les versions précédentes. Après un reformatage, tu retrouves tout.
  </p>

  <dl class="cx-kv">
    <dt>Dernière sauvegarde</dt>
    <dd>
      {#if d}
        {depuis(Number(d.date))}, depuis {d.pc} · {d.fichiers} fichier(s) de parties ({taille(d.taille)})
      {:else}
        jamais depuis ce PC
      {/if}
    </dd>
    {#if d?.sautes}
      <dt>À réessayer</dt>
      <dd>⚠ {d.sautes} fichier(s) n’ont pas pu partir la dernière fois : Frogtend réessaiera à la prochaine sauvegarde.</dd>
    {/if}
    {#if d?.sel_en_attente}
      <!-- Lot S4 : le sel du profil (sa « clé ») attend que Firehouse l'accepte ; jamais sa valeur. -->
      <dt>Clé du profil</dt>
      <dd class="muted">La clé de ton profil n’est pas encore chez Firehouse : Frogtend réessaiera à la prochaine sauvegarde.</dd>
    {/if}
    <dt>Place chez Firehouse</dt>
    <dd>
      {#if place}
        {placeTexte(place)} ({pourcent} %) — pour tous tes profils et tous tes PC
      {:else}
        inconnue pour l’instant (Firehouse n’a pas pu être lu)
      {/if}
    </dd>
  </dl>
  {#if place}
    <div class="jauge" role="meter" aria-label="Place utilisée chez Firehouse" aria-valuemin="0" aria-valuemax="100" aria-valuenow={pourcent}>
      <div class="rempli" style:width="{pourcent}%"></div>
    </div>
  {/if}

  <div class="cx-form-section">
    <label class="champ">
      <span>Sauvegarde automatique</span>
      <select value={etat.profil.sauvegarde.auto} onchange={(e) => reglerProfil('sauvegarde.auto', e.currentTarget.value)}>
        <option value="apres_partie">Après chaque partie (conseillé)</option>
        <option value="quotidienne">Une fois par jour</option>
        <option value="manuelle">Jamais : seulement quand je le demande</option>
      </select>
    </label>
    <!-- Lot S3 (Seb 23/11) : l'abri des parties des jeux PC sur CE PC, après chaque partie (allumé d'office). L'abri
         avant un retrait, une réinstallation ou un mod ne se coupe jamais. -->
    <label class="choix">
      <input
        type="checkbox"
        checked={etat.profil.parties?.abriApresPartie !== false}
        onchange={(e) => reglerProfil('parties.abriApresPartie', e.currentTarget.checked)}
      />
      <span>🛟 Mettre mes parties à l’abri sur ce PC après chaque partie (pour tous tes jeux PC)</span>
    </label>
    <!-- Lot S7 (Seb 24/11) : à l'ouverture du profil, les parties jouées HORS de Frogtend (Steam, Epic, un raccourci…)
         sont mises à l'abri elles aussi (allumé d'office, séparé de l'abri après une partie). -->
    <label class="choix">
      <input
        type="checkbox"
        checked={etat.profil.parties?.rattrapage !== false}
        onchange={(e) => reglerProfil('parties.rattrapage', e.currentTarget.checked)}
      />
      <span>🛟 Rattraper les parties jouées hors de Frogtend</span>
    </label>
  </div>

  <!-- Lot 7 (Seb 17/11) : les mises à jour et DLC Switch que Firehouse n'a pas, envoyés pour toute la famille. -->
  <div class="cx-form-section">
    <h3>📤 Partager avec la famille</h3>
    <label class="choix">
      <input
        type="checkbox"
        checked={!!etat.profil.envoisFirehouse?.auto}
        onchange={(e) => reglerProfil('envoisFirehouse.auto', e.currentTarget.checked)}
      />
      <span>Envoyer tout seul à Firehouse les mises à jour et DLC Switch qu’il n’a pas (sans redemander)</span>
    </label>
    <!-- Mods Switch (✅ Seb 18/11, choix 1a) : la case « À l'avenir, mettre à jour tout seul » de l'accord. -->
    <label class="choix">
      <input
        type="checkbox"
        checked={!!etat.profil.modsSwitch?.auto}
        onchange={(e) => reglerProfil('modsSwitch.auto', e.currentTarget.checked)}
      />
      <span>🧩 Mettre à jour tout seul les mods Switch déjà téléchargés quand Firehouse en a une nouvelle version (sans redemander)</span>
    </label>
    <label class="champ">
      <span>Débit d’envoi à Firehouse</span>
      <select value={etat.pc.envoisFirehouse?.debit ?? 'moitie'} onchange={(e) => reglerPc('envoisFirehouse.debit', e.currentTarget.value)}>
        <option value="moitie">La moitié de ma connexion (conseillé : Internet reste rapide)</option>
        <option value="sans_limite">Sans limite (plus rapide, mais Internet ralentit pendant l’envoi)</option>
      </select>
    </label>
  </div>

  <div class="actions">
    <button class="btn primary" onclick={() => sauvegarder()} disabled={sauvegarde.enCours || etat.pc.firehouse.simule}>
      {libelleSauvegarde(sauvegarde.enCours, sauvegarde.partieEnCours)}
    </button>
    <button class="btn" onclick={() => restaurer()} disabled={sauvegarde.restauration !== null || etat.pc.firehouse.simule}>
      {sauvegarde.restauration
        ? `Restauration… ${sauvegarde.restauration.faits}/${sauvegarde.restauration.total}`
        : '📥 Restaurer une sauvegarde…'}
    </button>
  </div>
  {#if enDouble.length}
    <p>💾 {enDouble.length} partie(s) restaurée(s) existe(nt) en deux versions : la tienne est gardée, la restaurée attend à part.</p>
    <div class="actions">
      <button class="btn" onclick={async () => { await proposerRemplacements(enDouble); await relireDoubles(); }}>🔎 Voir les parties en double</button>
    </div>
  {/if}
  {#if etat.pc.firehouse.simule}<p class="muted">En mode simulé, rien n’est sauvegardé.</p>{/if}
</div>

<style>
  .sauvegarde {
    display: grid;
    gap: calc(12 * var(--u));
  }
  .muted {
    margin: 0;
  }
  .actions {
    display: flex;
    gap: calc(8 * var(--u));
  }
  .choix {
    display: flex;
    gap: calc(6 * var(--u));
    align-items: center;
    min-height: var(--cible-min);
    cursor: pointer;
  }
  /* Lot S4 : la jauge de la place chez Firehouse (couleurs du thème, aucune en dur). */
  .jauge {
    height: calc(8 * var(--u));
    border-radius: calc(4 * var(--u));
    background: var(--bg2);
    border: 1px solid var(--line);
    overflow: hidden;
  }
  .rempli {
    height: 100%;
    background: var(--accent);
  }
</style>
