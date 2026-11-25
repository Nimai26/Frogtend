// Lot M1 (✅ Seb 25/11) : les textes du bloc « 🧩 Mods » des mods posés par VORTEX. Tout est GÉNÉRÉ depuis l'état et le
// diagnostic rendus par le cœur (aucun texte écrit pour un jeu précis) ; jamais un chemin, un « lien dur », un
// « manifeste » ni un nom de fichier technique à l'écran.
import type { BilanRemiseVortex, BilanRetraitModVortex, DiagnosticVortex, EtatVortex, ModVortex } from '$lib/api';
import { filtrerOptions } from '$lib/dialogues/filtre';

/** La liste se montre par paquets de cette taille (« Voir 25 de plus »). */
export const PAQUET = 25;

const pluriel = (n: number, un: string, plusieurs: string) => (n > 1 ? plusieurs : un);
/** Un nombre à la française (« 2 015 »). */
export const nb = (n: number) => n.toLocaleString('fr-FR');

/** La phrase qui dit de désinstaller Vortex (après le retrait, et dans son message). */
export const DESINSTALLER_VORTEX =
  'Désinstalle maintenant Vortex (Paramètres de Windows ▸ Applications ▸ Vortex ▸ Désinstaller) : s’il est rouvert, il remet ses mods.';

/** Le bloc a-t-il quelque chose à dire (Vortex présent, un retrait en cours, des mods à remettre ou remis) ? */
export function blocVortexVisible(e: EtatVortex | null | undefined): e is EtatVortex {
  return !!e && (e.present || !!e.retrait || e.retires.length > 0 || (e.remis?.length ?? 0) > 0);
}

/** Des mods de Vortex sont encore posés dans le jeu : seul cas où le diagnostic a un sens (après le retrait, ses
 * journaux parlent de mods qui ne sont plus là). */
export function modsEnPlace(e: EtatVortex): boolean {
  return e.present && e.mods.length > 0;
}

/** Le jeu ne démarre plus : un chargeur de mods s'arrête en chargeant l'un d'eux. */
export function neDemarrePlus(d: DiagnosticVortex): boolean {
  return d.plante !== null;
}

/** Le jeu a changé de version entre ses deux derniers journaux. */
export function misAJour(d: DiagnosticVortex): boolean {
  return !!d.version && !!d.version_precedente && d.version !== d.version_precedente;
}

/** L'en-tête du bloc (sans « 🧩 Mods : », que le bouton porte). */
export function enteteVortex(e: EtatVortex): string {
  if (e.retrait?.en_cours) return '⚠ le retrait des mods de Vortex a été interrompu';
  const alerte = modsEnPlace(e) && neDemarrePlus(e.diagnostic) ? ' · ⚠ le jeu ne démarre plus' : '';
  if (e.present && e.mods.length) {
    const n = e.mods.length;
    return `${nb(n)} ${pluriel(n, 'mod posé', 'mods posés')} par Vortex${alerte}`;
  }
  if (e.present) return 'des mods posés par Vortex';
  const n = e.retires.length;
  const r = e.remis?.length ?? 0;
  const dansLeJeu = `${nb(r)} ${pluriel(r, 'mod remis', 'mods remis')} dans le jeu`;
  if (!n) return dansLeJeu;
  const retires = `${nb(n)} ${pluriel(n, 'mod retiré', 'mods retirés')} de Vortex`;
  return r ? `${retires} · ${dansLeJeu}` : retires;
}

/** Ce que le diagnostic dit, phrase par phrase (rien s'il ne dit rien). */
export function lignesDiagnostic(d: DiagnosticVortex, titre: string): string[] {
  const l: string[] = [];
  const nom = titre.trim() || 'Le jeu';
  const demarre = neDemarrePlus(d);
  const souci = demarre || d.refus.length > 0 || d.erreurs_scripts > 0;
  if (misAJour(d)) {
    l.push(
      souci
        ? `⚠ ${nom} a été mis à jour (version ${d.version_precedente} → ${d.version}) et certains mods ne suivent plus${demarre ? ' : le jeu ne démarre plus' : ''}.`
        : `${nom} a été mis à jour (version ${d.version_precedente} → ${d.version}) : certains mods peuvent ne plus suivre.`,
    );
  } else if (demarre) {
    l.push('⚠ Le jeu ne démarre plus : un mod l’en empêche.');
  }
  for (const r of d.refus) l.push(`« ${r.outil} » ne marche pas avec cette version du jeu.`);
  if (d.plante) l.push(`« ${d.plante.chargeur} » s’arrête en chargeant « ${d.plante.greffon} ».`);
  if (d.erreurs_scripts > 0) {
    const n = d.erreurs_scripts;
    l.push(`« ${d.outil_scripts ?? 'Le compilateur des scripts'} » trouve ${nb(n)} ${pluriel(n, 'erreur', 'erreurs')} dans les scripts des mods.`);
  }
  return l;
}

