use dioxus::prelude::*;
use image::ImageFormat;
use std::io::Cursor;

fn main() {
    dioxus_web::launch(app);
}

fn app(cx: Scope) -> Element {
    let mut images: Vec<String> = Vec::new();

    let select_file = move |_| {
        let file_input = cx.use_ref(|_| None);
        let file_input = file_input.current().clone();
        let images = images.clone();

        move |event| {
            let files = event
                .target()
                .unwrap()
                .dyn_ref::<web_sys::HtmlInputElement>()
                .unwrap()
                .files()
                .unwrap();

            if let Some(file) = files.get(0) {
                let reader = web_sys::FileReader::new().unwrap();
                let images = images.clone();

                let callback = move |_| {
                    let data_url = reader.result().unwrap().as_string().unwrap();
                    images.push(data_url);
                    file_input.set(None);
                    cx.rerender();
                };

                reader.set_onload(Some(callback.as_ref().unchecked_ref()));
                reader.read_as_data_url(&file).unwrap();
            }
        }
    };

    rsx!(cx, div {
        class: "container",
        input(
            type: "file",
            onchange: select_file,
            accept: "image/*",
        )
        div {
            class: "images",
            images.iter().map(|image| {
                rsx!(cx, img {
                    class: "image",
                    src: "{image}",
                    alt: "Uploaded Image"
                })
            })
        }
    })
}