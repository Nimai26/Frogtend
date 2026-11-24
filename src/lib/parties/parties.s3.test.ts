// Lot S3 (23/11) : les textes de l'abri AUTOMATIQUE, sur les formes RÉELLES du cœur (`AbriVu.declencheur`,
// `BilanAbriAuto`, `Erreur::AbriImpossible` de commandes.rs / erreurs.rs).
import { describe, expect, it } from 'vitest';
import type { AbriVu, BilanAbriAuto, ErreurCoeur } from '$lib/api';
import {
  abrisAMontrer,
  estProblemeAbriAuto,
  ligneAbriAuto,
  messageAbriImpossible,
  messageSansAbri,
  optionsAbriImpossible,
  texteDeclencheur,
  texteDernierAbri,
  texteOperation,
  texteQuand,
  texteSorteAbri,
  texteToastAbriAuto,
  texteTotalAbris,
} from './parties';

const abri = (date: number, p: Partial<AbriVu> = {}): AbriVu => ({
  date: String(date),
  fichiers: 2,
  octets: 1024 * 1024,
  sorte: 'emplacements',
  a_moi: true,
  echecs: 0,
  en_ligne_seulement: 0,
  sans_origine: 0,
  declencheur: 'apres_partie',
  ...p,
});

const bilan = (p: Partial<BilanAbriAuto> = {}): BilanAbriAuto => ({
  jeu: 900,
  profil: 'p1',
  declencheur: 'apres_partie',
  etat: 'fait',
  fichiers: 3,
  octets: 7,
  en_ligne: 0,
  octets_en_ligne: 0,
  echecs: 0,
  motif: null,
  ...p,
});

describe('🛟 Les déclencheurs d’un abri', () => {
  it('chaque déclencheur a son repère et ses mots ; l’inconnu et l’absent valent « à la main »', () => {
    expect(texteDeclencheur('main')).toEqual({ repere: '🖐', texte: 'mis à l’abri à la main' });
    expect(texteDeclencheur('apres_partie')).toEqual({ repere: '🏁', texte: 'après une partie' });
    expect(texteDeclencheur('avant_reinstallation').texte).toBe('avant une réinstallation');
    expect(texteDeclencheur('avant_mise_a_jour').texte).toBe('avant une mise à jour');
    expect(texteDeclencheur('avant_retrait').texte).toBe('avant de retirer le jeu');
    expect(texteDeclencheur('avant_mod')).toEqual({ repere: '🛠', texte: 'avant un changement de mod' });
    // Lot S7 : une partie jouée hors de Frogtend, rattrapée à l'ouverture du profil.
    expect(texteDeclencheur('rattrapage')).toEqual({ repere: '🛟', texte: 'rattrapé' });
    expect(texteDeclencheur(undefined).texte).toBe('mis à l’abri à la main');
    expect(texteDeclencheur('').texte).toBe('mis à l’abri à la main');
    // Un déclencheur d'une version plus récente : jamais « à la main ».
    expect(texteDeclencheur('autre chose')).toEqual({ repere: '🛟', texte: 'mis à l’abri par Frogtend' });
  });

  it('la liste dit le déclencheur ; les anciens abris gardent leur libellé', () => {
    expect(texteSorteAbri(abri(1))).toBe('🏁 après une partie · dans ton profil');
    expect(texteSorteAbri(abri(1, { declencheur: 'avant_mod' }))).toBe('🛠 avant un changement de mod · dans ton profil');
    // Lot S7 : un abri rattrapé à confirmer, ou dit « pas à moi ».
    expect(texteSorteAbri(abri(1, { declencheur: 'rattrapage', a_confirmer: true }))).toBe('🛟 rattrapé · dans ton profil · à confirmer : c’est toi qui as joué ?');
    expect(texteSorteAbri(abri(1, { declencheur: 'rattrapage', pas_a_moi: true }))).toBe('🛟 rattrapé · dans ton profil · pas à toi : gardé sur ce PC, jamais envoyé');
    // Un abri du lot S2 (sans le champ) : à la main.
    expect(texteSorteAbri(abri(1, { declencheur: undefined }))).toBe('🖐 mis à l’abri à la main · dans ton profil');
    // Un abri du dossier du jeu : son libellé d'avant, sans déclencheur.
    expect(texteSorteAbri(abri(1, { sorte: 'jeu', declencheur: '' }))).toBe('mis à l’abri par Frogtend (avant un mod, une réinstallation ou un retrait) · dans ton profil');
  });
});