/** Les mods désignés par le diagnostic, parmi `mods`. */
export function modsEnCause(e: EtatVortex, mods: ModVortex[]): ModVortex[] {
  return mods.filter((m) => e.en_cause.includes(m.source));
}

/** Les autres mods, triés par nom (en français). */
export function autresMods(e: EtatVortex, mods: ModVortex[]): ModVortex[] {
  return trier(mods.filter((m) => !e.en_cause.includes(m.source)));
}

/** Trié par nom, à la française. */
export function trier(mods: ModVortex[]): ModVortex[] {
  return [...mods].sort((a, b) => a.nom.localeCompare(b.nom, 'fr') || a.source.localeCompare(b.source, 'fr'));
}

/** Les mods dont le nom répond au texte tapé (sans majuscules ni accents). */
export function chercher(mods: ModVortex[], texte: string): ModVortex[] {
  return filtrerOptions(
    mods.map((m) => ({ libelle: m.nom, m })),
    texte,
  ).map((o) => o.m);
}

/** « N sur 282 », ou le mot quand rien ne répond. */
export function compteRecherche(montres: number, total: number): string {
  return montres ? `${nb(montres)} sur ${nb(total)}` : 'Aucun mod ne correspond : essaie un autre mot.';
}

/** Une ligne de mod : son nombre de fichiers. */
export function detailMod(m: ModVortex): string {
  return `${nb(m.fichiers)} ${pluriel(m.fichiers, 'fichier', 'fichiers')}`;
}

/** Le texte du bouton de retrait. */
export function libelleRetrait(e: EtatVortex): string {
  return `🧹 Retirer les mods de Vortex (${nb(e.mods.length)})`;
}

/** Le titre de la fenêtre de confirmation du retrait. */
export function titreRetrait(e: EtatVortex): string {
  const n = e.mods.length;
  return n === 1 ? '🧹 Retirer le mod de Vortex ?' : `🧹 Retirer les ${nb(n)} mods de Vortex ?`;
}

/** Le message de la confirmation : chiffré (fichiers PROUVÉS, taille), ce qui se passe, et Vortex à désinstaller. */
export function messageRetrait(e: EtatVortex, taille: (o: number) => string): string {
  const n = e.fichiers;
  return [
    'Le jeu démarrera sans mods.',
    `Aucun fichier de mod n’est effacé ni copié : ${n === 1 ? 'le fichier' : `les ${nb(n)} fichiers`} (${taille(e.octets)}) ${n === 1 ? 'est rangé' : 'sont rangés'} dans un dossier à part, dans le dossier du jeu, en quelques secondes et sans place en plus sur le disque.`,
    'Tes parties sont mises à l’abri avant.',
    'Tu pourras remettre chaque mod, un par un.',
    'Ensuite, désinstalle Vortex (Paramètres de Windows ▸ Applications ▸ Vortex ▸ Désinstaller) et ne l’ouvre plus d’ici là : il remettrait ses mods.',
  ].join(' ');
}

/** Ce qu'un retrait a fait (après le retrait, puis à chaque ouverture tant que des mods sont rangés). */
export function lignesRetrait(r: { mods: number; laisses: number; laisses_rompus: number; sauvegardes_vortex?: number }): string[] {
  const l = [`✅ ${nb(r.mods)} ${pluriel(r.mods, 'mod retiré', 'mods retirés')}.`];
  const rompus = r.laisses_rompus;
  if (rompus) l.push(rompus === 1 ? '1 fichier laissé tel quel : il ne menait plus nulle part.' : `${nb(rompus)} fichiers laissés tels quels : ils ne menaient plus nulle part.`);
  const autres = r.laisses - rompus;
  if (autres > 0)
    l.push(
      autres === 1
        ? '1 fichier laissé tel quel : Frogtend n’a pas pu prouver qu’il venait de Vortex.'
        : `${nb(autres)} fichiers laissés tels quels : Frogtend n’a pas pu prouver qu’ils venaient de Vortex.`,
    );
  l.push(DESINSTALLER_VORTEX);
  const s = r.sauvegardes_vortex ?? 0;
  if (s > 0)
    l.push(
      s === 1
        ? 'Vortex avait gardé à côté l’original d’un fichier du jeu : Frogtend le laisse tel quel. Si le jeu se plaint d’un fichier, fais vérifier ses fichiers par sa boutique.'
        : `Vortex avait gardé à côté l’original de ${nb(s)} fichiers du jeu : Frogtend les laisse tels quels. Si le jeu se plaint d’un fichier, fais vérifier ses fichiers par sa boutique.`,
    );
  l.push('Les réglages que tes mods avaient écrits restent dans le jeu : ils resserviront si tu les remets.');
  l.push('⚠ Tes parties faites avec des mods peuvent en avoir besoin : si une partie refuse de se charger, remets le mod.');
  return l;
}

