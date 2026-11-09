use gettextrs::{LocaleCategory, bind_textdomain_codeset, bindtextdomain, setlocale, textdomain};

pub fn init() {
    unsafe { setlocale(LocaleCategory::LcAll, "") };

    let package = env!("GETTEXT_PACKAGE");
    let localedir = env!("LOCALEDIR");
    bindtextdomain(package, localedir).expect("Failed to bind text domain");
    bind_textdomain_codeset(package, "UTF-8").expect("Failed to set text domain codeset");
    textdomain(package).expect("Failed to set text domain");
}
