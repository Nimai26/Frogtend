<script lang="ts">
  // 🔎 Chercher dans la base de Firehouse les jeux importés qui n'ont pas de fiche (contrat 1.32, besoin 16). Annoncé
  // d'abord (combien, ce qui part) ; seuls les jeux trouvés à coup sûr sont reliés tout seuls ; pour les autres, tu
  // choisis parmi ce que Firehouse propose (jamais le premier d'office), ou « aucun de ceux-là ».
  import { onMount, tick } from 'svelte';
  import { api } from '$lib/api';
  import { choisir, confirmer, toast } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import { nomAffiche, rechargerListe } from '$lib/ludotheque/ludotheque.svelte';
  import { rechargerJeuxDuPc } from '$lib/ludotheque/telechargements.svelte';
  import { libelleCandidat, texteAnnonceBase, type AChoisir } from './identification';

  const PAS = 100;
  let annonce = $state<[number, number, number] | null>(null);
  let aChoisir = $state<AChoisir[]>([]);
  let montres = $state(PAS);
  let enCours = $state(false);
  let occupe = $state(false);
  let liste: HTMLElement | undefined = $state();

  async function lire() {
    annonce = await api.identificationAnnonce().catch(() => null);
    aChoisir = await api.identificationAChoisir().catch(() => []);
  }
  onMount(lire);

  async function lancer(tout: boolean) {
    if (tout) {
      const n = (annonce?.[0] ?? 0) + (annonce?.[1] ?? 0);
      const ok = await confirmer(`🔎 Chercher de nouveau ${n} jeu(x) ?`, {
        message: 'Même ceux déjà cherchés (sauf ceux que tu as déjà choisis toi-même). Seuls leurs noms partent ; aucun fichier n’est lu ni modifié.',
        libelleValider: 'Chercher',
      });
      if (!ok) return;
    }
    enCours = true;
    try {
      const b = await api.identificationLancer(tout);
      toast(
        `🔎 ${b.exacts} jeu(x) trouvé(s) à coup sûr, ${b.probables} à choisir, ${b.inconnus} inconnu(s) de la base.` +
          (b.motif ? ` La recherche s’est arrêtée avant la fin : ${b.motif} (relance-la plus tard).` : ''),
        b.motif ? 'alerte' : 'ok',
      );
      await rechargerJeuxDuPc();
      await rechargerListe();
      await lire();
    } catch (e) {
      toast(`Recherche impossible : ${motifDuRefus(e)}`, 'erreur');
    } finally {
      enCours = false;
      // Le bouton a pu disparaître : le focus va à la première ligne à choisir, sinon au titre (jamais perdu).
      await tick();
      (liste?.querySelector<HTMLElement>('li button') ?? liste?.querySelector<HTMLElement>('h3'))?.focus();
    }
  }

  /** Après une action la ligne disparaît : le focus va à la ligne qui prend sa place, jamais perdu. */
  async function apres(indice: number) {
    await lire();
    await rechargerJeuxDuPc();
    await rechargerListe();
    await tick();
    const lignes = liste?.querySelectorAll<HTMLElement>('li') ?? [];
    (lignes[Math.min(indice, lignes.length - 1)]?.querySelector<HTMLElement>('button') ?? liste?.querySelector<HTMLElement>('h3'))?.focus();
  }

  async function decider(j: AChoisir, i: number) {
    if (occupe) return;
    occupe = true;
    try {
      const AUCUN = -1;
      const options = [
        ...j.candidats.map((c) => ({ valeur: c.launchbox_id, libelle: libelleCandidat(c, nomAffiche), detail: c.fiche_id ? 'Il est dans ta ludothèque : ton jeu y sera relié.' : undefined })),
        { valeur: AUCUN, libelle: '✖ Aucun de ceux-là' },
      ];
      let choix: number | null;
      if (j.candidats.length === 1) {
        const ok = await confirmer(`🔎 « ${j.titre} » est-il bien « ${libelleCandidat(j.candidats[0], nomAffiche)} » ?`, {
          message: 'Ton jeu reçoit sa jaquette et son résumé. Ses fichiers et son temps de jeu ne changent pas.',
          libelleValider: '✅ Oui, c’est ce jeu',
        });
        choix = ok ? j.candidats[0].launchbox_id : null;
      } else {
        choix = await choisir<number>(`🔎 Quel jeu est « ${j.titre} » ?`, options, 'Firehouse hésite entre ces jeux. Ses fichiers et son temps de jeu ne changent pas.');
      }
      if (choix === null) return;
      if (choix === AUCUN) await api.identificationEcarter(j.local);
      else await api.identificationChoisir(j.local, choix);
      await apres(i);
    } catch (e) {
      toast(`Refusé : ${motifDuRefus(e)}`, 'erreur');
    } finally {
      occupe = false;
    }
  }

  /** « Afficher plus » : le focus va à la première ligne nouvellement montrée. */
  async function plus() {
    const avant = montres;
    montres += PAS;
    await tick();
    liste?.querySelectorAll<HTMLElement>('li')[avant]?.querySelector<HTMLElement>('button')?.focus();
  }

  const ecarter = async (j: AChoisir, i: number) => {
    if (occupe) return;
    occupe = true;
    try {
      await api.identificationEcarter(j.local);
      toast(`« ${j.titre} » : plus proposé. « Tout chercher de nouveau » le reproposera si tu changes d’avis.`);
      await apres(i);
    } catch (e) {
      toast(`Refusé : ${motifDuRefus(e)}`, 'erreur');
    } finally {
      occupe = false;
    }
  };
</script>

<section class="cx-block" bind:this={liste}>
  <h3 tabindex="-1">🔎 Chercher mes jeux sans fiche dans la base de Firehouse</h3>
  {#if annonce}
    <p>{texteAnnonceBase(annonce[0], annonce[1])}</p>
    <div class="actions">
      {#if annonce[0] > 0}
        <button class="btn primary" onclick={() => lancer(false)} disabled={enCours}>{enCours ? 'Recherche…' : `🔎 Chercher ces ${annonce[0]} jeu(x)`}</button>
      {/if}
      {#if annonce[1] > 0}
        <button class="btn" onclick={() => lancer(true)} disabled={enCours}>↻ Tout chercher de nouveau</button>
      {/if}
    </div>
  {/if}
  {#if aChoisir.length}
    <h4>À choisir ({aChoisir.length})</h4>
    <ul class="lignes">
      {#each aChoisir.slice(0, montres) as j, i (j.local)}
        <li>
          <span>{j.titre} <span class="muted">— {nomAffiche(j.plateforme)} · {j.candidats.length} proposition(s)</span></span>
          <span class="actions">
            <button class="btn primary" onclick={() => decider(j, i)} disabled={occupe}>Choisir</button>
            <button class="btn" onclick={() => ecarter(j, i)} disabled={occupe}>Aucun</button>
          </span>
        </li>
      {/each}
    </ul>
    {#if aChoisir.length > montres}<button class="btn" onclick={plus}>Afficher 100 de plus</button>{/if}
  {/if}
</section>

<style>
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: calc(8 * var(--u));
  }
  .lignes {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: calc(8 * var(--u));
  }
  .lignes li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: calc(12 * var(--u));
    flex-wrap: wrap;
    padding: calc(8 * var(--u)) 0;
  }
</style>
