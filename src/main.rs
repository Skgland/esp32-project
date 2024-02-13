use std::{error::Error, time::Duration};

use display_interface_spi::SPIInterfaceNoCS;
use embedded_graphics::{
    image::{ImageRaw, ImageRawBE},
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

    let qoi_logo_data = include_bytes!("qoi_logo-240x135.qoi");
    let cube4_data = include_bytes!("Qube4-esp32.qoi");
    static mut QOI_LOGO: [u8; 240 * 135 * 2] = [0u8; 240 * 135 * 2];
    static mut CUBE_4: [u8; 240 * 135 * 2] = [0u8; 240 * 135 * 2];

    println!("Decoding Qoi Image Data");

    let Some((qoi_logo_header, pixels)) = arqoii::QoiDecoder::new(qoi_logo_data.iter().copied())
    else {
        return Err(Box::<dyn Error>::from("Qoi Decoding Error"));
    };

    for (dest_pixel, src_pixel) in unsafe { QOI_LOGO.chunks_exact_mut(2) }.zip(pixels) {
        let be_pixel =
            Rgb565::from(Rgb888::new(src_pixel.r, src_pixel.g, src_pixel.b)).to_be_bytes();
        dest_pixel[0] = be_pixel[0];
        dest_pixel[1] = be_pixel[1];
    }

    let Some((cube4_header, pixels)) = arqoii::QoiDecoder::new(cube4_data.iter().copied()) else {
        return Err(Box::<dyn Error>::from("Qoi Decoding Error"));
    };

    for (dest_pixel, src_pixel) in unsafe { CUBE_4.chunks_exact_mut(2) }.zip(pixels) {
        let be_pixel =
            Rgb565::from(Rgb888::new(src_pixel.r, src_pixel.g, src_pixel.b)).to_be_bytes();
        dest_pixel[0] = be_pixel[0];
        dest_pixel[1] = be_pixel[1];
    }

    loop {
        println!("Displaying: Qoi Logo");

        let raw: ImageRawBE<Rgb565> = ImageRaw::new(unsafe { &QOI_LOGO }, qoi_logo_header.width);
        raw.draw(&mut display)
            .map_err(|_| Box::<dyn Error>::from("draw image"))?;

        std::thread::sleep(Duration::from_secs(5));

        println!("Displaying: Cube 4");

        let raw: ImageRawBE<Rgb565> = ImageRaw::new(unsafe { &CUBE_4 }, cube4_header.width);
        raw.draw(&mut display)
            .map_err(|_| Box::<dyn Error>::from("draw image"))?;

        std::thread::sleep(Duration::from_secs(5));
    }

    #[allow(unreachable_code)]
    Ok(())
}
