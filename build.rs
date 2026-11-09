use std::{env, fs, path::Path, process::Command};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let po_dir = Path::new("po");
    let text_domain = "io.github.pisekpiskovec.AlbumCard";

    // Tell Cargo to re-run the script fi any .po files change
    println!("cargo:rerun-if-changed=po");

    if po_dir.exists() {
        for entry in fs::read_dir(po_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("po") {
                let lang = path.file_stem().unwrap().to_str().unwrap();

                let dest_dir = Path::new(&out_dir)
                    .join("locale")
                    .join(lang)
                    .join("LC_MESSAGES");
                fs::create_dir_all(&dest_dir).unwrap();

                let dest_file = dest_dir.join(format!("{}.mo", text_domain));

                // Compile .po to .mo using host's msgfmt tool
                let status = Command::new("msgfmt")
                    .arg(&path)
                    .arg("-o")
                    .arg(&dest_file)
                    .status();

                match status {
                    Ok(s) if s.success() => {}
                    _ => {
                        eprintln!("Warning: 'msgfmt' failed or is not installed. Translation compilation for '{}' skipped.", lang);
                    }
                }
            }
        }
    }

    // Expose envirnoment variables to Rust code at compile time
    println!("cargo:rustc-env=LOCALEDIR={}/locale", out_dir);
    println!("cargo:rustc-env=GETTEXT_PACKAGE={}", text_domain);
}
