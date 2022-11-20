use display_interface_spi::SPIInterfaceNoCS;
use embedded_graphics::{
    mono_font::{ascii::FONT_7X13, MonoTextStyle, MonoTextStyleBuilder},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle},
    text::Text,
};
use esp_idf_hal::{
    delay::Ets,
    gpio::{self, *},
    peripherals::Peripherals,
    spi::*,
    units::FromValueType,
};
use esp_idf_sys as _;
use mipidsi::{Builder, Orientation};
use rotary_encoder_embedded::{Direction, RotaryEncoder};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicI32, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

fn main() -> anyhow::Result<()> {
    let peripherals = Peripherals::take().unwrap();
    let spi = peripherals.spi2;

    let rotary_a = PinDriver::input(peripherals.pins.gpio1)?;
    let rotary_b = PinDriver::input(peripherals.pins.gpio2)?;
    let button = PinDriver::input(peripherals.pins.gpio3)?;

    let mut rotary_encoder = RotaryEncoder::new(rotary_a, rotary_b).into_standard_mode();

    let rst = PinDriver::output(peripherals.pins.gpio8)?;
    let dc = PinDriver::output(peripherals.pins.gpio9)?;
    let mut backlight = PinDriver::output(peripherals.pins.gpio10)?;

    let sclk = peripherals.pins.gpio6;
    let sda = peripherals.pins.gpio7;

    let mut delay = Ets;

    // configuring the spi interface, note that in order for the ST7789 to work, the data_mode needs to be set to MODE_3
    let config = config::Config::new()
        .baudrate(26.MHz().into())
        .data_mode(embedded_hal::spi::MODE_3);

    let device = SpiDeviceDriver::new_single(
        spi,
        sclk,
        sda,
        // Some(sdi),
        Option::<gpio::Gpio1>::None,
        Dma::Auto(1024),
        // Some(cs),
        Option::<gpio::Gpio2>::None,
        &config,
    )?;

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

    // Rectangle::new(Point::new(10, 10), Size::new(40, 50))
    //     .into_styled(
    //         PrimitiveStyleBuilder::new()
    //             .stroke_width(3)
    //             .stroke_color(Rgb565::RED)
    //             .fill_color(Rgb565::GREEN)
    //             .build(),
    //     )
    //     .draw(&mut display)
    //     .unwrap();

    // println!("Image printed!");

    let mut count = Arc::new(AtomicI32::new(0));
    let mut button_state = Arc::new(AtomicBool::new(false));

    let count_writer = count.clone();
    let button_state_writer = button_state.clone();

    thread::spawn(move || {
        loop {
            rotary_encoder.update();

            match rotary_encoder.direction() {
                Direction::Clockwise => {
                    count_writer.fetch_sub(1, Ordering::Relaxed);
                }
                Direction::Anticlockwise => {
                    count_writer.fetch_add(1, Ordering::Relaxed);
                }
                Direction::None => {
                    // Do nothing
                }
            }

            // TODO: `debouncr`
            if button.is_high() {
                button_state_writer.store(false, Ordering::Relaxed);
            } else {
                button_state_writer.store(true, Ordering::Relaxed);
            }

            thread::sleep(Duration::from_millis(10));
        }
    });

    let style = MonoTextStyleBuilder::new()
        .font(&FONT_7X13)
        .background_color(Rgb565::BLACK)
        .text_color(Rgb565::WHITE)
        .build();

    loop {
        Text::new(
            &format!(
                "Count: {}\nButton: {}",
                count.load(Ordering::Relaxed),
                if button_state.load(Ordering::Relaxed) {
                    "1"
                } else {
                    "0"
                },
            ),
            Point::new(20, 20),
            style,
        )
        .draw(&mut display)
        .unwrap();

        thread::sleep(Duration::from_millis(100));
    }
}
