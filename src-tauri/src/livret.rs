//! Le livret d'un jeu (contrat 1.7, document `livret` : un PDF) lu DANS Frogtend : ses pages sont rendues en images
//! par Windows lui-même (`Windows.Data.Pdf`, intégré au système, rien à installer), puis montrées par le lecteur
//! maison — pilotable au clavier et à la manette, ce que ni une visionneuse PDF intégrée ni le lecteur de Windows ne
//! permettent (et ce dernier s'ouvrirait derrière le menu en jeu).

use std::path::Path;

use windows::Data::Pdf::{PdfDocument, PdfPageRenderOptions};
use windows::Storage::StorageFile;
use windows::Storage::Streams::{DataReader, InMemoryRandomAccessStream};

use crate::erreurs::{Erreur, Resultat};

/// La largeur d'une page rendue : bornée (une image de page reste raisonnable, même demandée en très grand).
pub const LARGEUR_MIN: u32 = 200;
pub const LARGEUR_MAX: u32 = 2400;

fn illisible(e: windows::core::Error) -> Erreur {
    Erreur::Disque(format!("Ce livret ne se lit pas ({}).", e.message()))
}

/// Les appels WinRT demandent un fil initialisé (COM) : fait une fois par fil, sans effet s'il l'est déjà.
fn preparer_fil() {
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
    // S_FALSE (déjà fait) ou RPC_E_CHANGED_MODE (fil déjà en STA) conviennent tous deux.
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
}

fn ouvrir(pdf: &Path) -> Resultat<PdfDocument> {
    preparer_fil();
    let chemin = std::path::absolute(pdf)?;
    let fichier = StorageFile::GetFileFromPathAsync(&windows::core::HSTRING::from(chemin.as_os_str()))
        .and_then(|a| a.get())
        .map_err(illisible)?;
    PdfDocument::LoadFromFileAsync(&fichier).and_then(|a| a.get()).map_err(illisible)
}

/// Le nombre de pages du livret.
pub fn pages(pdf: &Path) -> Resultat<u32> {
    ouvrir(pdf)?.PageCount().map_err(illisible)
}

/// La page `n` (à partir de 0) rendue en PNG, à la largeur demandée (bornée), la hauteur suivant la page.
pub fn page_png(pdf: &Path, n: u32, largeur: u32) -> Resultat<Vec<u8>> {
    let doc = ouvrir(pdf)?;
    let total = doc.PageCount().map_err(illisible)?;
    if n >= total {
        return Err(Erreur::Refus(format!("Ce livret n'a que {total} page(s).")));
    }
    let page = doc.GetPage(n).map_err(illisible)?;
    let options = PdfPageRenderOptions::new().map_err(illisible)?;
    options.SetDestinationWidth(largeur.clamp(LARGEUR_MIN, LARGEUR_MAX)).map_err(illisible)?;
    let flux = InMemoryRandomAccessStream::new().map_err(illisible)?;
    page.RenderWithOptionsToStreamAsync(&flux, &options).and_then(|a| a.get()).map_err(illisible)?;
    let taille = flux.Size().map_err(illisible)? as u32;
    let lecteur = DataReader::CreateDataReader(&flux.GetInputStreamAt(0).map_err(illisible)?).map_err(illisible)?;
    lecteur.LoadAsync(taille).and_then(|a| a.get()).map_err(illisible)?;
    let mut octets = vec![0u8; taille as usize];
    lecteur.ReadBytes(&mut octets).map_err(illisible)?;
    Ok(octets)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Un PDF minimal mais complet (table des renvois exacte), de `n` pages A5.
    pub fn pdf_de_test(n: usize) -> Vec<u8> {
        let mut objets: Vec<String> = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".into(),
            format!("<< /Type /Pages /Kids [{}] /Count {n} >>", (0..n).map(|i| format!("{} 0 R", 3 + i)).collect::<Vec<_>>().join(" ")),
        ];
        for _ in 0..n {
            objets.push("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 420 595] >>".into());
        }
        let mut pdf = b"%PDF-1.4\n".to_vec();
        let mut positions = vec![];
        for (i, o) in objets.iter().enumerate() {
            positions.push(pdf.len());
            pdf.extend(format!("{} 0 obj\n{o}\nendobj\n", i + 1).as_bytes());
        }
        let xref = pdf.len();
        pdf.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objets.len() + 1).as_bytes());
        for p in positions {
            pdf.extend(format!("{p:010} 00000 n \n").as_bytes());
        }
        pdf.extend(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objets.len() + 1).as_bytes());
        pdf
    }

    #[test]
    fn les_pages_d_un_livret_se_rendent_en_images() {
        let d = tempfile::tempdir().unwrap();
        let pdf = d.path().join("livret.pdf");
        std::fs::write(&pdf, pdf_de_test(3)).unwrap();
        assert_eq!(pages(&pdf).unwrap(), 3);
        let png = page_png(&pdf, 1, 600).unwrap();
        assert!(png.starts_with(b"\x89PNG"), "une image PNG");
        // Largeur demandée = largeur rendue (octets 16..20 de l'en-tête IHDR) ; la hauteur suit la page A5.
        let l = u32::from_be_bytes(png[16..20].try_into().unwrap());
        let h = u32::from_be_bytes(png[20..24].try_into().unwrap());
        assert_eq!(l, 600);
        assert!((845..=852).contains(&h), "{h}");
        // Bornes : une largeur folle est ramenée au plafond ; une page hors du livret est refusée.
        let grand = page_png(&pdf, 0, 100_000).unwrap();
        assert_eq!(u32::from_be_bytes(grand[16..20].try_into().unwrap()), LARGEUR_MAX);
        assert!(page_png(&pdf, 3, 600).is_err());
        // Un fichier qui n'est pas un PDF : une erreur, jamais un plantage.
        let faux = d.path().join("faux.pdf");
        std::fs::write(&faux, b"pas un pdf").unwrap();
        assert!(pages(&faux).is_err());
    }
}
