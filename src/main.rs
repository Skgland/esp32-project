use std::{error::Error};

use embedded_graphics_core::pixelcolor::{Rgb565, Rgb888, raw::ToBytes};
use display_interface_spi::SPIInterfaceNoCS;
use mipidsi::Builder; 
use embedded_graphics::{prelude::*, image::{ImageRawBE, ImageRaw}};
use esp_idf_hal::{
    delay::Ets,
    gpio::{Gpio16, Gpio18, Gpio19, Gpio23, Gpio5, PinDriver, AnyIOPin, Gpio4},
    spi::{config::Config, Dma, SpiDeviceDriver, SpiDriver, SPI2},
};

use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

fn main()  {
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
        Dma::Channel1(240 * 135 * 2 + 8), // 2 bytes per pixel: 5 bit red, 6 bit green, 5 bit blue
    )?;


    let cs = unsafe { Gpio5::new() };

    let spi = SpiDeviceDriver::new(spi, Some(cs), &Config::new())?;
    
    let di = SPIInterfaceNoCS::new(spi, dc);
    let mut display = Builder::st7789_pico1(di)
        .init(&mut delay, Some(rst))
	    .map_err(|err| Box::<dyn Error>::from(format!("{err:?}")))?;
    
    display.clear(Rgb565::RED)
        .map_err(|err| Box::<dyn Error>::from(format!("{err:?}")))?;

    let mut bl = PinDriver::input_output_od(unsafe { Gpio4::new() })?;
    bl.set_high()?;

    
    let img_data = include_bytes!("qoi_logo-240x135.qoi");
    static mut IMG : [u8; 240 * 135 * 2] = [0u8; 240 * 135 * 2];

    let Some((header , pixels)) = arqoii::QoiDecoder::new(img_data.iter().copied()) else {
        return Err(Box::<dyn Error>::from("Qoi Decoding Error"))
    };
    
    println!("Decoding Qoi Image Data");
    
    for (dest_pixel, src_pixel) in unsafe {IMG.chunks_exact_mut(2)}.zip(pixels) {
        let be_pixel = Rgb565::from(Rgb888::new(src_pixel.r, src_pixel.g, src_pixel.b)).to_be_bytes();
        dest_pixel[0] = be_pixel[0];
        dest_pixel[1] = be_pixel[1];
    }

    println!("Displaying Image");
    
    let raw: ImageRawBE<Rgb565> = ImageRaw::new(unsafe {&IMG}, header.width);
        raw
        .draw(&mut display)
        .map_err(|_| Box::<dyn Error>::from("draw image"))?;

    Ok(())
}