describe('🛟 Dernier abri et liste longue', () => {
  const maintenant = new Date(2026, 10, 23, 22, 0);
  const a21h14 = new Date(2026, 10, 23, 21, 14).getTime() / 1000;

  it('« aujourd’hui à 21:14, après une partie » ; hier ; une autre date', () => {
    expect(texteQuand(String(a21h14), maintenant)).toBe('aujourd’hui à 21:14');
    expect(texteQuand(String(a21h14 - 86400), maintenant)).toBe('hier à 21:14');
    expect(texteQuand(String(a21h14 - 3 * 86400), maintenant)).toBe('le 20 novembre à 21:14');
    expect(texteQuand('abc', maintenant)).toBe('à une date inconnue');
    expect(texteDernierAbri([abri(a21h14)], maintenant)).toBe('🛟 Dernier abri : aujourd’hui à 21:14, après une partie');
    expect(texteDernierAbri([abri(a21h14, { declencheur: 'avant_retrait' })], maintenant)).toBe('🛟 Dernier abri : aujourd’hui à 21:14, avant de retirer le jeu');
    expect(texteDernierAbri([abri(a21h14, { declencheur: 'main' })], maintenant)).toBe('🛟 Dernier abri : aujourd’hui à 21:14, fait à la main');
    expect(texteDernierAbri([abri(a21h14, { declencheur: 'rattrapage' })], maintenant)).toBe('🛟 Dernier abri : aujourd’hui à 21:14, rattrapé');
    expect(texteDernierAbri([abri(a21h14, { sorte: 'nuage-steam', declencheur: '' })], maintenant)).toBe('🛟 Dernier abri : aujourd’hui à 21:14, avec le nuage de Steam');
    // Seulement un abri de CE profil (un ancien sans propriétaire ne compte pas).
    expect(texteDernierAbri([abri(a21h14, { a_moi: false })], maintenant)).toBeNull();
    expect(texteDernierAbri([], maintenant)).toBeNull();
    expect(texteDernierAbri(undefined, maintenant)).toBeNull();
  });

  it('5 plus récents, plus le dernier de chaque « avant … » s’il est plus ancien ; le reste se déplie', () => {
    const l = [
      ...Array.from({ length: 6 }, (_, i) => abri(100 - i)),
      abri(90, { declencheur: 'avant_retrait' }),
      abri(89),
      abri(88, { declencheur: 'avant_retrait' }),
      abri(87, { declencheur: 'avant_mod' }),
      abri(86, { sorte: 'jeu', declencheur: '' }),
    ];
    const { montres, caches } = abrisAMontrer(l);
    expect(montres.map((a) => a.date)).toEqual(['100', '99', '98', '97', '96', '90', '87']);
    expect(caches).toBe(4);
    expect(abrisAMontrer(l, true)).toEqual({ montres: l, caches: 0 });
    expect(abrisAMontrer(l.slice(0, 3))).toEqual({ montres: l.slice(0, 3), caches: 0 });
    expect(texteTotalAbris(l)).toMatch(/^11 abris · /);
    expect(texteTotalAbris([abri(1)])).toMatch(/^1 abri · /);
    expect(texteTotalAbris([])).toBeNull();
  });
});

