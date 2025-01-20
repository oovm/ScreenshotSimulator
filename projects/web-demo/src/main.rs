use base64::{Engine, engine::general_purpose::STANDARD};
use cint::EncodedSrgb;
use dioxus::{
    dioxus_core::{SpawnIfAsync, internal::generational_box::GenerationalRef},
    html::FileEngine,
    prelude::*,
    web::WebFileEngineExt,
};
use image::{
    EncodableLayout, ExtendedColorType, GenericImage, ImageEncoder, ImageError, ImageFormat, Rgb, RgbImage, RgbaImage,
    codecs::png::PngEncoder,
    error::{ParameterError, ParameterErrorKind},
};
use std::{
    cell::{Ref, RefCell},
    future::Future,
    io::Cursor,
    num::ParseIntError,
    rc::Rc,
    sync::Arc,
};
use tracing::{error, info, trace};

fn main() {
    launch(App);
}

#[derive(Clone)]
pub struct IOS18ScreenShot {
    source: Rc<RefCell<RgbImage>>,
    iterations: Rc<RefCell<usize>>,
    results: Signal<Vec<RgbImage>>,
    animation: Signal<Option<RgbImage>>,
}

pub fn use_custom_image() -> IOS18ScreenShot {
    use_hook(|| IOS18ScreenShot {
        iterations: Rc::default(),
        source: Rc::default(),
        results: Signal::new(vec![]),
        animation: Signal::new(None),
    })
}

impl IOS18ScreenShot {
    async fn read_image(&self, event: Event<FormData>) -> Result<(), ImageError> {
        // self.results.set(Vec::with_capacity(self.get_iteration()));
        match event.data.files() {
            Some(files) => match files.files().first() {
                Some(name) => {
                    let bytes = files.read_file(name).await.unwrap_or_default();
                    let any = image::load_from_memory(&bytes)?;
                    return Ok(*self.source.borrow_mut() = any.to_rgb8());
                }
                None => {}
            },
            None => {}
        }
        Err(ImageError::Parameter(ParameterError::from_kind(ParameterErrorKind::NoMoreData)))
    }
    fn convert_pixel(&self, pixel: [u8; 3]) -> [u8; 3] {
        let p3_input = cint::EncodedDisplayP3 { r: pixel[0], g: pixel[1], b: pixel[2] };
        let srgb_out: EncodedSrgb = p3_input.into_cint();
        [srgb_out.r, srgb_out.g, srgb_out.b]
    }
    pub unsafe fn convert_image(&self, srgb: &RgbImage) -> RgbImage {
        let mut p3_out = RgbImage::new(srgb.width(), srgb.height());
        for (x, y, c) in srgb.enumerate_pixels() {
            let px = self.convert_pixel(c.0);
            p3_out.unsafe_put_pixel(x, y, Rgb(px));
        }
        p3_out
    }
    fn width(&self) -> usize {
        self.source.borrow().width() as usize
    }
    fn source_base64(&self) -> Result<String, ImageError> {
        base64_image(&*self.source.borrow())
    }
    pub fn get_iteration(&self) -> usize {
        *self.iterations.borrow()
    }
    pub fn set_iteration(&self, e: Event<FormData>) {
        let value = e.data.value();
        match value.parse::<usize>() {
            Ok(o) => *self.iterations.borrow_mut() = o,
            Err(_) => {}
        }
    }
}

fn base64_image(image: &RgbImage) -> Result<String, ImageError> {
    let mut buffer = Vec::new();
    let png = PngEncoder::new(&mut buffer);
    png.write_image(image.as_bytes(), image.width(), image.height(), ExtendedColorType::Rgb8)?;
    let base64 = STANDARD.encode(buffer);
    Ok(format!("data:image/png;base64,{base64}"))
}
#[component]
fn App() -> Element {
    let image = use_custom_image();
    let width = image.get_iteration();
    let source_base64 = image.source_base64().unwrap_or_default();

    rsx! {
        div {
            input {
                r#type: "file",
                oninput: move |e| {
                    let value = image.clone();
                    async move {

                    match value.read_image(e).await {
                        Ok(_) => info!("image loaded"),
                        Err(e) => error!("{:?}", e)

                        }
                    }
                }
            }
            span {
                "iteration: {width}"
            }
            // input {
            //     r#type: "number",
            //     value: "{image.get_iteration()}",
            //     oninput: move |e| {
            //         image.set_iteration(e)
            //     }
            // }
            img { src: source_base64 }
        }
    }
}
