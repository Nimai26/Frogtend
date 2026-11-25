// Lot M1 : les textes du bloc des mods de Vortex, GÉNÉRÉS depuis l'état du cœur (aucun texte propre à un jeu).
import { describe, expect, it } from 'vitest';
import type { DiagnosticVortex, EtatVortex, ModVortex } from '$lib/api';
import {
  autresMods,
  blocVortexVisible,
  chercher,
  compteRecherche,
  encoreOuvert,
  enteteVortex,
  libelleRetrait,
  ligneInterrompu,
  lignesDiagnostic,
  lignesRetrait,
  ligneVortexRevenu,
  messageApresRemise,
  messageRemise,
  messageRetrait,
  messageToutRemettre,
  modsEnCause,
  modsEnPlace,
  motifVerrou,
  nb,
  titreRetrait,
  toastRetrait,
  trier,
} from './vortex';

const SANS: DiagnosticVortex = { version: null, version_precedente: null, refus: [], plante: null, erreurs_scripts: 0, outil_scripts: null };
/** Le diagnostic réel de Cyberpunk 2077 (relevé du 25/11). */
const CYBERPUNK: DiagnosticVortex = {
  version: '2.50',
  version_precedente: '2.21',
  refus: [{ outil: 'Cyber Engine Tweaks', version_attendue: '2.21' }],
  plante: { chargeur: 'RED4ext', greffon: 'ArchiveXL' },
  erreurs_scripts: 0,
  outil_scripts: null,
};
const m = (nom: string, fichiers = 1, source = `${nom}-1-1-1700000000`): ModVortex => ({ source, nom, fichiers });

function etatVortex(x: Partial<EtatVortex> = {}): EtatVortex {
  const mods = Array.from({ length: 284 }, (_, i) => m(`Mod ${String(i).padStart(3, '0')}`, 7));
  return {
    present: true,
    refus: null,
    mods,
    fichiers: 2015,
    octets: 4.1 * 1024 ** 3,
    retrait: null,
    retires: [],
    diagnostic: CYBERPUNK,
    en_cause: [],
    verrou: null,
    ...x,
  };
}

const taille = (o: number) => `${(o / 1024 ** 3).toLocaleString('fr-FR', { maximumFractionDigits: 1 })} Go`;

