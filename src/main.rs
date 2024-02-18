use std::{error::Error, time::{Duration, Instant}};

use display_interface_spi::SPIInterfaceNoCS;
use embedded_graphics::{
    image::{Image, ImageRaw},
    prelude::*,
};
use embedded_graphics_core::pixelcolor::{raw::ToBytes, Rgb565, Rgb888};
use esp_idf_hal::{
    delay::Ets,
    gpio::{AnyIOPin, Gpio16, Gpio18, Gpio19, Gpio23, Gpio4, Gpio5, PinDriver},
    spi::{
        config::{Config, DriverConfig},
        Dma, SpiDeviceDriver, SpiDriver, SPI2,
    },
};
use mipidsi::Builder;

use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    if let Err(err) = run() {
        println!("Got an Error {err}");
    }
}

// the display is in portrait mode so we need images 135 wide by 240 high
static IMAGES: &[(&str, &[u8])] = &[
    ("Qoi Logo", include_bytes!("../images/qoi_logo-135x240.qoi")),
    ("Cube 4", include_bytes!("../images/Qube4-esp32.qoi")),
    ("Honey", include_bytes!("../images/Honey.qoi"))
];

fn run() -> Result<(), Box<dyn Error>> {
    println!("Hello, world!");

    let rst = PinDriver::input_output_od(unsafe { Gpio23::new() })?;
    let dc = PinDriver::input_output_od(unsafe { Gpio16::new() })?;
    let mut delay = Ets;

    let sclk = unsafe { Gpio18::new() };
    let spi = unsafe { SPI2::new() };
    let sdo = unsafe { Gpio19::new() };

    let spi = SpiDriver::new(
        spi,
        sclk,
        sdo,
        None::<AnyIOPin>,
        &DriverConfig::new().dma(
            // Dma::Channel1(240 * 135 * 2 + 8), // 2 bytes per pixel: 5 bit red, 6 bit green, 5 bit blue
            Dma::Channel1(0x1000), // must be multiple of 4 and in 1..=4096 i.e not 0 and <= 0x1000
        ),
    )?;

    let cs = unsafe { Gpio5::new() };

    let spi = SpiDeviceDriver::new(spi, Some(cs), &Config::new())?;

    let di = SPIInterfaceNoCS::new(spi, dc);
    let mut display = Builder::st7789_pico1(di)
        .init(&mut delay, Some(rst))
        .map_err(|err| Box::<dyn Error>::from(format!("{err:?}")))?;

    display
        .clear(Rgb565::RED)
        .map_err(|err| Box::<dyn Error>::from(format!("{err:?}")))?;

    let mut bl = PinDriver::input_output_od(unsafe { Gpio4::new() })?;
    bl.set_high()?;

    static mut IMAGE_BUFFER: [u8; 240 * 135 * 2] = [0u8; 240 * 135 * 2];
    // Safety:
    // - this is the only place a reference to IMAGE_BUFFER is taken
    // - no concurrent calls to run are made
    // - the reference taken does not escape the function
    let image_buffer = unsafe {&mut IMAGE_BUFFER};

    let mut next_draw = Instant::now();

    for (image_name, image_data) in IMAGES.into_iter().cycle() {

        println!("Decoding {image_name} Image Data");

        let Some(raw_image) = decode_qoi_image(image_data, image_buffer)
        else {
            return Err(Box::<dyn Error>::from("Qoi Decoding Error"));
        };

        let image = Image::new(&raw_image, Point::zero());

        sleep_until(next_draw);

        println!("Displaying: {image_name}");

        image.draw(&mut display)
            .map_err(|_| Box::<dyn Error>::from("draw image"))?;

        next_draw = Instant::now() + Duration::from_secs(5);
    }

    #[allow(unreachable_code)]
    Ok(())
}

fn sleep_until(target: Instant) {
    let now = Instant::now();
    if now < target {
        println!("Waiting for next image change!");
        std::thread::sleep(target - now);
    }
}

fn decode_qoi_image<'b>(qoi_data: &[u8], pixel_buffer: &'b mut[u8]) -> Option<ImageRaw<'b, Rgb565>> {

    let (qoi_header, pixels) = arqoii::decode::QoiDecoder::new(qoi_data.iter().copied())?;

    for (dest_pixel, src_pixel) in pixel_buffer.chunks_exact_mut(2).zip(pixels) {
        let be_pixel =
            Rgb565::from(Rgb888::new(src_pixel.r, src_pixel.g, src_pixel.b)).to_be_bytes();
        dest_pixel[0] = be_pixel[0];
        dest_pixel[1] = be_pixel[1];
    }

    Some(ImageRaw::new(pixel_buffer, qoi_header.width))
}
