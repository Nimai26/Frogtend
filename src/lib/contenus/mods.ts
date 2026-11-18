// 🧩 Les mods Switch servis par Firehouse (contrat 1.44, ✅ Seb 18/11) : ce que la personne voit sur la fiche d'un jeu
// (bureau) et dans « Réglages du jeu » (Taodbox). Fonctions pures, partagées par les deux écrans. Jamais de jargon :
// ni « exefs », ni « romfs », ni « build id », ni « clé ».

import { duree, taille, type ModSwitchVu, type ModsSwitch, type SuiviModsSwitch } from '$lib/api';

const s = (n: number) => (n > 1 ? 's' : '');

/** Le mod est-il rangé sur ce PC (donc allumable) ? Un mod grisé peut l'être aussi (rangé avant un changement de
 * version) : il garde alors son état, mais jamais d'interrupteur. */
export function estInstalle(m: ModSwitchVu): boolean {
  return m.etat === 'telecharge' || m.etat === 'revision_dispo' || (m.etat === 'incompatible' && m.revision_rangee != null);
}

/** Les mods qui peuvent avoir un interrupteur : rangés et pas grisés. */
export function aUnInterrupteur(m: ModSwitchVu): boolean {
  return m.etat === 'telecharge' || m.etat === 'revision_dispo';
}

/** Les mods à télécharger pour la première fois (jamais un mod grisé : il ne servirait à rien). */
export function aTelecharger(l: ModSwitchVu[]): ModSwitchVu[] {
  return l.filter((m) => m.etat === 'pas_telecharge' && m.servi);
}

/** Les mods déjà rangés dont Firehouse sert une nouvelle révision. */
export function aMettreAJour(l: ModSwitchVu[]): ModSwitchVu[] {
  return l.filter((m) => m.etat === 'revision_dispo' && m.servi);
}

/** La forme que le cœur attend pour l'accord : ce que la personne a VU (clé, révision, taille, empreinte ; le cœur
 * refuse si ça a changé). */
export function clesVues(l: ModSwitchVu[]): { cle: string; revision: number; taille: number; sha256: string }[] {
  return l.map((m) => ({ cle: m.cle, revision: m.revision, taille: m.taille, sha256: m.sha256 }));
}

// Les mots techniques que Firehouse ou les émulateurs peuvent écrire (raison, réglage requis), dits simplement. Les
// plus longs d'abord (« build id » avant « id »…), sans casse, en mots entiers.
const MOTS_TECHNIQUES: [RegExp, string][] = [
  [/\bbuild[\s_-]?ids?\b/gi, 'repère de version'],
  [/\bexefs\b/gi, 'programme du jeu'],
  [/\bromfs\b/gi, 'fichiers du jeu'],
  [/\bnso\b/gi, 'programme du jeu'],
  [/\bpchtxt\b/gi, 'correctif'],
  [/\bips\b/gi, 'correctif'],
  [/\bcheats\b/gi, 'triches'],
  [/\bzip\b/gi, 'archive'],
];

/** Un texte venu de Firehouse (raison, réglage requis), gardé tel quel mais sans jargon, avec sa majuscule. */
export function texteSimple(t: string | null | undefined, avecMajuscule = true): string {
  let r = (t ?? '').trim();
  for (const [motif, simple] of MOTS_TECHNIQUES) r = r.replace(motif, simple);
  return avecMajuscule ? majuscule(r) : r;
}

/** La première lettre en majuscule (« fait pour la version… » → « Fait pour la version… »). */
export function majuscule(t: string): string {
  return t ? t.charAt(0).toLocaleUpperCase('fr-FR') + t.slice(1) : t;
}

export function totalOctets(l: ModSwitchVu[]): number {
  return l.reduce((n, m) => n + m.taille, 0);
}

/** Le résumé du bouton repliable : « 2 allumés sur 3, 1 à télécharger, 1 mise à jour ». */
export function resumeMods(l: ModSwitchVu[]): string {
  if (!l.length) return 'aucun';
  const installes = l.filter(aUnInterrupteur);
  const allumes = installes.filter((m) => m.allume).length;
  const morceaux: string[] = [];
  if (installes.length) morceaux.push(`${allumes} allumé${s(allumes)} sur ${installes.length}`);
  const nouveaux = aTelecharger(l).length;
  if (nouveaux) morceaux.push(`${nouveaux} à télécharger`);
  const majs = aMettreAJour(l).length;
  if (majs) morceaux.push(`${majs} mise${s(majs)} à jour`);
  const grises = l.filter((m) => m.etat === 'incompatible').length;
  if (grises) morceaux.push(`${grises} ${grises > 1 ? 'ne vont' : 'ne va'} pas avec ta version`);
  return morceaux.join(', ') || `${l.length} mod${s(l.length)}`;
}