describe('les textes des mods de Vortex', () => {
  it('le bloc ne se montre que s’il a quelque chose à dire', () => {
    expect(blocVortexVisible(null)).toBe(false);
    expect(blocVortexVisible(etatVortex({ present: false }))).toBe(false);
    expect(blocVortexVisible(etatVortex())).toBe(true);
    expect(blocVortexVisible(etatVortex({ present: false, retires: [m('A')] }))).toBe(true);
  });

  it('l’en-tête dit le nombre de mods et si le jeu ne démarre plus', () => {
    expect(enteteVortex(etatVortex())).toBe('284 mods posés par Vortex · ⚠ le jeu ne démarre plus');
    expect(enteteVortex(etatVortex({ mods: [m('A')], diagnostic: SANS }))).toBe('1 mod posé par Vortex');
    expect(enteteVortex(etatVortex({ present: false, retires: [m('A'), m('B')], diagnostic: SANS }))).toBe('2 mods retirés de Vortex');
    expect(enteteVortex(etatVortex({ retrait: { en_cours: true, faits: 812, total: 2015, laisses: 0, laisses_rompus: 0, sauvegardes_vortex: 0 } }))).toContain('interrompu');
  });

  it('le diagnostic se dit avec les mots de Seb, générés depuis ce que le cœur a lu', () => {
    expect(lignesDiagnostic(CYBERPUNK, 'Cyberpunk 2077')).toEqual([
      '⚠ Cyberpunk 2077 a été mis à jour (version 2.21 → 2.50) et certains mods ne suivent plus : le jeu ne démarre plus.',
      '« Cyber Engine Tweaks » ne marche pas avec cette version du jeu.',
      '« RED4ext » s’arrête en chargeant « ArchiveXL ».',
    ]);
    // Un autre jeu, une autre version : rien d'écrit pour Cyberpunk.
    const autre = { ...CYBERPUNK, version: '1.7', version_precedente: '1.6', refus: [], plante: { chargeur: 'Chargeur', greffon: 'Truc' } };
    expect(lignesDiagnostic(autre, 'Mon jeu')[0]).toBe('⚠ Mon jeu a été mis à jour (version 1.6 → 1.7) et certains mods ne suivent plus : le jeu ne démarre plus.');
    expect(lignesDiagnostic({ ...SANS, version: '2.50', version_precedente: '2.21' }, 'X')).toEqual(['X a été mis à jour (version 2.21 → 2.50) : certains mods peuvent ne plus suivre.']);
    expect(lignesDiagnostic({ ...SANS, version: '2.50', version_precedente: '2.21', refus: [{ outil: 'CET', version_attendue: null }] }, 'X')[0]).toBe('⚠ X a été mis à jour (version 2.21 → 2.50) et certains mods ne suivent plus.');
    expect(lignesDiagnostic({ ...SANS, erreurs_scripts: 3, outil_scripts: 'redscript' }, 'X')).toEqual(['« redscript » trouve 3 erreurs dans les scripts des mods.']);
    expect(lignesDiagnostic({ ...SANS, plante: { chargeur: 'RED4ext', greffon: 'A' } }, 'X')[0]).toBe('⚠ Le jeu ne démarre plus : un mod l’en empêche.');
    expect(lignesDiagnostic(SANS, 'X')).toEqual([]);
  });

  it('le retrait est annoncé et chiffré avant le oui', () => {
    const e = etatVortex();
    expect(libelleRetrait(e)).toBe('🧹 Retirer les mods de Vortex (284)');
    expect(titreRetrait(e)).toBe('🧹 Retirer les 284 mods de Vortex ?');
    expect(messageRetrait({ ...e, fichiers: 2014 }, taille)).toBe(
      `Le jeu démarrera sans mods. Aucun fichier de mod n’est effacé ni copié : les ${nb(2014)} fichiers (4,1 Go) sont rangés dans un dossier à part, dans le dossier du jeu, en quelques secondes et sans place en plus sur le disque. Tes parties sont mises à l’abri avant. Tu pourras remettre chaque mod, un par un. Ensuite, désinstalle Vortex (Paramètres de Windows ▸ Applications ▸ Vortex ▸ Désinstaller) et ne l’ouvre plus d’ici là : il remettrait ses mods.`,
    );
    expect(nb(2014)).not.toBe('2014');
    expect(titreRetrait(etatVortex({ mods: [m('A')] }))).toBe('🧹 Retirer le mod de Vortex ?');
  });

  it('après le retrait : combien, le fichier laissé, les réglages et les parties', () => {
    expect(lignesRetrait({ mods: 284, laisses: 1, laisses_rompus: 1 })).toEqual([
      '✅ 284 mods retirés.',
      '1 fichier laissé tel quel : il ne menait plus nulle part.',
      'Désinstalle maintenant Vortex (Paramètres de Windows ▸ Applications ▸ Vortex ▸ Désinstaller) : s’il est rouvert, il remet ses mods.',
      'Les réglages que tes mods avaient écrits restent dans le jeu : ils resserviront si tu les remets.',
      '⚠ Tes parties faites avec des mods peuvent en avoir besoin : si une partie refuse de se charger, remets le mod.',
    ]);
    const l = lignesRetrait({ mods: 1, laisses: 3, laisses_rompus: 0 });
    expect(l[0]).toBe('✅ 1 mod retiré.');
    expect(l[1]).toBe('3 fichiers laissés tels quels : Frogtend n’a pas pu prouver qu’ils venaient de Vortex.');
    // I5 : une sauvegarde de Vortex à côté d'un fichier rangé se dit en une phrase.
    expect(lignesRetrait({ mods: 2, laisses: 0, laisses_rompus: 0, sauvegardes_vortex: 1 })).toContain(
      'Vortex avait gardé à côté l’original d’un fichier du jeu : Frogtend le laisse tel quel. Si le jeu se plaint d’un fichier, fais vérifier ses fichiers par sa boutique.',
    );
    expect(lignesRetrait({ mods: 2, laisses: 0, laisses_rompus: 0 }).some((x) => x.startsWith('Vortex avait'))).toBe(false);
    // Le toast : combien, le laissé, et Vortex à désinstaller.
    expect(toastRetrait({ mods: 284, laisses: 1, laisses_rompus: 1 })).toBe(
      '✅ 284 mods retirés. 1 fichier laissé tel quel : il ne menait plus nulle part. Désinstalle maintenant Vortex (Paramètres de Windows ▸ Applications ▸ Vortex ▸ Désinstaller) : s’il est rouvert, il remet ses mods.',
    );
  });

  it('B1 : après le retrait, plus d’alerte « ne démarre plus » ; Vortex revenu se dit', () => {
    const apres = etatVortex({ present: false, mods: [], retires: [m('A')] });
    expect(enteteVortex(apres)).toBe('1 mod retiré de Vortex');
    expect(modsEnPlace(apres)).toBe(false);
    expect(ligneVortexRevenu(apres)).toBeNull();
    expect(ligneVortexRevenu(etatVortex({ retires: [m('A')] }))).toMatch(/^⚠ Vortex a reposé des mods/);
  });

  it('« tout remettre » au singulier comme au pluriel ; ce qui bloque encore', () => {
    expect(messageToutRemettre(false, 1, '2.50')).toBe('Le mod retiré revient dans le jeu, à sa place, comme Vortex l’avait posé. Il ne marche peut-être pas avec la version 2.50. Tes parties sont mises à l’abri avant.');
    expect(messageToutRemettre(false, 284, null)).toMatch(/^Les 284 mods retirés reviennent .* Ils ne marchent peut-être pas avec cette version du jeu/);
    expect(messageToutRemettre(true, 0, null)).toMatch(/déjà rangés/);
    expect(encoreOuvert('galaxy')).toMatch(/^GOG Galaxy est encore ouvert/);
    expect(encoreOuvert('vortex')).toMatch(/^Vortex est encore ouvert/);
    expect(encoreOuvert(null)).toBeNull();
  });

  it('la reprise, la remise et le verrou', () => {
    expect(ligneInterrompu({ faits: 812, total: 2015 })).toBe(`⚠ Le retrait des mods a été interrompu (812 fichiers sur ${nb(2015)})`);
    expect(messageRemise(m('ArchiveXL', 12), '2.50')).toBe('« ArchiveXL » revient dans le jeu (12 fichiers). Il ne marche peut-être pas avec la version 2.50.');
    expect(messageRemise(m('A', 1), null)).toBe('« A » revient dans le jeu (1 fichier). Il ne marche peut-être pas avec cette version du jeu.');
    expect(messageApresRemise({ remis: 11, places_prises: 1, changes: 0, deploiement_remis: false }, 'ArchiveXL')).toBe(
      '✅ « ArchiveXL » remis dans le jeu (11 fichiers). 1 fichier reste rangé : sa place dans le jeu est prise par un autre.',
    );
    expect(messageApresRemise({ remis: 3, places_prises: 0, changes: 2, deploiement_remis: true }, null)).toBe('✅ Mods remis dans le jeu (3 fichiers). 2 fichiers rangés ont changé : ils restent à part.');
    expect(motifVerrou('galaxy', false)).toMatch(/^⛔ Ferme GOG Galaxy d’abord/);
    expect(motifVerrou('partie', false)).toBe('⛔ Le jeu est en cours, ou Frogtend y travaille : attends la fin.');
    expect(motifVerrou('vortex', false)).toMatch(/^⛔ Ferme Vortex d’abord/);
    expect(motifVerrou(null, true)).toMatch(/en cours/);
    expect(motifVerrou(null, false)).toBeNull();
  });

  it('les mods en cause d’abord, les autres triés à la française, la recherche sans accents', () => {
    const mods = [m('Écran'), m('archiveXL', 3, 'ArchiveXL-4198-1-21-1-1737797101'), m('Zeta'), m('eau')];
    const e = etatVortex({ mods, en_cause: ['ArchiveXL-4198-1-21-1-1737797101'] });
    expect(modsEnCause(e, mods).map((x) => x.nom)).toEqual(['archiveXL']);
    expect(autresMods(e, mods).map((x) => x.nom)).toEqual(['eau', 'Écran', 'Zeta']);
    expect(trier(mods).map((x) => x.nom)).toEqual(['archiveXL', 'eau', 'Écran', 'Zeta']);
    expect(chercher(mods, 'ecr').map((x) => x.nom)).toEqual(['Écran']);
    expect(compteRecherche(1, 284)).toBe('1 sur 284');
    expect(compteRecherche(0, 284)).toMatch(/Aucun/);
  });

  it('aucun texte ne dit un chemin ni un mot technique', () => {
    const tous = [
      enteteVortex(etatVortex()),
      ...lignesDiagnostic(CYBERPUNK, 'Cyberpunk 2077'),
      messageRetrait(etatVortex(), taille),
      ...lignesRetrait({ mods: 2, laisses: 2, laisses_rompus: 1 }),
      motifVerrou('galaxy', false) ?? '',
    ].join(' ');
    expect(tous).not.toMatch(/lien dur|manifeste|deployment|\.json|\\|[A-Z]:\//i);
  });
});
