//! Display known desktop entries with mime type associations.

fn main() {
    let mut list = cosmic_mime_apps::List::default();
    list.load_from_paths(&cosmic_mime_apps::list_paths());
    println!("{:#?}", cosmic_mime_apps::associations::by_app(&list))
}
