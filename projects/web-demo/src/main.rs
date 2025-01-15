use dioxus::{dioxus_core::SpawnIfAsync, html::FileEngine, prelude::*, web::WebFileEngineExt};
use image::{ImageError, ImageFormat, RgbImage, RgbaImage};
use std::{future::Future, io::Cursor, sync::Arc};
use tracing::info;

fn main() {
    launch(App);
}

#[component]
fn App() -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        input {
            r#type: "file",
             oninput: |e| async move {
                let buff = read_image_buffer(e).await
             }
        }
        // input {
        //     r#type: "text",
        //     oninput: move |evt| {
        //         info!("text input {:?}", evt);
        //     }
        // }
    }
}

async fn read_image(event: Event<FormData>) -> Result<RgbImage, ImageError> {
    let bytes = read_image_buffer(event).await.unwrap_or_default();
    let image = image::load_from_memory(&bytes)?;
    Ok(image.into_rgb8())
}


async fn read_image_buffer(event: Event<FormData>) -> Option<Vec<u8>> {
    let files = event.data.files()?;
    let name = files.files().first()?;
    files.read_file(name).await
}
