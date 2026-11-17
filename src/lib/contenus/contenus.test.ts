import { describe, expect, it } from 'vitest';
import { cochesCoherentes, dlcEnPartieEden, emulateurAutomatique, libelleGenre, motifInterrupteursBloques, quelquesNoms, lignesRemarque, nomCourt, raisonNonPose, resume, resumeChoix, texteInterrupteur, type ContenuVu } from './contenus';

describe('les contenus additionnels', () => {
  it('se présentent simplement', () => {
    expect(nomCourt('Asuras Wrath - Episode 11.5 (Europe) (EnJaFrDeEsIt) (DLC)', "Asura's Wrath")).toBe('Episode 11.5');
    expect(nomCourt('God of War - Ascension - Achilles (Europe) (Avatar)', 'God of War - Ascension')).toBe('Ascension - Achilles');
    expect(nomCourt('Thème libre', 'Autre jeu')).toBe('Thème libre');
    expect(libelleGenre('avatar')).toBe('Avatar');
    expect(libelleGenre('licence')).toBe('Licence');
    expect(libelleGenre('inconnu')).toBe('Contenu');
    const c = { id: 'a', nom: 'x', genre: 'dlc', taille: 1, licence: false, installe: false };
    expect(resume([c, { ...c, id: 'b', installe: true }])).toBe('2 disponible(s), 1 installé(s)');
    expect(resume([c])).toBe('1 disponible(s)');
  });

  it('Switch : les interrupteurs se résument (Seb 17/11)', () => {
    // La forme réelle de `apercu_ryubing` / `apercu_eden` : `cle` et `actif` ; une vieille mise à jour n'a pas de clé.
    const c: ContenuVu = { id: 'a', nom: 'x', genre: 'dlc', taille: 1, licence: false, installe: true, cle: 'dlc:01007EF00011F001', actif: true };
    const vieille: ContenuVu = { ...c, id: 'b', genre: 'maj', installe: false, cle: null, actif: false, raison: 'pas utilisée : une plus récente est là' };
    const eteint: ContenuVu = { ...c, id: 'd', cle: 'dlc:01007EF00011F002', actif: false, installe: false };
    expect(resumeChoix([c, { ...c, id: 'c', cle: 'maj' }, { ...c, id: 'e', cle: 'dlc:3' }, vieille])).toBe('3 sur 3 utilisés');
    expect(resumeChoix([c, { ...c, id: 'c', cle: 'maj' }, eteint])).toBe('2 utilisés, 1 éteint');
    expect(resumeChoix([c])).toBe('1 sur 1 utilisé');
    expect(resumeChoix([vieille])).toBe('aucun à choisir');
    // Allumé mais pas pris par l'émulateur (cause technique) : jamais compté « utilisé », dit à part.
    const enPanne: ContenuVu = { ...c, id: 'f', installe: false, raison: 'Ryubing ne le prend pas pour l’instant' };
    expect(resumeChoix([c, enPanne])).toBe('1 utilisé, 1 allumé mais pas utilisé');
    expect(resumeChoix([c, enPanne, eteint])).toBe('1 utilisé, 1 éteint, 1 allumé mais pas utilisé');
    expect(texteInterrupteur(enPanne)).toBe('⚠ Allumé, pas utilisé');
    expect(texteInterrupteur(c)).toBe('✅ Utilisé');
    expect(texteInterrupteur(eteint)).toBe('⬜ Pas utilisé');
    // Eden, DLC éteint mais encore utilisé (il les garde tous ; Seb 17/11) : le panneau dit ce qui se passe vraiment.
    const maj: ContenuVu = { ...c, cle: 'maj', id: 'm', genre: 'maj' };
    const eden = [maj, c, eteint];
    expect(texteInterrupteur(eteint, 'eden', eden)).toBe('⚠ Éteint, mais Eden l’utilise');
    expect(resumeChoix(eden, 'eden')).toBe('3 utilisés (dont 1 éteint qu’Eden garde)');
    // La même liste avec Ryubing (il coupe un DLC précis) : jamais le texte d'Eden.
    expect(texteInterrupteur(eteint, 'ryubing', eden)).toBe('⬜ Pas utilisé');
    expect(resumeChoix(eden, 'ryubing')).toBe('2 utilisés, 1 éteint');
    // Tous éteints : Eden les coupe, jamais « Eden l'utilise » — même si le cœur, lecture incomplète, dit installe.
    const tous = [maj, { ...c, actif: false, installe: true }, { ...eteint, installe: true }];
    expect(texteInterrupteur(tous[1], 'eden', tous)).toBe('⬜ Pas utilisé');
    expect(resumeChoix(tous, 'eden')).toBe('1 utilisé, 2 éteints');
    // Mise à jour éteinte + DLC gardé par Eden : lisible.
    const mixte = [{ ...maj, actif: false, installe: false }, c, eteint];
    expect(resumeChoix(mixte, 'eden')).toBe('2 utilisés (dont 1 éteint qu’Eden garde), 1 éteint');
    // Eden : la remarque seulement quand des DLC sont éteints EN PARTIE.
    expect(dlcEnPartieEden('eden', [c, eteint])).toBe(true);
    expect(dlcEnPartieEden('eden', [{ ...c, actif: false }, eteint])).toBe(false);
    expect(dlcEnPartieEden('ryubing', [c, eteint])).toBe(false);
    expect(motifInterrupteursBloques(null)).toContain('Une partie est en cours');
    expect(motifInterrupteursBloques('🕹 Ryubing est ouvert : ferme-le pour jouer.')).toContain('Ryubing est ouvert');
    // La cause technique vient du cœur, jamais devinée.
    expect(raisonNonPose(vieille)).toBe('pas utilisée : une plus récente est là');
    expect(raisonNonPose({ ...c, installe: false, raison: "Ryubing ne le prend pas pour l'instant" })).toBe("Ryubing ne le prend pas pour l'instant");
    expect(raisonNonPose({ ...c, installe: false, raison: null })).toBe('pas utilisé');
    expect(lignesRemarque('La liste des DLC de Ryubing ne se lit pas.\nDLC non utilisés (…)\n')).toEqual([
      'La liste des DLC de Ryubing ne se lit pas.',
      'DLC non utilisés (…)',
    ]);
    expect(lignesRemarque(null)).toEqual([]);
    // Eden aussi (Seb 16/11 : automatique partout) ; la PS3 reste au choix de la personne, la Wii U incluse.
    expect(emulateurAutomatique('ryubing')).toBe('Ryubing');
    expect(emulateurAutomatique('eden')).toBe('Eden');
    expect(emulateurAutomatique('ps3')).toBeNull();
    expect(emulateurAutomatique('switch')).toBeNull();
    expect(emulateurAutomatique(undefined)).toBeNull();
    expect(quelquesNoms(['a', 'b'])).toBe('a, b');
    expect(quelquesNoms(['1', '2', '3', '4', '5', '6', '7'])).toBe('1, 2, 3, 4, 5 et 2 autre(s)');
  });

  it('cochent les mises à jour dans l’ordre de la chaîne', () => {
    const maj = (v: string, installe = false): ContenuVu => ({ id: `A${v}`, nom: `Mise à jour ${v}`, genre: 'maj', taille: 1, licence: false, installe, version: v });
    const l = [maj('01.05', true), maj('01.06'), maj('01.07'), maj('01.08'), { id: 'dlc', nom: 'DLC', genre: 'dlc', taille: 1, licence: false, installe: false }];
    // Cocher 01.08 coche aussi 01.06 et 01.07 (01.05 est installée).
    expect(cochesCoherentes(l, ['A01.08'], 'A01.08').sort()).toEqual(['A01.06', 'A01.07', 'A01.08']);
    // Décocher 01.06 décoche les suivantes, jamais le DLC.
    expect(cochesCoherentes(l, ['A01.07', 'A01.08', 'dlc'], 'A01.06')).toEqual(['dlc']);
    // Un DLC ne change rien aux autres.
    expect(cochesCoherentes(l, ['dlc'], 'dlc')).toEqual(['dlc']);
  });
});