/** Le message court après un retrait (toast) : combien, les fichiers laissés, et Vortex à désinstaller. */
export function toastRetrait(r: { mods: number; laisses: number; laisses_rompus: number }): string {
  return lignesRetrait(r)
    .filter((l) => !l.startsWith('Les réglages') && !l.startsWith('⚠') && !l.startsWith('Vortex avait'))
    .join(' ');
}

/** Vortex a reposé des mods après un retrait (il a été rouvert) : à dire clairement. */
export function ligneVortexRevenu(e: EtatVortex): string | null {
  return e.present && e.retires.length > 0
    ? '⚠ Vortex a reposé des mods dans le jeu depuis le retrait : il a été rouvert. Ferme-le, désinstalle-le, puis retire ses mods de nouveau.'
    : null;
}

/** La ligne d'un retrait interrompu. */
export function ligneInterrompu(r: { faits: number; total: number }): string {
  return `⚠ Le retrait des mods a été interrompu (${nb(r.faits)} ${pluriel(r.faits, 'fichier', 'fichiers')} sur ${nb(r.total)})`;
}

/** Le message de la confirmation « ↩ Remettre ». */
export function messageRemise(m: ModVortex, version: string | null): string {
  const base = `« ${m.nom} » revient dans le jeu (${detailMod(m)}).`;
  return version ? `${base} Il ne marche peut-être pas avec la version ${version}.` : `${base} Il ne marche peut-être pas avec cette version du jeu.`;
}

/** 0.227.1 : le titre de la confirmation « 🧹 Retirer » d'un mod remis (partie « 🧩 Dans le jeu »). */
export function titreRetraitMod(m: ModVortex): string {
  return `🧹 Retirer « ${m.nom} » ?`;
}

/** Le message de cette confirmation : ce qui se passe, sans rien effacer. */
export function messageRetraitMod(m: ModVortex): string {
  const n = m.fichiers;
  return [
    `« ${m.nom} » quitte le jeu : ${n === 1 ? 'son fichier est rangé' : `ses ${nb(n)} fichiers sont rangés`} de nouveau à part, dans le dossier du jeu, avec les autres mods retirés. Rien n’est effacé.`,
    'Tes parties sont mises à l’abri avant.',
    'Tu pourras le remettre.',
  ].join(' ');
}

/** Pourquoi des fichiers d'un mod remis sont restés dans le jeu (ou à part), phrase par phrase. */
function causesRestes(b: BilanRetraitModVortex): string[] {
  const l: string[] = [];
  const autres = b.changes + b.partis;
  if (autres)
    l.push(autres === 1 ? '1 fichier n’est plus celui que Frogtend avait remis (changé ou effacé)' : `${nb(autres)} fichiers ne sont plus ceux que Frogtend avait remis (changés ou effacés)`);
  if (b.places_prises) l.push(b.places_prises === 1 ? '1 fichier ne trouve plus sa place à part (déjà prise)' : `${nb(b.places_prises)} fichiers ne trouvent plus leur place à part (déjà prise)`);
  if (b.erreurs)
    l.push(b.erreurs === 1 ? 'Windows a refusé de déplacer 1 fichier (ouvert ou protégé) : réessaie plus tard' : `Windows a refusé de déplacer ${nb(b.erreurs)} fichiers (ouverts ou protégés) : réessaie plus tard`);
  if (b.coinces)
    l.push(b.coinces === 1 ? '1 fichier rangé n’a pas pu être vérifié : il reste proposé à la remise' : `${nb(b.coinces)} fichiers rangés n’ont pas pu être vérifiés : ils restent proposés à la remise`);
  return l;
}

/** Le message après le retrait d'un mod remis : la vérité, rien de plus (resté dans le jeu, en partie, ou retiré). */
export function messageApresRetraitMod(b: BilanRetraitModVortex, nom: string): string {
  const causes = causesRestes(b);
  const ranges = `${nb(b.ranges)} ${pluriel(b.ranges, 'fichier rangé', 'fichiers rangés')}`;
  if (b.ranges === 0 && b.coinces === 0) return `⚠ « ${nom} » est resté dans le jeu : ${causes.length ? causes.join(' ; ') : 'aucun fichier n’a pu être rangé'}.`;
  if (b.changes + b.places_prises + b.erreurs + b.coinces > 0) return `« ${nom} » est en partie retiré (${ranges}) : ${causes.join(' ; ')}.`;
  return `✅ « ${nom} » retiré du jeu (${ranges}).${causes.length ? ` ${causes.join(' ; ')}.` : ''}`;
}

