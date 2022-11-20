use display_interface_spi::SPIInterfaceNoCS;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::PrimitiveStyleBuilder;
use embedded_graphics::primitives::Rectangle;
use esp_idf_hal::delay::Ets;
use esp_idf_hal::gpio::*;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::spi::*;
use esp_idf_hal::units::FromValueType;
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use std::thread;
use std::time::Duration;

use mipidsi::{Builder, Orientation};

fn main() -> anyhow::Result<()> {
    let peripherals = Peripherals::take().unwrap();
    let spi = peripherals.spi2;

    let rst = PinDriver::output(peripherals.pins.gpio8)?;
    let dc = PinDriver::output(peripherals.pins.gpio9)?;
    let mut backlight = PinDriver::output(peripherals.pins.gpio10)?;

    let sclk = peripherals.pins.gpio6;
    let sda = peripherals.pins.gpio7;

    let sdi = peripherals.pins.gpio1;
    let cs = peripherals.pins.gpio2;

    let mut delay = Ets;

    // configuring the spi interface, note that in order for the ST7789 to work, the data_mode needs to be set to MODE_3
    let config = config::Config::new()
        .baudrate(26.MHz().into())
        .data_mode(embedded_hal::spi::MODE_3);

    let device =
        SpiDeviceDriver::new_single(spi, sclk, sda, Some(sdi), Dma::Disabled, Some(cs), &config)?;

    // display interface abstraction from SPI and DC
    let di = SPIInterfaceNoCS::new(device, dc);

    // create driver
    let mut display = Builder::st7789(di)
        .with_display_size(240, 240)
        // set default orientation
        .with_orientation(Orientation::Portrait(false))
        // initialize
        .init(&mut delay, Some(rst))
        .unwrap();

    // turn on the backlight
    backlight.set_high()?;

    // let raw_image_data = ImageRawLE::new(include_bytes!("../examples/assets/ferris.raw"), 86);
    // let ferris = Image::new(&raw_image_data, Point::new(0, 0));

    // draw image on black background
    display.clear(Rgb565::BLACK).unwrap();
    // ferris.draw(&mut display).unwrap();

    Rectangle::new(Point::new(10, 10), Size::new(40, 50))
        .into_styled(
            PrimitiveStyleBuilder::new()
                .stroke_width(3)
                .stroke_color(Rgb565::RED)
                .fill_color(Rgb565::GREEN)
                .build(),
        )
        .draw(&mut display)
        .unwrap();

    println!("Image printed!");

    loop {
        thread::sleep(Duration::from_millis(1000));
    }
}