/** La ligne grise sous le nom : « par Wollnashorn · licence : MIT » (« aucune » si elle n'est pas dite). */
export function ligneAuteur(m: ModSwitchVu): string {
  const morceaux: string[] = [];
  if (m.auteur.trim()) morceaux.push(`par ${m.auteur.trim()}`);
  morceaux.push(`licence : ${m.licence.trim() || 'aucune'}`);
  return morceaux.join(' · ');
}

/** Les mentions d'un mod, en mots simples (une par ligne). */
export function mentions(m: ModSwitchVu): string[] {
  const l: string[] = [];
  if (m.reglage_requis.trim())
    l.push(`⚙ Il lui faut un réglage de l’émulateur : ${texteSimple(m.reglage_requis, false).replace(/[.\s]+$/, '')}. Tant que ce réglage n’est pas fait, Frogtend ne le met pas en place.`);
  if (m.touche_sauvegardes) l.push('💾 Il touche à tes parties : elles seront mises à l’abri avant.');
  if (m.compat === 'pas_verifie') l.push('Version du jeu pas vérifiée : il peut ne pas marcher.');
  if (!m.servi) l.push('Firehouse ne le propose plus : il reste sur ce PC tant que tu ne le retires pas.');
  return l;
}

/** Une description assez longue pour être repliée à deux lignes (avec « Lire plus »). */
export function descriptionLongue(d: string): boolean {
  return d.trim().length > 120 || d.trim().includes('\n');
}

/** La mention discrète : personne ne l'a encore essayé dans un émulateur. */
export function pasEncoreEssaye(m: ModSwitchVu): boolean {
  return m.servi && !m.emulateurs_essayes.length;
}

type DernierePose = ModsSwitch['derniere_pose'];

/** Allumé, mais pas mis en place à la dernière partie (la pose l'a dit : « « nom » pas posé … »). */
export function allumePasPose(m: ModSwitchVu, dp: DernierePose | undefined): boolean {
  return m.allume && !!dp?.raisons.some((r) => r.startsWith(`« ${m.nom} » pas posé`));
}

/** Le texte de l'interrupteur (jamais la couleur seule) ; un mod allumé que la dernière partie n'a pas mis en place
 * se voit. */
export function texteInterrupteurMod(m: ModSwitchVu, dp?: DernierePose): string {
  if (allumePasPose(m, dp)) return '⚠ Allumé, pas utilisé';
  return m.allume ? '✅ Allumé' : '⬜ Éteint';
}

/** « À la dernière partie (Eden, le 18/11 à 14:05) : », puis une ligne par raison ; rien si tout a été posé. */
export function lignesDernierePose(dp: DernierePose | undefined): string[] {
  if (!dp?.raisons.length) return [];
  const quand = dateVue(dp.quand);
  return [
    `⚠ À la dernière partie (${[dp.emulateur, quand].filter(Boolean).join(', ')}) :`,
    ...dp.raisons.slice(0, 8).map((r) => `• ${texteSimple(r)}`),
    ...(dp.raisons.length > 8 ? [`… et ${dp.raisons.length - 8} autre(s).`] : []),
  ];
}

/** La raison d'un mod grisé, dite simplement (jamais vide). */
export function raisonGrise(m: ModSwitchVu): string {
  return texteSimple(m.raison) || 'Il ne va pas avec la version du jeu utilisée sur ce PC.';
}

/** Le mod allumé avec lequel `cle` ne va pas (dans un sens ou dans l'autre), ou `null`. Même règle que le cœur
 * (`mods_switch::conflit`) : seuls les mods rangés et allumés comptent. */
export function conflitAvec(l: ModSwitchVu[], cle: string): ModSwitchVu | null {
  const a = l.find((m) => m.cle === cle);
  if (!a) return null;
  return l.find((b) => b.cle !== cle && b.allume && estInstalle(b) && (a.conflits.includes(b.cle) || b.conflits.includes(a.cle))) ?? null;
}

/** « environ 3 min » si le débit de ce PC est connu ; sinon rien (jamais une durée inventée). */
export function dureeEstimee(octets: number, debit: number | null | undefined): string | null {
  return debit && debit > 0 ? `environ ${duree(octets / debit)}` : null;
}