describe('🛟 Après une partie : ce qui est dit', () => {
  it('réussi : court, sans chiffres ni chemin ; rien de neuf, aucun, en cours : rien', () => {
    expect(texteToastAbriAuto(bilan(), 'Hades')).toBe('🛟 Tes parties de « Hades » sont à l’abri.');
    for (const etat of ['rien_de_neuf', 'aucun', 'en_cours'] as const) expect(texteToastAbriAuto(bilan({ etat }), 'Hades')).toBeNull();
    expect(estProblemeAbriAuto(bilan())).toBe(false);
  });

  it('un problème : un toast sans faire peur (sauf attente dépassée et inconnu : sans bruit), et une ligne qui reste (sans titre)', () => {
    for (const etat of ['en_ligne', 'trop_gros', 'echec', 'attente_depassee', 'inconnu'] as const) {
      const b = bilan({ etat, en_ligne: 9, octets_en_ligne: 180 * 1024 * 1024, motif: 'Le jeu semble encore écrire ses parties : réessaie dans une minute.' });
      expect(estProblemeAbriAuto(b)).toBe(true);
      if (etat === 'attente_depassee' || etat === 'inconnu') expect(texteToastAbriAuto(b, 'Hades')).toBeNull();
      else expect(texteToastAbriAuto(b, 'Hades')).toContain('« Hades »');
      const ligne = ligneAbriAuto(b)!;
      expect(ligne).toBeTruthy();
      expect(ligne).not.toContain('Hades');
      // Une consigne (« réessaie dans une minute ») n'est jamais recopiée à côté de « Frogtend réessaiera ».
      expect(ligne).not.toContain('réessaie dans une minute');
    }
    expect(ligneAbriAuto(bilan({ etat: 'inconnu' }))).toContain('ne sait pas encore où ce jeu range ses parties');
    expect(ligneAbriAuto(bilan({ etat: 'en_ligne', en_ligne: 9, octets_en_ligne: 180 * 1024 * 1024 }))).toMatch(/^☁ Dernier abri automatique : 9 fichiers \(.+\) seulement dans ton nuage OneDrive/);
    expect(ligneAbriAuto(bilan({ etat: 'echec', motif: 'Plus assez de place sur le disque des abris (il faut 1025 Mo) : rien n’est mis à l’abri.' }))).toBe(
      '⚠ Dernier abri automatique impossible : Plus assez de place sur le disque des abris (il faut 1025 Mo) : rien n’est mis à l’abri. Frogtend réessaiera après ta prochaine partie.',
    );
    expect(texteToastAbriAuto(bilan({ etat: 'echec' }), 'Hades')).toContain('elles sont toujours dans le jeu');
    // Lot S7 : un échec du RATTRAPAGE se refait à l'ouverture de Frogtend, jamais « après ta prochaine partie ».
    for (const etat of ['echec', 'attente_depassee', 'inconnu'] as const) {
      const l = ligneAbriAuto(bilan({ etat, declencheur: 'rattrapage', motif: 'Frogtend n’a pas pu relire la clé.' }))!;
      expect(l).toContain('à la prochaine ouverture de Frogtend');
      expect(l).not.toContain('après ta prochaine partie');
    }
    expect(ligneAbriAuto(bilan())).toBeNull();
    expect(ligneAbriAuto(null)).toBeNull();
  });
});

describe('🛟 Avant une opération : l’abri a échoué', () => {
  const erreur = (p: Partial<ErreurCoeur>): ErreurCoeur => ({ sorte: 'abri_impossible', motif: 'Le motif.', cause: 'echec', operation: 'avant_retrait', fichiers_en_ligne: 0, octets_en_ligne: 0, octets: 0, ...p });

  it('l’ordre des choix : l’option qui règle la cause d’abord (focus), Réessayer, Continuer sans abri', () => {
    const echec = optionsAbriImpossible(erreur({}));
    expect(echec.map((o) => o.valeur)).toEqual(['reessayer', 'sans_abri']);
    expect(echec[0].detail).toBe('si le jeu est ouvert, quitte-le d’abord');
    const nuage = optionsAbriImpossible(erreur({ cause: 'onedrive', fichiers_en_ligne: 9, octets_en_ligne: 180 }));
    expect(nuage.map((o) => o.valeur)).toEqual(['onedrive', 'reessayer', 'sans_abri']);
    expect(nuage[0].libelle).toBe('☁ Télécharger depuis OneDrive puis réessayer');
    expect(nuage[0].detail).toMatch(/^9 fichiers · .+ à télécharger$/);
    expect(nuage[1]).toEqual({ valeur: 'reessayer', libelle: '🔁 Réessayer', detail: undefined });
    expect(nuage[2].libelle).toBe('Continuer sans abri');
    expect(optionsAbriImpossible(erreur({ cause: 'trop_gros', octets: 3 << 30 })).map((o) => o.valeur)).toEqual(['quand_meme', 'reessayer', 'sans_abri']);
    const inconnu = optionsAbriImpossible(erreur({ cause: 'inconnu' }));
    expect(inconnu.map((o) => o.valeur)).toEqual(['reessayer', 'sans_abri']);
    expect(inconnu[0].detail).toBe('Frogtend redemande à Firehouse où sont tes parties');
  });

  it('le message dit que rien n’a été fait, et l’opération en mots', () => {
    expect(messageAbriImpossible(erreur({}))).toBe('Le motif.\n\nRien n’a été fait : le retrait du jeu attend que tes parties soient à l’abri.');
    expect(texteOperation('avant_reinstallation')).toBe('la réinstallation');
    expect(texteOperation('avant_mise_a_jour')).toBe('la mise à jour');
    expect(texteOperation('avant_mod')).toBe('le changement de mod');
    expect(messageSansAbri(erreur({ operation: 'avant_mod' }))).toContain('avant le changement de mod');
  });
});
