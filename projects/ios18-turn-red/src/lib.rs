use image::{GenericImage, ImageError, Rgb, RgbImage};
use rcms::link::{LinkError, link};

pub struct IOS18ScreenShot {
    pipeline: rcms::pipeline::Pipeline,
}

impl IOS18ScreenShot {
    pub fn new() -> Result<Self, LinkError> {
        let srgb_profile = rcms::IccProfile::new_srgb();
        let p3_profile = rcms::IccProfile::new_display_p3();
        let pipe = link(
            &[&p3_profile, &srgb_profile],
            &[rcms::profile::Intent::Perceptual, rcms::profile::Intent::Perceptual],
            &[false, false],
            &[0., 0.],
        )?;
        Ok(Self { pipeline: pipe })
    }

    pub fn convert_f64(&self, srgb: &[f64; 3], p3: &mut [f64; 3]) {
        self.pipeline.transform(srgb, p3)
    }
    pub fn convert_u8(&self, [r, g, b]: &[u8; 3]) -> [u8; 3] {
        let srgb_in = &[*r as f64 / 255., *g as f64 / 255., *b as f64 / 255.];
        let mut p3_out = [0.; 3];
        self.pipeline.transform(srgb_in, &mut p3_out);
        [(p3_out[0] * 255.) as u8, (p3_out[1] * 255.) as u8, (p3_out[2] * 255.) as u8]
    }

    pub fn convert_image(&self, srgb: &RgbImage) -> RgbImage {
        let mut p3_out = RgbImage::new(srgb.width(), srgb.height());
        for (x, y, c) in srgb.enumerate_pixels() {
            unsafe {
                let px = self.convert_u8(&c.0);
                p3_out.unsafe_put_pixel(x, y, Rgb(px));
            }
        }
        p3_out
    }
    pub fn convert_file(&self, path_in: &str, path_out: &str) -> Result<(), ImageError> {
        let srgb = image::open(path_in)?;
        let p3_out = self.convert_image(&srgb.to_rgb8());
        p3_out.save(path_out)
    }
}

#[test]
pub fn turn_red() {
    let pipe = IOS18ScreenShot::new().unwrap();
    let src = [0.8, 0.5, 0.9];
    let mut p3_out = src;
    let mut srgb_colors = Vec::with_capacity(10);
    srgb_colors.push(p3_out);
    for _ in 0..10 {
        let srgb_in = p3_out;
        pipe.convert_f64(&srgb_in, &mut p3_out);
        srgb_colors.push(p3_out);
    }
    println!("| 迭代 |      红 |      绿 |      蓝 |     RGB |");
    println!("|---:|-------:|-------:|-------:|--------:|");
    for (i, p3_out) in srgb_colors.iter().enumerate() {
        let hash = format!("#{:02X}{:02X}{:02X}", (p3_out[0] * 255.) as u8, (p3_out[1] * 255.) as u8, (p3_out[2] * 255.) as u8);
        println!("|{}|{:.4}|{:.4}|{:.4}|{}|", i, p3_out[0], p3_out[1], p3_out[2], hash);
    }
}

#[test]
fn test_file() {
    let pipe = IOS18ScreenShot::new().unwrap();
    pipe.convert_file(r#"C:\Users\Aster\Desktop\10.png"#, r#"C:\Users\Aster\Desktop\11.png"#).unwrap();
    pipe.convert_file(r#"C:\Users\Aster\Desktop\11.png"#, r#"C:\Users\Aster\Desktop\12.png"#).unwrap();
    pipe.convert_file(r#"C:\Users\Aster\Desktop\12.png"#, r#"C:\Users\Aster\Desktop\13.png"#).unwrap();
    pipe.convert_file(r#"C:\Users\Aster\Desktop\13.png"#, r#"C:\Users\Aster\Desktop\14.png"#).unwrap();
    pipe.convert_file(r#"C:\Users\Aster\Desktop\14.png"#, r#"C:\Users\Aster\Desktop\15.png"#).unwrap();
    pipe.convert_file(r#"C:\Users\Aster\Desktop\15.png"#, r#"C:\Users\Aster\Desktop\16.png"#).unwrap();
    pipe.convert_file(r#"C:\Users\Aster\Desktop\16.png"#, r#"C:\Users\Aster\Desktop\17.png"#).unwrap();
    pipe.convert_file(r#"C:\Users\Aster\Desktop\17.png"#, r#"C:\Users\Aster\Desktop\18.png"#).unwrap();
    pipe.convert_file(r#"C:\Users\Aster\Desktop\18.png"#, r#"C:\Users\Aster\Desktop\19.png"#).unwrap();
    pipe.convert_file(r#"C:\Users\Aster\Desktop\19.png"#, r#"C:\Users\Aster\Desktop\20.png"#).unwrap();
}
