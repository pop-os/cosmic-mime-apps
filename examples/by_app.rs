//! Display known desktop entries with mime type associations.

fn main() {
    println!("{:#?}", cosmic_mime_apps::associations::by_app())
}
