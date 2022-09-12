#![no_std]
#![no_main]

use cortex_m_rt::entry;
use embedded_graphics::{
    image::Image,
    mono_font::{ascii::FONT_10X20, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Circle, PrimitiveStyle},
    text::{Baseline, Text, TextStyleBuilder},
};
use embedded_hal::digital::v2::{InputPin, OutputPin};
use epd_waveshare::{
    color::*, epd2in7b::Display2in7b as EPDisplay, epd2in7b::Epd2in7b as EPD, epd2in7b::HEIGHT,
    epd2in7b::WIDTH, prelude::*,
};
use fugit::RateExtU32;
use rp2040_hal as hal;
use rp_pico::hal::{gpio, pac, prelude::*, spi};

use panic_halt as _;

#[entry]
fn main() -> ! {
    // Grab our singleton objects
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // Configure the clocks
    //
    // The default is to generate a 125 MHz system clock
    let clocks = hal::clocks::init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    // The single-cycle I/O block controls our GPIO pins
    let sio = hal::Sio::new(pac.SIO);

    // Set the pins up according to their function on this particular board
    let pins = rp_pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // Setup a delay for the LED blink signals:
    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    // These are implicitly used by the spi driver if they are in the correct mode
    let _spi_sclk = pins.gpio10.into_mode::<gpio::FunctionSpi>();
    let _spi_mosi = pins.gpio11.into_mode::<gpio::FunctionSpi>();

    // Create an SPI driver instance for the SPI0 device
    let spi = spi::Spi::<_, _, 8>::new(pac.SPI1);

    // Exchange the uninitialised SPI driver for an initialised one
    let mut spi = spi.init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        16_000_000u32.Hz(),
        &embedded_hal::spi::MODE_0,
    );
    // End of SPI declaration

    // Start the rest of pins needed to communicate with the screen
    let mut cs = pins.gpio9.into_push_pull_output(); // CS
    cs.set_high().unwrap();
    let busy = pins.gpio13.into_pull_up_input(); // BUSY
    let dc = pins.gpio8.into_push_pull_output(); // DC
    let rst = pins.gpio12.into_push_pull_output(); // RST

    // Start the EPD struct
    let mut epd = EPD::new(
        &mut spi,   // SPI
        cs,         // CS
        busy,       // BUSY
        dc,         // DC
        rst,        // RST
        &mut delay, // DELAY
    )
    .unwrap();
    // Start the display buffer
    let mut display = EPDisplay::default();
    display.set_rotation(DisplayRotation::Rotate90);
    display.clear_buffer(Color::Black);

    let crab: tinybmp::Bmp<BinaryColor> = tinybmp::Bmp::from_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/crab.bmp"
    )))
    .unwrap();

    let rpi: tinybmp::Bmp<BinaryColor> = tinybmp::Bmp::from_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/rpi.bmp"
    )))
    .unwrap();

    let rust: tinybmp::Bmp<BinaryColor> = tinybmp::Bmp::from_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/rust.bmp"
    )))
    .unwrap();

    write_image(&mut display, &crab);

    epd.update_frame(&mut spi, display.buffer(), &mut delay)
        .unwrap();
    epd.display_frame(&mut spi, &mut delay).unwrap();

    // Our LED output
    let mut led_pin = pins.led.into_push_pull_output();
    led_pin.set_high().unwrap();
    delay.delay_ms(500);

    // Our button input
    let key_0 = pins.gpio15.into_pull_up_input();
    let key_1 = pins.gpio17.into_pull_up_input();
    let key_2 = pins.gpio2.into_pull_up_input();

    loop {
        if key_0.is_low().unwrap() {
            led_pin.set_high().unwrap();
            display.clear_buffer(Color::Black);
            write_image(&mut display, &crab);
            epd.update_frame(&mut spi, display.buffer(), &mut delay)
                .unwrap();
            epd.display_frame(&mut spi, &mut delay).unwrap();
            led_pin.set_low().unwrap();
        }
        if key_1.is_low().unwrap() {
            led_pin.set_high().unwrap();
            display.clear_buffer(Color::Black);
            write_image(&mut display, &rpi);
            epd.update_frame(&mut spi, display.buffer(), &mut delay)
                .unwrap();
            epd.display_frame(&mut spi, &mut delay).unwrap();
            led_pin.set_low().unwrap();
        }
        if key_2.is_low().unwrap() {
            led_pin.set_high().unwrap();
            display.clear_buffer(Color::Black);
            write_image(&mut display, &rust);
            epd.update_frame(&mut spi, display.buffer(), &mut delay)
                .unwrap();
            epd.display_frame(&mut spi, &mut delay).unwrap();
            led_pin.set_low().unwrap();
        }
    }
}

fn write_image(display: &mut EPDisplay, image: &tinybmp::Bmp<BinaryColor>) {
    let size = image.size();
    Image::new(
        image,
        Point::new(
            HEIGHT as i32 / 2 - size.width as i32 / 2,
            WIDTH as i32 / 2 - size.height as i32 / 2,
        ),
    )
    .draw(display)
    .unwrap();
}

fn _write_text(display: &mut EPDisplay, text: &str, x: i32, y: i32, inverse: bool) {
    let mut text_color = White;
    let mut background_color = Black;
    if inverse {
        text_color = Black;
        background_color = White;
    }

    let style = MonoTextStyleBuilder::new()
        .font(&FONT_10X20)
        .text_color(text_color)
        .background_color(background_color)
        .build();

    let text_style = TextStyleBuilder::new().baseline(Baseline::Top).build();

    Text::with_text_style(text, Point::new(x, y), style, text_style)
        .draw(display)
        .unwrap();
}

fn _write_circle(display: &mut EPDisplay) {
    Circle::new(Point::new(WIDTH as i32 / 4, 0), WIDTH)
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::Off, 1))
        .draw(display)
        .unwrap();
}
