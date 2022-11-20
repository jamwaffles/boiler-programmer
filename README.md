# Boiler programmer

A replacement for the crappy timer in my boiler.

Using an AI-Thinker NodeMCU-Series ESP-C3-32S-Kit

Schematic
[here](https://docs.ai-thinker.com/_media/esp32/docs/esp-c3-32s-kit-v1.0_specification.pdf).

---

Wifi MQTT example:
<https://github.com/bjoernQ/esp32c3-rust-std-temperature-logger/blob/main/src/main.rs>

ESPHome Rust client: <https://github.com/pixelspark/esphome-rs/blob/main/src/connection.rs>

More HTTP stuff: <https://espressif-trainings.ferrous-systems.com/03_3_2_http_client.html> (also has
interrupt examples)

Note: <https://crates.io/crates/rotary-encoder-embedded>

Links to other projects: <https://github.com/esp-rs/awesome-esp-rust>

Interrupt: <https://github.com/ferrous-systems/espressif-trainings/tree/main/advanced>

Async networking using `smol`:
<https://github.com/ivmarkov/rust-esp32-std-demo/blob/main/src/main.rs>