/** Le titre de l'accord : combien, combien de Mo, combien de temps. */
export function titreTelechargement(l: ModSwitchVu[], debit: number | null | undefined, miseAJour = false): string {
  const total = totalOctets(l);
  const d = dureeEstimee(total, debit);
  const chiffres = `${taille(total)}${d ? `, ${d}` : ''}`;
  if (l.length === 1) return `🧩 ${miseAJour ? 'Mettre à jour' : 'Télécharger'} « ${l[0].nom} » (${chiffres}) ?`;
  return `🧩 ${miseAJour ? 'Mettre à jour' : 'Télécharger'} ${l.length} mods (${chiffres}) ?`;
}

/** Le texte de l'accord (une ligne par paragraphe). */
export function messageTelechargement(l: ModSwitchVu[], miseAJour = false): string {
  const lignes: string[] = [];
  if (l.length > 1) {
    lignes.push(...l.slice(0, 8).map((m) => `• ${m.nom} (${taille(m.taille)})`));
    if (l.length > 8) lignes.push(`… et ${l.length - 8} autre(s).`);
  }
  const plus = l.length > 1;
  lignes.push(
    plus
      ? 'Ils viennent de Firehouse, qui les a vérifiés. Ils valent pour toute la maison, avec Eden comme avec Ryubing.'
      : 'Il vient de Firehouse, qui l’a vérifié. Il vaut pour toute la maison, avec Eden comme avec Ryubing.',
  );
  if (miseAJour) lignes.push('L’ancienne version reste en place tant que la nouvelle n’est pas vérifiée.');
  else lignes.push(plus ? 'Une fois téléchargés, chacun s’allume ou s’éteint d’un simple bouton.' : 'Une fois téléchargé, il s’allume ou s’éteint d’un simple bouton.');
  const touchent = l.filter((m) => m.touche_sauvegardes);
  if (touchent.length)
    lignes.push(
      plus
        ? `💾 ${touchent.map((m) => `« ${m.nom} »`).join(', ')} touche${touchent.length > 1 ? 'nt' : ''} à tes parties : Frogtend les met à l’abri avant la prochaine partie.`
        : '💾 Il touche à tes parties : Frogtend les met à l’abri avant la prochaine partie.',
    );
  lignes.push('Le téléchargement se fait en arrière-plan ; il s’arrête si une partie commence.');
  return lignes.join('\n');
}

/** « ⬇ « 1080p » : 45 % (1 sur 3) » — la ligne visible pendant un téléchargement. */
export function ligneProgresMods(su: SuiviModsSwitch | null | undefined): string | null {
  if (!su?.actif || !su.en_cours) return su?.actif ? '⬇ Téléchargement des mods…' : null;
  const pourcent = su.total > 0 ? Math.min(100, Math.floor((su.recus * 100) / su.total)) : 0;
  const rang = su.a_faire > 1 ? ` (${Math.min(su.faits + 1, su.a_faire)} sur ${su.a_faire})` : '';
  return `⬇ « ${su.en_cours} » : ${pourcent} %${rang}`;
}

/** Le pourcentage seul du mod en cours (sur sa ligne). */
export function pourcentEnCours(su: SuiviModsSwitch | null | undefined): number {
  return su?.actif && su.total > 0 ? Math.min(100, Math.floor((su.recus * 100) / su.total)) : 0;
}

/** Ce qu'annonce le lecteur d'écran : l'ÉTAPE (le mod en cours), jamais le pourcentage qui bouge toutes les 2 s. */
export function etapeModsAnnoncee(su: SuiviModsSwitch | null | undefined): string {
  if (!su?.actif) return '';
  if (!su.en_cours) return 'Téléchargement des mods en préparation.';
  return `Téléchargement du mod « ${su.en_cours} »${su.a_faire > 1 ? `, ${Math.min(su.faits + 1, su.a_faire)} sur ${su.a_faire}` : ''}.`;
}

/** Le compte rendu de la fin (erreurs, arrêt) : une ligne par souci ; vide si tout s'est bien passé. */
export function lignesFinMods(su: SuiviModsSwitch | null | undefined): string[] {
  if (!su || su.actif) return [];
  // Les messages du cœur passent par `texteSimple` (jamais « exefs », « zip »… à l'écran).
  const l = su.erreurs.slice(0, 5).map((e) => `⚠ Pas téléchargé : ${texteSimple(e, false)}`);
  if (su.erreurs.length > 5) l.push(`… et ${su.erreurs.length - 5} autre(s) souci(s).`);
  if (su.arret) l.push(`⏸ Arrêté : ${texteSimple(su.arret, false)}.`);
  return l;
}

