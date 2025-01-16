use dioxus::{dioxus_core::SpawnIfAsync, html::FileEngine, prelude::*, web::WebFileEngineExt};
use image::{ImageError, ImageFormat, RgbImage, RgbaImage, error::{ParameterError, ParameterErrorKind}, EncodableLayout};
use std::{future::Future, io::Cursor, sync::Arc};
use image::codecs::png::PngEncoder;
use tracing::{error, info, trace};

fn main() {
    launch(App);
}

#[derive(Clone, Copy)]
pub struct CustomImage {
    image: Signal<Option<RgbImage>>,
}

pub fn use_custom_image() -> CustomImage {
    use_hook(|| CustomImage { image: Signal::new(None) })
}

impl CustomImage {
    async fn read_image(&mut self, event: Event<FormData>) -> Result<(), ImageError> {
        match event.data.files() {
            Some(files) => match files.files().first() {
                Some(name) => {
                    let bytes = files.read_file(name).await.unwrap_or_default();
                    let any = image::load_from_memory(&bytes)?;
                    return Ok(self.image.set(Some(any.to_rgb8())));
                }
                None => {}
            },
            None => {}
        }
        Err(ImageError::Parameter(ParameterError::from_kind(ParameterErrorKind::NoMoreData)))
    }
    fn width(&self) -> usize {
        match self.image.try_read() {
            Ok(a) => match a.as_ref() {
                Some(s) => s.width() as usize,
                None => 0,
            },
            Err(_) => 0,
        }
    }

}

fn base64_image(image: &RgbImage) -> Result<String, ImageError> {
    let png = PngEncoder::new(Cursor::new(Vec::new()))?;


    let base64 = base64::encode();
    format!("data:image/png;base64,{base64}")
}
#[component]
fn App() -> Element {
    let mut image = use_custom_image();
    let width = image.width();

    rsx! {
        div {
            input {
                r#type: "file",
                oninput: move |e| async move {
                    match image.read_image(e).await {
                        Ok(_) => info!("image loaded"),
                        Err(e) => error!("{:?}", e)
                    }
                }
            }
            span {
                "size: {width}"
            }
            //   <img src="data:image/png;base64, iVBORw0KGgoAAAANSUhEUgAAAAUA
            //     AAAFCAYAAACNbyblAAAAHElEQVQI12P4//8/w38GIAXDIBKE0DHxgljNBAAO
            //         9TXL0Y4OHwAAAABJRU5ErkJggg==" alt="Red dot" />
            img { src: "/logos/logo.png" }
        }
    }
}
