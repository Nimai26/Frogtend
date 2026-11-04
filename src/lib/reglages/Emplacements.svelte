<script lang="ts">
  // Les emplacements des jeux sur CE PC : un ou plusieurs dossiers par défaut, et, si besoin, des dossiers propres
  // à un système (SNES, MS-DOS…), dans l'ordre de préférence. Retirer un emplacement n'efface AUCUN fichier.
  import { nomAffiche } from '$lib/ludotheque/ludotheque.svelte';
  import { onMount } from 'svelte';
  import { isTauri } from '@tauri-apps/api/core';
  import { api, taille, type Plateforme } from '$lib/api';
  import { choisir, confirmer, demander, fermerToast, informer, majProgres, toast, toastProgres } from '$lib/dialogues/fenetres.svelte';
  import { listen } from '@tauri-apps/api/event';
  import { dureeEstimee } from '$lib/decompression/decompression';
  import { etat, reglerPc } from '$lib/etat.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';

  const e = $derived(etat.pc.emplacements);
  let libres = $state<Record<string, number | null>>({});
  let plateformes = $state<Plateforme[]>([]);

  async function mesurer(chemins: string[]) {
    for (const c of chemins) {
      libres[c] = await api.espaceLibre(c).catch(() => null);
    }
  }
  $effect(() => {
    mesurer([...e.defaut, ...Object.values(e.systemes).flat()]);
  });
  onMount(async () => {
    plateformes = await api.plateformes(false).catch(() => []);
  });

  /** Demande un dossier : le sélecteur de Windows, ou un chemin tapé à la main. */
  async function demanderDossier(): Promise<string | null> {
    const facon = await choisir('📁 Ajouter un emplacement', [
      { valeur: 'parcourir', libelle: '📂 Choisir un dossier', detail: 'avec l’explorateur de Windows' },
      { valeur: 'taper', libelle: '✏ Taper le chemin', detail: 'par exemple E:\\Jeux ou \\\\NAS\\Jeux' },
    ]);
    if (facon === 'parcourir' && isTauri()) {
      return api.parcourir({ dossier: true, titre: 'Où ranger les jeux ?' });
    }
    if (facon === 'taper') {
      const c = await demander('✏ Chemin du dossier', { valeur: 'D:\\Jeux' });
      return c?.trim() || null;
    }
    return null;
  }

  async function ajouter(systeme: string | null) {
    const c = await demanderDossier();
    if (!c) return;
    // Jamais la racine d'un disque, Windows ou Documents : le dire, avec un sous-dossier proposé.
    const refus = await api.dossierJeuxRefus(c).catch(() => null);
    if (refus) {
      await informer('📁 Choisis un autre dossier', refus);
      return;
    }
    const libre = await api.espaceLibre(c).catch(() => null);
    if (libre === null) {
      toast(`Refusé : le dossier « ${c} » est introuvable. Crée-le d’abord, ou vérifie que le disque est branché.`, 'erreur');
      return;
    }
    if (systeme === null) {
      if (e.defaut.includes(c)) return;
      await reglerPc('emplacements.defaut', [...e.defaut, c]);
    } else {
      const liste = e.systemes[systeme] ?? [];
      if (liste.includes(c)) return;
      await reglerPc('emplacements.systemes', { ...e.systemes, [systeme]: [...liste, c] });
    }
    toast(`✅ Emplacement ajouté : ${c} (${taille(libre)} libres).`);
  }

  async function modifier(systeme: string | null, liste: string[]) {
    if (systeme === null) await reglerPc('emplacements.defaut', liste);
    else {
      const s = { ...e.systemes };
      if (liste.length === 0) delete s[systeme];
      else s[systeme] = liste;
      await reglerPc('emplacements.systemes', s);
    }
  }
  const monter = (l: string[], i: number) => (i > 0 ? [...l.slice(0, i - 1), l[i], l[i - 1], ...l.slice(i + 1)] : l);
  const retirer = (l: string[], i: number) => l.filter((_, k) => k !== i);

  /** Les autres dossiers de Frogtend (règle « pas de pieuvre ») : rien ne s'écrit ailleurs. */
  const AUTRES = [
    { cle: 'dossierEmulateurs', titre: '🕹 Émulateurs', aide: 'chaque émulateur dans son sous-dossier, avec sa configuration' },
    { cle: 'dossierOutils', titre: '🧰 Outils', aide: 'Cheat Engine, trainers, gestionnaires de mods' },
    { cle: 'dossierAbris', titre: '💾 Abris de parties', aide: 'la copie de tes parties avant de retirer un jeu' },
    { cle: 'dossierCaptures', titre: '📷 Captures d’écran', aide: 'les captures de tous les jeux, rangées par système puis par jeu (RetroArch, PS3, Game Boy Advance pour l’instant)' },
  ] as const;

  async function choisirAutre(cle: (typeof AUTRES)[number]['cle']) {
    const c = await demanderDossier();
    if (!c) return;
    // Même règle que pour les jeux : jamais la racine d'un disque, Windows ou Documents (le cœur le revérifie).
    const sorte = cle === 'dossierEmulateurs' ? 'emulateurs' : cle === 'dossierOutils' ? 'outils' : cle === 'dossierCaptures' ? 'captures' : 'abris';
    const refus = await api.dossierJeuxRefus(c, sorte).catch(() => null);
    if (refus) {
      await informer('📁 Choisis un autre dossier', refus);
      return;
    }
    const ancien = etat.pc[cle];
    const change = !!ancien && ancien.toLowerCase().replace(/[\\/]+$/, '') !== c.toLowerCase().replace(/[\\/]+$/, '');
    // Émulateurs et outils : ceux déjà installés restent où ils sont (et s'y mettent à jour) ; seuls les nouveaux vont
    // au nouveau dossier (déplacer un émulateur demande de réécrire ses chemins : pas encore, avis lancement 04/11).
    if (change && (cle === 'dossierEmulateurs' || cle === 'dossierOutils')) {
      const quoi = cle === 'dossierEmulateurs' ? 'émulateurs' : 'outils';
      const oui = await confirmer(`📁 Changer le dossier des ${quoi} ?`, {
        message: [
          `Ceux déjà installés restent dans ${ancien} : ils continuent de marcher et s’y mettent à jour.`,
          `Seuls les prochains iront dans ${c}.`,
        ].join('\n'),
        libelleValider: 'Changer',
      });
      if (!oui) return;
    }
    // Abris : le RÉGLAGE change d'abord (les nouveaux abris vont tout de suite au nouveau dossier, l'ancien reste lu),
    // PUIS ses abris sont déplacés (relecture lancement 04/11 : jamais d'abris dans un dossier que personne ne lit).
    if (change && cle === 'dossierAbris') {
      const m = await api.dossierContenu('abris', ancien).catch(() => null);
      const chiffres = m ? `${m.fichiers} fichier${m.fichiers > 1 ? 's' : ''}, ${taille(m.octets)} (${dureeEstimee(m.octets)})` : 'contenu illisible pour l’instant';
      const oui = await confirmer('💾 Changer le dossier des abris de parties ?', {
        message: [
          `Tes abris (${chiffres}) vont de ${ancien} à ${c}.`,
          'Chaque fichier est copié et vérifié avant d’être retiré de l’ancien dossier ; rien n’est écrit par-dessus. Seuls les abris de Frogtend bougent : rien d’autre de ce dossier.',
          'Ce qui ne pourrait pas partir reste lu là-bas : tes parties ne sont jamais perdues de vue.',
        ].join('\n'),
        libelleValider: 'Changer et déplacer',
      });
      if (!oui) return;
      await reglerPc('anciensDossiersAbris', [...new Set([...(etat.pc.anciensDossiersAbris ?? []), ancien])]);
      await reglerPc(cle, c);
      try {
        await api.abrisRegler(c);
      } catch (err) {
        toast(`Dossier réglé, mais Frogtend n’a pas pu le préparer : ${motifDuRefus(err)}`, 'erreur');
        return;
      }
      if (m && m.fichiers > 0) await deplacer('abris', ancien, c, m.octets);
      else toast(`✅ Dossier des abris réglé : ${c}.`);
      return;
    }
    // Captures : déplacer celles déjà prises, ou les laisser (AVANT le changement de réglage).
    if (change && cle === 'dossierCaptures') {
      const m = await api.dossierContenu('captures', ancien).catch(() => null);
      if (m && m.fichiers > 0) {
        const chiffres = `${m.fichiers} capture${m.fichiers > 1 ? 's' : ''}, ${taille(m.octets)} (${dureeEstimee(m.octets)})`;
        const d = await choisir<boolean>(
          '📷 Et les captures déjà prises ?',
          [
            { valeur: true, libelle: '📦 Les déplacer dans le nouveau dossier', detail: chiffres },
            { valeur: false, libelle: '📁 Les laisser où elles sont', detail: ancien },
          ],
          'Rien n’est écrit par-dessus une capture déjà là ; seules les captures rangées par Frogtend bougent.',
        );
        if (d === null) return;
        if (d && !(await deplacer('captures', ancien, c, m.octets))) return;
      }
    }
    await reglerPc(cle, c);
    if (cle === 'dossierAbris') {
      try {
        const n = await api.abrisRegler(c);
        toast(n ? `✅ Dossier des abris réglé ; ${n} abri(s) de parties y ont été rangés (copie vérifiée).` : '✅ Dossier des abris réglé.');
      } catch (err) {
        toast(`Dossier réglé, mais les anciens abris n’ont pas pu y être rangés : ${motifDuRefus(err)}`, 'erreur');
      }
    } else if (cle === 'dossierCaptures') {
      toast(`✅ Dossier des captures réglé : ${c}. Les captures des prochaines parties y seront rangées.`);
    } else {
      toast(`✅ Dossier réglé : ${c}. Les prochaines installations iront là ; celles déjà faites restent à leur place.`);
    }
  }

  /** Déplace le contenu d'un ancien dossier (barre de progression) ; rend faux si le déplacement a échoué. */
  async function deplacer(sorte: 'abris' | 'captures', ancien: string, nouveau: string, total: number): Promise<boolean> {
    const t = toastProgres(`📦 Déplacement vers ${nouveau}… Ne ferme pas Frogtend.`);
    const arreter = isTauri()
      ? await listen<{ octets: number }>('demenagement', (e) =>
          majProgres(t, { etape: 'Déplacement', chiffres: `${taille(e.payload.octets)} / ${taille(total)}`, pourcent: total ? Math.round((e.payload.octets / total) * 100) : 0 }),
        )
      : () => {};
    try {
      const b = await api.dossierDeplacerContenu(sorte, ancien, nouveau);
      if (b.laisses.length) {
        await informer('📦 Presque tout est déplacé', `${b.deplaces} fichier(s) déplacé(s). ${b.laisses.length} sont restés dans ${ancien} (déjà présents dans le nouveau dossier, ou impossibles à déplacer)${sorte === 'abris' ? ' : Frogtend continue de les lire là-bas.' : '.'}`);
      } else {
        toast(`✅ ${b.deplaces} fichier(s) déplacé(s) (${taille(b.octets)}).`);
      }
      return true;
    } catch (err) {
      toast(`Déplacement impossible : ${motifDuRefus(err)}${sorte === 'abris' ? ' Tes abris restent lus dans l’ancien dossier.' : ' Le dossier n’a pas changé.'}`, 'erreur');
      return false;
    } finally {
      arreter();
      fermerToast(t);
    }
  }

  async function ajouterSysteme() {
    const deja = new Set(Object.keys(e.systemes));
    const s = await choisir(
      '🎮 Pour quel système ?',
      plateformes.filter((p) => !deja.has(p.nom)).map((p) => ({ valeur: p.nom, libelle: nomAffiche(p.nom), detail: `${p.jeux} jeu(x)` })),
      'Ses jeux iront dans ses propres dossiers au lieu des dossiers par défaut.',
    );
    if (s) await ajouter(s);
  }