/** Ce que la manette sait viser dans une ligne de mod : un bouton actif ou un interrupteur (jamais la ligne elle-même,
 * `tabindex="-1"`, que la manette ignore). */
export const COMMANDES_DE_LIGNE = 'button:not([disabled]), [role="switch"]:not([aria-disabled="true"])';

/** Après Télécharger ou Retirer : où rendre le focus — la première commande active de la ligne du mod `cle`, sinon
 * d'une ligne qui la suivait (`ordre` : les clés quand l'action a commencé), sinon l'en-tête du bloc. */
export function cibleDuFocus(lignes: HTMLElement[], cle: string, ordre: string[], entete: HTMLElement | null): HTMLElement | null {
  const avant = ordre.indexOf(cle);
  const rang = (l: HTMLElement) => {
    const r = ordre.indexOf(l.dataset.cle ?? '');
    return r < 0 ? Number.MAX_SAFE_INTEGER : r;
  };
  const ici = lignes.filter((l) => l.dataset.cle === cle);
  const suivantes = lignes.filter((l) => l.dataset.cle !== cle && rang(l) > avant).sort((a, b) => rang(a) - rang(b));
  for (const l of [...ici, ...suivantes]) {
    const c = l.querySelector<HTMLElement>(COMMANDES_DE_LIGNE);
    if (c) return c;
  }
  return entete;
}

/** « le 18/11 à 14:05 » depuis des secondes (forme du cœur) ; un autre texte est rendu tel quel. */
export function dateVue(vueLe: string | null | undefined): string | null {
  if (!vueLe) return null;
  const n = Number(vueLe);
  if (!Number.isFinite(n) || n <= 0) return vueLe;
  const d = new Date(n * 1000);
  const p = (x: number) => String(x).padStart(2, '0');
  return `le ${p(d.getDate())}/${p(d.getMonth() + 1)} à ${p(d.getHours())}:${p(d.getMinutes())}`;
}

/** Ce qu'il faut dire de la liste elle-même (hors ligne, injoignable), ou `null`. */
export function ligneEtatListe(l: ModsSwitch | null | undefined): string | null {
  if (!l) return null;
  if (l.etat === 'hors_ligne') {
    const d = dateVue(l.vue_le);
    return `📴 Firehouse ne répond pas : voici la liste vue ${d ?? 'la dernière fois'}. Rien ne peut être téléchargé pour l’instant.`;
  }
  if (l.etat === 'injoignable') return '📴 Firehouse ne répond pas : la liste des mods n’a pas pu être lue.';
  return null;
}

/** Le bloc des mods se montre-t-il ? S'il y a des mods, ou quelque chose à dire (Firehouse injoignable, une remarque,
 * ce que la dernière partie a dit). */
export function blocModsVisible(l: ModsSwitch | null | undefined): boolean {
  if (!l || l.etat === 'rien') return false;
  return l.mods.length > 0 || l.etat === 'injoignable' || !!l.remarque?.trim() || !!l.derniere_pose?.raisons.length;
}

/** À la fin d'un téléchargement : le toast (combien sont prêts) ; `null` s'il n'y a rien de prêt. */
export function messageFinMods(su: SuiviModsSwitch | null | undefined): string | null {
  if (!su || su.actif || !su.faits) return null;
  return `🧩 ${su.faits} mod${s(su.faits)} prêt${s(su.faits)} : ${su.faits > 1 ? 'ils s’allument' : 'il s’allume'} d’un simple bouton.`;
}

/** Une partie refusée parce que des mods de ce jeu arrivent (sorte « refus » du cœur) ? */
export function refusModsEnRoute(e: unknown): boolean {
  return typeof e === 'object' && e !== null && (e as { sorte?: unknown }).sorte === 'refus' && /mods de ce jeu sont en train d.arriver/i.test(String((e as { motif?: unknown }).motif ?? ''));
}

/** Le message de ce refus ; au canapé, où le suivre (il n'y a pas de fiche du jeu à l'écran). */
export function messageModsEnRoute(motif: string, canape: boolean): string {
  return `⏳ ${motif}${canape ? ' Au canapé : Y, puis « 🧩 Mises à jour, DLC et mods ».' : ''}`;
}
