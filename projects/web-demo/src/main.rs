use base64::{Engine, engine::general_purpose::STANDARD};

use dioxus::{
    dioxus_core::{SpawnIfAsync, internal::generational_box::GenerationalRef},
    html::FileEngine,
    prelude::*,
    web::WebFileEngineExt,
};
use image::{
    EncodableLayout, ExtendedColorType, GenericImage, ImageEncoder, ImageError, ImageFormat, ImageResult, Rgb, RgbImage,
    RgbaImage,
    codecs::png::PngEncoder,
    error::{ParameterError, ParameterErrorKind},
};
use rcms::link::link;
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
    pipeline: rcms::pipeline::Pipeline,
    source: Rc<RefCell<RgbImage>>,
    iterations: Rc<RefCell<usize>>,
    frames: Rc<RefCell<Vec<RgbImage>>>,
    animation: Rc<RefCell<Option<RgbImage>>>,
}

pub fn use_custom_image() -> IOS18ScreenShot {
    let srgb_profile = rcms::IccProfile::new_srgb();
    let p3_profile = rcms::IccProfile::new_display_p3();
    let pipe = link(
        &[&p3_profile, &srgb_profile],
        &[rcms::profile::Intent::Perceptual, rcms::profile::Intent::Perceptual],
        &[false, false],
        &[0., 0.],
    )
    .unwrap();
    use_hook(|| IOS18ScreenShot {
        pipeline: pipe,
        iterations: Rc::new(RefCell::new(3)),
        source: Rc::default(),
        frames: Rc::default(),
        animation: Rc::default(),
    })
}

impl IOS18ScreenShot {
    pub fn get_source(&self) -> Result<String, ImageError> {
        base64_image(&*self.source.borrow())
    }
    pub fn set_source(self, e: Event<FormData>) -> Task {
        self.clear_frames();
        spawn(async move {
            match e.data.files() {
                Some(files) => match files.files().first() {
                    Some(name) => {
                        let bytes = files.read_file(name).await.unwrap_or_default();
                        match image::load_from_memory(&bytes) {
                            Ok(any) => {
                                *self.source.borrow_mut() = any.to_rgb8();
                                self.collect_frames();
                            }
                            Err(_) => {}
                        }
                    }
                    None => {}
                },
                None => {}
            }
        })
    }
    fn convert_pixel(&self, pixel: [u8; 3]) -> [u8; 3] {
        let [r, g, b] = pixel;
        let srgb_in = &[r as f64 / 255., g as f64 / 255., b as f64 / 255.];
        let mut p3_out = [0.; 3];
        self.pipeline.transform(srgb_in, &mut p3_out);
        [(p3_out[0] * 255.) as u8, (p3_out[1] * 255.) as u8, (p3_out[2] * 255.) as u8]
    }
    pub unsafe fn convert_image(&self, srgb: &RgbImage) -> RgbImage {
        let mut p3_out = RgbImage::new(srgb.width(), srgb.height());
        for (x, y, c) in srgb.enumerate_pixels() {
            let px = self.convert_pixel(c.0);
            p3_out.unsafe_put_pixel(x, y, Rgb(px));
        }
        p3_out
    }
    pub fn get_width(&self) -> usize {
        self.source.borrow().width() as usize
    }
    pub fn get_height(&self) -> usize {
        self.source.borrow().height() as usize
    }

    pub fn get_iteration(&self) -> usize {
        *self.iterations.borrow()
    }
    pub fn set_iteration(&self, e: &Event<FormData>) {
        let value = e.data.value();
        match value.parse::<usize>() {
            Ok(o) => *self.iterations.borrow_mut() = o,
            Err(_) => {}
        }
        // needs_update()
    }

    pub fn clear_frames(&self) {
        *self.frames.borrow_mut() = Vec::with_capacity(self.get_iteration());
    }

    pub fn collect_frames(self) -> Task {
        let mut srgb_in = self.source.borrow().clone();

        spawn(async move {
            // index[0] = source image
            self.frames.borrow_mut().push(srgb_in.clone());
            info!("push frame 0");
            for i in 0..self.get_iteration() {
                let p3_out = unsafe { self.convert_image(&srgb_in) };
                srgb_in = p3_out.clone();
                self.frames.borrow_mut().push(p3_out);
                info!("push frame {}", i + 1);
                needs_update();
            }
        })
    }
    pub fn show_frames(&self) -> Element {
        let frames = self.frames.borrow().clone();
        rsx! {
            div {
                for frame in frames {
                    match base64_image(&frame) {
                        Ok(o) => rsx! {
                            img {
                                src: o
                            }
                        },
                        Err(e) => rsx! {
                            span {
                                "{e}"
                            }
                        },
                    }
                }
            }
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

fn App() -> Element {
    let image = use_custom_image();
    let width = image.get_iteration();
    // let source_base64 = match image.get_source() {
    //     Ok(o) => o,
    //     Err(e) => e.to_string(),
    // };
    let frames = image.show_frames();

    let value1 = image.clone();
    let value2 = image.clone();
    rsx! {
        div {
            input {
                r#type: "file",
                oninput: move |e| {
                    value1.clone().set_source(e);
                }
            }
            span {
                "iteration:"
            }
            input {
                r#type: "number",
                value: "{image.get_iteration()}",
                oninput: move |e| {
                  value2.set_iteration(&e)
                }
            }
            // img { src: source_base64 }
            {frames}
        }
    }
}
