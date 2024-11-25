//! Display the current state of the system's mimeapps.lists in a
//! mimeapps.list format.

fn main() {
    let mut list = cosmic_mime_apps::List::default();
    list.load_from_paths(&cosmic_mime_apps::list_paths());
    println!("{}", list.to_string());
}
