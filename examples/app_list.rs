//! Display known desktop entries with mime type associations.

fn main() {
    let assocs = cosmic_mime_apps::associations::by_app();

    for mime in cosmic_mime_apps::configured_mime_types(&assocs) {
        println!("{mime}:");

        for (name, _app) in cosmic_mime_apps::apps_for_mime(&mime, &assocs) {
            println!("  {name}");
        }
    }
}