/** Le bilan est-il sans reste (un message « ok ») ? */
export function retraitModComplet(b: BilanRetraitModVortex): boolean {
  return b.ranges > 0 && b.changes + b.places_prises + b.erreurs + b.coinces === 0;
}

/** 🧹 « Tout ranger de nouveau » (sans le dépôt de Vortex) : la confirmation. */
export function messageToutRanger(n: number): string {
  return `${n === 1 ? 'Le mod remis quitte' : `Les ${nb(n)} mods remis quittent`} le jeu : ${n === 1 ? 'ses fichiers sont rangés' : 'leurs fichiers sont rangés'} de nouveau à part, dans le dossier du jeu. Rien n’est effacé. Tes parties sont mises à l’abri avant. Tu pourras les remettre.`;
}

/** Le message après « Tout ranger de nouveau ». */
export function messageApresToutRanger(b: BilanRetraitModVortex): string {
  const causes = causesRestes(b);
  if (b.ranges === 0 && b.coinces === 0) return `⚠ Aucun mod n’a été rangé : ${causes.length ? causes.join(' ; ') : 'aucun fichier n’a pu être rangé'}.`;
  const tete = `${nb(b.mods)} ${pluriel(b.mods, 'mod rangé', 'mods rangés')} de nouveau (${nb(b.ranges)} ${pluriel(b.ranges, 'fichier', 'fichiers')})`;
  return causes.length ? `${tete} : ${causes.join(' ; ')}.` : `✅ ${tete}.`;
}

/** Le message de la confirmation « ↩ Tout remettre comme avant ». */
export function messageToutRemettre(enCours: boolean, n: number, version: string | null): string {
  if (enCours) return 'Les fichiers déjà rangés reviennent dans le jeu, à leur place : le jeu redevient comme avant le retrait. Tes parties sont mises à l’abri avant.';
  const quoi = n === 1 ? 'Le mod retiré revient dans le jeu, à sa place, comme Vortex l’avait posé.' : `Les ${nb(n)} mods retirés reviennent dans le jeu, à leur place, comme Vortex les avait posés.`;
  const il = n === 1 ? 'Il ne marche' : 'Ils ne marchent';
  return `${quoi} ${il} peut-être pas avec ${version ? `la version ${version}` : 'cette version du jeu'}. Tes parties sont mises à l’abri avant.`;
}

/** Le message après une remise (un mod, ou tout). */
export function messageApresRemise(b: BilanRemiseVortex, nom: string | null): string {
  const quoi = nom ? `« ${nom} » remis dans le jeu` : 'Mods remis dans le jeu';
  const l = [`✅ ${quoi} (${nb(b.remis)} ${pluriel(b.remis, 'fichier', 'fichiers')}).`];
  if (b.places_prises) l.push(`${nb(b.places_prises)} ${pluriel(b.places_prises, 'fichier reste rangé : sa place', 'fichiers restent rangés : leur place')} dans le jeu est prise par un autre.`);
  if (b.changes) l.push(`${nb(b.changes)} ${pluriel(b.changes, 'fichier rangé a changé : il', 'fichiers rangés ont changé : ils')} reste${b.changes > 1 ? 'nt' : ''} à part.`);
  const e = b.erreurs ?? 0;
  if (e) l.push(e === 1 ? 'Windows a refusé de déplacer 1 fichier (ouvert ou protégé) : il reste à part, réessaie plus tard.' : `Windows a refusé de déplacer ${nb(e)} fichiers (ouverts ou protégés) : ils restent à part, réessaie plus tard.`);
  return l.join(' ');
}

/** Le motif d'un bloc verrouillé (rien ne peut se faire maintenant). */
export function motifVerrou(v: EtatVortex['verrou'], occupe: boolean): string | null {
  if (v === 'galaxy') return '⛔ Ferme GOG Galaxy d’abord : il pourrait écrire dans le jeu pendant ce temps.';
  if (v === 'vortex') return '⛔ Ferme Vortex d’abord : il remettrait ses mods pendant ce temps.';
  if (v === 'partie') return '⛔ Le jeu est en cours, ou Frogtend y travaille : attends la fin.';
  if (occupe) return '⏳ Une opération est en cours sur ce jeu.';
  return null;
}

/** Après « 🔁 C'est fait, revérifier » : ce qui bloque encore. */
export function encoreOuvert(v: EtatVortex['verrou']): string | null {
  if (v === 'galaxy') return 'GOG Galaxy est encore ouvert : ferme-le (aussi dans la zone de notification, près de l’horloge).';
  if (v === 'vortex') return 'Vortex est encore ouvert : ferme-le (aussi dans la zone de notification, près de l’horloge).';
  if (v === 'partie') return 'Le jeu est encore en cours, ou Frogtend y travaille encore.';
  return null;
}
