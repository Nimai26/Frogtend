// Le pack MAME (contrat 1.33 ; ✅ Seb 27/10) : tout le romset de Firehouse dans un dossier à lui, annoncé et chiffré
// AVANT tout accord (combien de fichiers, combien de Go, combien de temps, la place libre).

export interface AnnoncePack {
  version: string;
  a_recevoir: number;
  octets: number;
  deja_la: number;
  /** Fichiers du pack d'une autre version : la mise à jour les remplace après accord (les anciens gardés à l'abri). */
  anciens: number;
  /** Ce que coûte leur mise à jour (les anciens restent sur le disque). */
  octets_maj: number;
  /** La place que les anciens garderont, à l'abri dans le pack. */
  octets_anciens: number;
  /** Fichiers du pack que Firehouse ne sert plus à ce profil : jamais déplacés, seulement dits. */
  retires: number;
  /** Une réception du pack est déjà en cours sur ce PC. */
  en_cours: boolean;
  libre: number | null;
  chd_fichiers: number;
  chd_octets: number;
}

const GO = 1024 ** 3;
const go = (o: number) => `${(o / GO).toLocaleString('fr-FR', { maximumFractionDigits: 1 })} Go`;

/** Assez de place pour tout, mise à jour comprise (avec 1 Go de marge) ? Place inconnue : on ne bloque pas. */
export function placeSuffisante(a: AnnoncePack): boolean {
  return a.libre === null || a.libre >= a.octets + a.octets_maj + GO;
}

/** Quelque chose à faire : des fichiers à recevoir, ou à mettre à jour. */
export const aFaire = (a: AnnoncePack) => a.a_recevoir > 0 || a.anciens > 0;

/** L'annonce lisible : combien, combien de Go, combien de temps (au débit mesuré sur ce PC), la place. */
export function texteAnnoncePack(a: AnnoncePack, debit: number | null, duree: (s: number) => string): string {
  const retires = a.retires
    ? `${a.retires.toLocaleString('fr-FR')} fichier(s) de ton pack ne sont plus fournis par Firehouse : ils restent tels quels, rien n’est supprimé.`
    : '';
  if (!aFaire(a)) return [`Le pack MAME ${a.version} est complet dans ce dossier (${a.deja_la} fichier(s)).`, retires].filter(Boolean).join('\n');
  const total = a.octets + a.octets_maj;
  const lignes = [
    a.a_recevoir
      ? `${a.a_recevoir.toLocaleString('fr-FR')} fichier(s) à recevoir, ${go(a.octets)} (MAME ${a.version})${a.deja_la ? ` ; ${a.deja_la.toLocaleString('fr-FR')} déjà là` : ''}.`
      : '',
    a.anciens
      ? `Mise à jour vers MAME ${a.version} : ${a.anciens.toLocaleString('fr-FR')} fichier(s) ont changé, ${go(a.octets_maj)} à recevoir. Les anciens ne sont pas effacés : ils sont gardés à part (${go(a.octets_anciens)}, dossier « .frogtend-ancien » dans ton dossier MAME) ; tu pourras supprimer ce dossier quand tout marche.`
      : '',
    debit ? `Durée estimée à ta vitesse habituelle : ${duree(Math.round(total / debit))}.` : 'Durée : inconnue (Frogtend n’a pas encore mesuré ta connexion).',
    a.libre === null ? '' : `Place libre sur ce disque : ${go(a.libre)}${placeSuffisante(a) ? '' : ' — pas assez !'}.`,
    retires,
  ];
  return lignes.filter(Boolean).join('\n');
}

/** MAME et son romset doivent être de la même version (un romset 0.289 dans un MAME 0.288 : des jeux qui ne démarrent
 * pas). Rend l'avis à montrer, ou rien quand tout va. `mame` : le MAME installé (s'il y en a un). */
export function avisVersionMame(mame: { version: string | null; par_frogtend: boolean } | undefined, attendue: string): string | null {
  if (!attendue) return null;
  if (!mame) return `MAME n’est pas encore installé : Frogtend l’installera (version ${attendue}) au premier jeu, après ton accord.`;
  if (!mame.version) return `Ton MAME a été installé à la main (version inconnue) : le pack est fait pour MAME ${attendue}.`;
  if (mame.version === attendue) return null;
  return mame.par_frogtend
    ? `Ton MAME est en version ${mame.version}, le pack en ${attendue} : mets MAME à jour (⚙ Options ▸ Émulateurs ▸ MAME), sinon certains jeux ne démarreront pas.`
    : `Ton MAME est en version ${mame.version}, le pack en ${attendue} : certains jeux ne démarreront pas tant que MAME n’est pas en ${attendue}.`;
}
