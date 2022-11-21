use debouncr::{debounce_2, Edge};
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
use esp_idf_sys::{
    c_types::c_void, esp, gpio_config, gpio_config_t, gpio_install_isr_service,
    gpio_int_type_t_GPIO_INTR_ANYEDGE, gpio_isr_handler_add, gpio_mode_t_GPIO_MODE_INPUT,
    xQueueGenericCreate, xQueueGiveFromISR, xQueueReceive, QueueHandle_t, ESP_INTR_FLAG_IRAM,
};
use mipidsi::{Builder, Orientation};
use rotary_encoder_embedded::{Direction, RotaryEncoder};
use std::{
    ptr,
    sync::{
        atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

// This `static mut` holds the queue handle we are going to get from `xQueueGenericCreate`.
// This is unsafe, but we are careful not to enable our GPIO interrupt handler until after this value has been initialised, and then never modify it again
static mut EVENT_QUEUE: Option<QueueHandle_t> = None;

#[link_section = ".iram0.text"]
unsafe extern "C" fn button_interrupt(_: *mut c_void) {
    xQueueGiveFromISR(EVENT_QUEUE.unwrap(), std::ptr::null_mut());
}

fn main() -> anyhow::Result<()> {
    let peripherals = Peripherals::take().unwrap();
    let spi = peripherals.spi2;

    // To S2 on encoder
    let rotary_a = PinDriver::input(peripherals.pins.gpio1)?;
    // To S1 on encoder
    let rotary_b = PinDriver::input(peripherals.pins.gpio2)?;
    // To KEY on encoder
    let button = PinDriver::input(peripherals.pins.gpio3)?;

    // Configures the button
    let io_conf = gpio_config_t {
        pin_bit_mask: (1 << 1) | (1 << 2) | (1 << 3),
        mode: gpio_mode_t_GPIO_MODE_INPUT,
        pull_up_en: false.into(),
        pull_down_en: false.into(),
        intr_type: gpio_int_type_t_GPIO_INTR_ANYEDGE,
    };

    // Queue configurations
    const QUEUE_TYPE_BASE: u8 = 0;
    const ITEM_SIZE: u32 = 0; // we're not posting any actual data, just notifying
    const QUEUE_SIZE: u32 = 10;

    unsafe {
        // Writes the button configuration to the registers
        esp!(gpio_config(&io_conf))?;

        // Installs the generic GPIO interrupt handler
        esp!(gpio_install_isr_service(ESP_INTR_FLAG_IRAM as i32))?;

        // Instantiates the event queue
        EVENT_QUEUE = Some(xQueueGenericCreate(QUEUE_SIZE, ITEM_SIZE, QUEUE_TYPE_BASE));

        // Registers our function with the generic GPIO interrupt handler we installed earlier.
        esp!(gpio_isr_handler_add(
            1,
            Some(button_interrupt),
            std::ptr::null_mut()
        ))?;
        esp!(gpio_isr_handler_add(
            2,
            Some(button_interrupt),
            std::ptr::null_mut()
        ))?;
        // Button - we'll just poll this in a loop for debouncing reasons
        // esp!(gpio_isr_handler_add(
        //     3,
        //     Some(button_interrupt),
        //     std::ptr::null_mut()
        // ))?;
    }

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
        Option::<gpio::Gpio1>::None,
        Dma::Auto(1024),
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
    display.clear(Rgb565::BLACK).unwrap();

    let mut count = Arc::new(AtomicI32::new(0));
    // let mut button_state = Arc::new(AtomicBool::new(false));
    let mut button_state = Arc::new(AtomicU32::new(0));

    let count_writer = count.clone();
    let button_state_writer = button_state.clone();

    thread::spawn(move || {
        // Reads the queue in a loop.
        loop {
            unsafe {
                // maximum delay
                const QUEUE_WAIT_TICKS: u32 = 1000;

                // Reads the event item out of the queue
                let res = xQueueReceive(EVENT_QUEUE.unwrap(), ptr::null_mut(), QUEUE_WAIT_TICKS);

                if res == 1 {
                    rotary_encoder.update();

                    match rotary_encoder.direction() {
                        Direction::Clockwise => {
                            count_writer.fetch_add(1, Ordering::SeqCst);
                        }
                        Direction::Anticlockwise => {
                            count_writer.fetch_sub(1, Ordering::SeqCst);
                        }
                        Direction::None => {
                            // Do nothing
                        }
                    }
                }
            }
        }
    });

    thread::spawn(move || {
        let mut debouncer = debounce_2(false);

        loop {
            // Button is active-low
            if let Some(Edge::Rising) = debouncer.update(button.is_low()) {
                button_state_writer.fetch_add(1, Ordering::SeqCst);
            }

            thread::sleep(Duration::from_millis(15));
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
                "Count: {}         \nButton: {}",
                count.load(Ordering::SeqCst),
                button_state.load(Ordering::SeqCst) // if button_state.load(Ordering::SeqCst) {
                                                    //     "1"
                                                    // } else {
                                                    //     "0"
                                                    // },
            ),
            Point::new(20, 20),
            style,
        )
        .draw(&mut display)
        .unwrap();

        thread::sleep(Duration::from_millis(50));
    }
}
