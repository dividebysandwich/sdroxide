//! Save a text file from the UI. Native pops a "Save As" dialog; wasm
//! triggers a browser download via a Blob + anchor click.

/// Save `data` under a suggested `name`.
#[cfg(not(target_arch = "wasm32"))]
pub fn save(name: &str, data: &[u8]) {
    let data = data.to_vec();
    let name = name.to_string();
    // rfd's dialog is blocking; run it off the UI thread.
    std::thread::spawn(move || {
        if let Some(path) = rfd::FileDialog::new().set_file_name(&name).save_file() {
            if let Err(e) = std::fs::write(&path, &data) {
                eprintln!("sdroxide: saving {}: {e}", path.display());
            }
        }
    });
}

#[cfg(target_arch = "wasm32")]
pub fn save(name: &str, data: &[u8]) {
    use wasm_bindgen::JsCast;

    let array = js_sys::Uint8Array::from(data);
    let parts = js_sys::Array::new();
    parts.push(&array.buffer());
    let mut opts = web_sys::BlobPropertyBag::new();
    opts.type_("text/plain");
    let blob = match web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &opts) {
        Ok(b) => b,
        Err(_) => return,
    };
    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else { return };

    let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return };
    if let Ok(a) = doc.create_element("a") {
        let a: web_sys::HtmlAnchorElement = a.unchecked_into();
        a.set_href(&url);
        a.set_download(name);
        a.click();
    }
    let _ = web_sys::Url::revoke_object_url(&url);
}
