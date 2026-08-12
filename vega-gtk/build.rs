use std::path::Path;
use std::process::Command;

const DOMAIN: &str = "vega-gtk";
const LOCALES: &[(&str, &str)] = &[
    ("en-US", "en_US"),
    ("pt-BR", "pt_BR"),
    ("es-ES", "es_ES"),
    ("zh-CN", "zh_CN"),
];

fn main() {
    println!("cargo:rerun-if-changed=po");

    let po_dir = Path::new("po");
    if Command::new("msgfmt").arg("--version").output().is_err() {
        panic!("msgfmt is required to build the four vega-gtk translation catalogs");
    }

    for (catalog, locale_dir) in LOCALES {
        let po_path = po_dir.join(format!("{catalog}.po"));
        assert!(
            po_path.is_file(),
            "missing required catalog: {}",
            po_path.display()
        );

        // "locale/<lang>/LC_MESSAGES/" espelha o layout que o pacote instala
        // em /usr/share/locale — é o que `TextDomain::prepend` espera achar
        // dentro do diretório que a gente passa (ele sempre soma "locale").
        let out_dir = po_dir.join("locale").join(locale_dir).join("LC_MESSAGES");
        if let Err(error) = std::fs::create_dir_all(&out_dir) {
            println!(
                "cargo:warning=não foi possível criar {}: {error}",
                out_dir.display()
            );
            continue;
        }

        let mo_path = out_dir.join(format!("{DOMAIN}.mo"));
        match Command::new("msgfmt")
            .arg("-o")
            .arg(&mo_path)
            .arg(&po_path)
            .status()
        {
            Ok(status) if status.success() => {}
            _ => panic!("failed to compile {}", po_path.display()),
        }

        // A second, English-only domain in every supported locale provides
        // deterministic per-key fallback when the active catalog is damaged
        // or incomplete. GNU gettext otherwise returns the Portuguese msgid.
        let fallback_path = out_dir.join(format!("{DOMAIN}-fallback.mo"));
        let english_po = po_dir.join("en-US.po");
        let status = Command::new("msgfmt")
            .arg("-o")
            .arg(&fallback_path)
            .arg(&english_po)
            .status()
            .expect("failed to run msgfmt for English fallback catalog");
        assert!(
            status.success(),
            "failed to compile English fallback catalog"
        );
    }
}