</script>

{#snippet liste(systeme: string | null, chemins: string[])}
  {#if chemins.length === 0}
    <p class="muted">Aucun dossier.</p>
  {:else}
    <ol class="chemins">
      {#each chemins as c, i (c)}
        <li>
          <span class="chemin">{c}</span>
          <span class="muted">
            {libres[c] === undefined ? '…' : libres[c] === null ? '⚠ introuvable' : `${taille(libres[c] ?? 0)} libres`}
          </span>
          <span class="boutons">
            {#if i > 0}
              <button class="btn petit" title="Préférer ce dossier" onclick={() => modifier(systeme, monter(chemins, i))}>↑</button>
            {/if}
            <button class="btn petit" title="Retirer (aucun fichier n’est effacé)" onclick={() => modifier(systeme, retirer(chemins, i))}>✕</button>
          </span>
        </li>
      {/each}
    </ol>
  {/if}
{/snippet}

<div class="emplacements">
  <p class="muted">
    Où ranger les jeux sur ce PC. Frogtend prend le premier dossier qui a assez de place. Retirer un dossier d’ici
    n’efface aucun fichier.
  </p>

  <div class="cx-block">
    <h3>Par défaut (tous les systèmes)</h3>
    {@render liste(null, e.defaut)}
    <button class="btn" onclick={() => ajouter(null)}>➕ Ajouter un dossier</button>
  </div>

  {#each Object.entries(e.systemes) as [systeme, chemins] (systeme)}
    <div class="cx-block">
      <h3>{nomAffiche(systeme)}</h3>
      {@render liste(systeme, chemins)}
      <button class="btn" onclick={() => ajouter(systeme)}>➕ Ajouter un dossier pour {nomAffiche(systeme)}</button>
    </div>
  {/each}

  <button class="btn" onclick={ajouterSysteme} disabled={plateformes.length === 0}>
    🎮 Des dossiers propres à un système…
  </button>

  <div class="cx-block">
    <h3>Les autres dossiers de Frogtend</h3>
    <p class="muted">Frogtend n’écrit rien en dehors de ces dossiers et de ceux des jeux. Ce qu’il ajoute à un jeu (jaquette, fiche, documents, mods, triches) va dans le dossier du jeu, sous « Frogtend ».</p>
    <dl class="cx-kv">
      {#each AUTRES as a (a.cle)}
        <dt>{a.titre}</dt>
        <dd>
          <span class="chemin">{etat.pc[a.cle] || (a.cle === 'dossierCaptures' ? 'pas choisi : chaque émulateur garde ses captures' : 'pas encore choisi (demandé quand il le faut)')}</span>
          <button class="btn petit" onclick={() => choisirAutre(a.cle)}>📂 Choisir</button>
          <span class="muted">{a.aide}</span>
        </dd>
      {/each}
    </dl>
  </div>
</div>

<style>
  .emplacements {
    display: grid;
    gap: calc(12 * var(--u));
  }
  .emplacements > .btn,
  .cx-block > .btn {
    justify-self: start;
  }
  .cx-block {
    display: grid;
    gap: calc(8 * var(--u));
  }
  .cx-block > h3 {
    margin-bottom: 0;
  }
  .muted {
    margin: 0;
  }
  .chemins {
    margin: 0;
    padding-left: calc(20 * var(--u));
    display: grid;
    gap: calc(6 * var(--u));
  }
  .chemins li {
    display: grid;
    grid-template-columns: 1fr auto auto;
    align-items: center;
    gap: calc(10 * var(--u));
  }
  .chemin {
    font-family: var(--mono);
    font-size: calc(12.5 * var(--u));
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .boutons {
    display: flex;
    gap: calc(4 * var(--u));
  }
  .petit {
    min-height: calc(30 * var(--u));
    padding: calc(2 * var(--u)) calc(9 * var(--u));
  }
</style>
