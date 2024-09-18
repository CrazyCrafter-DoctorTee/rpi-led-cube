use std::{thread, time::Duration};

use rppal::gpio::{Gpio, Level, OutputPin, Result};

const SLOWDOWN: u64 = 1;
const ROW_DRIVE_CLOCK_SLEEP: Duration = Duration::from_micros(5 * SLOWDOWN);
const ROW_WRITE_CLOCK_SLEEP: Duration = Duration::from_micros(5 * SLOWDOWN);
const LAYER_STROBE_SLEEP: Duration = Duration::from_micros(100 * SLOWDOWN);

/**
 * Handles all bit-banging and state for driving the cube
 */
pub struct CubeDriver {
    par_1: OutputPin,
    par_2: OutputPin,
    par_3: OutputPin,
    par_4: OutputPin,
    par_5: OutputPin,
    par_6: OutputPin,
    par_7: OutputPin,
    par_8: OutputPin,
    /// Rising edge
    par_rclk: OutputPin,
    /// Rising edge
    par_srclk: OutputPin,
    /// Active low
    par_srclr: OutputPin,
    layer_sel_bit_0: OutputPin,
    layer_sel_bit_1: OutputPin,
    layer_sel_bit_2: OutputPin,
    /// Active low
    out_enable: OutputPin,
}

#[inline]
fn check_bit(value: u8, pow2: u8) -> Level {
    if value & pow2 == 0 {
        Level::Low
    } else {
        Level::High
    }
}

impl Drop for CubeDriver {
    fn drop(&mut self) {
        self.layer_sel_bit_0.set_low();
        self.layer_sel_bit_1.set_low();
        self.layer_sel_bit_2.set_low();
        self.out_enable.set_high(); // Disable output

        self.par_1.set_low();
        self.par_2.set_low();
        self.par_3.set_low();
        self.par_4.set_low();
        self.par_5.set_low();
        self.par_6.set_low();
        self.par_7.set_low();
        self.par_8.set_low();
        self.par_rclk.set_low();
        self.par_srclk.set_low();
        self.par_srclr.set_low();
    }
}

impl CubeDriver {
    pub fn try_new() -> Result<Self> {
        let gpio = Gpio::new()?;

        let layer_sel_bit_0 = gpio.get(06)?.into_output_low();
        let layer_sel_bit_1 = gpio.get(13)?.into_output_low();
        let layer_sel_bit_2 = gpio.get(16)?.into_output_low();
        let out_enable = gpio.get(09)?.into_output_high(); // Start inactive

        let par_1 = gpio.get(12)?.into_output_low();
        let par_2 = gpio.get(05)?.into_output_low();
        let par_3 = gpio.get(10)?.into_output_low();
        let par_4 = gpio.get(18)?.into_output_low();
        let par_5 = gpio.get(17)?.into_output_low();
        let par_6 = gpio.get(04)?.into_output_low();
        let par_7 = gpio.get(02)?.into_output_low();
        let par_8 = gpio.get(03)?.into_output_low();
        let par_rclk = gpio.get(08)?.into_output_low();
        let par_srclk = gpio.get(11)?.into_output_low();
        let mut par_srclr = gpio.get(07)?.into_output_low();

        // Wait for initial levels to apply and settle
        thread::sleep(Duration::from_micros(5));

        // Clear the buffers
        par_srclr.set_low();
        thread::sleep(Duration::from_micros(5));
        par_srclr.set_high();
        thread::sleep(Duration::from_micros(5));

        Ok(CubeDriver {
            par_1,
            par_2,
            par_3,
            par_4,
            par_5,
            par_6,
            par_7,
            par_8,
            par_rclk,
            par_srclk,
            par_srclr,
            layer_sel_bit_0,
            layer_sel_bit_1,
            layer_sel_bit_2,
            out_enable,
        })
    }

    fn set_layer(&mut self, layer: u8) {
        self.layer_sel_bit_0.write(check_bit(layer, 1));
        self.layer_sel_bit_1.write(check_bit(layer, 2));
        self.layer_sel_bit_2.write(check_bit(layer, 4));
    }

    fn write_row(&mut self, pattern: u8) {
        // Need to sleep between setting channels and driving clock to allow inputs to settle
        self.par_1.write(check_bit(pattern, 1));
        self.par_2.write(check_bit(pattern, 2));
        self.par_3.write(check_bit(pattern, 4));
        self.par_4.write(check_bit(pattern, 8));
        self.par_5.write(check_bit(pattern, 16));
        self.par_6.write(check_bit(pattern, 32));
        self.par_7.write(check_bit(pattern, 64));
        self.par_8.write(check_bit(pattern, 128));
        thread::sleep(ROW_DRIVE_CLOCK_SLEEP);

        // Trigger rising edge clock pulse
        self.par_srclk.set_high();
        thread::sleep(ROW_DRIVE_CLOCK_SLEEP);

        // Relax clock line
        self.par_srclk.set_low();
        thread::sleep(ROW_DRIVE_CLOCK_SLEEP);
    }

    fn write_layer(&mut self, layer: u8, rows: [u8; 8]) {
        for row in rows {
            // Write 1 bit of each column in parallel
            self.write_row(row);
        }
        // Disable output to avoid ghosting, active low
        self.out_enable.set_high();
        thread::sleep(ROW_WRITE_CLOCK_SLEEP);

        // Move data to output register by triggering rising edge
        self.par_rclk.set_high();
        // Switch active layer too
        self.set_layer(layer);
        thread::sleep(ROW_WRITE_CLOCK_SLEEP);

        // Relax clock line and enable output
        self.par_rclk.set_low();
        self.out_enable.set_low();
        thread::sleep(ROW_WRITE_CLOCK_SLEEP);
    }

    pub fn test_layer(&mut self, layer: u8, data: [u8; 8]) {
        self.write_layer(layer, data);
        thread::sleep(LAYER_STROBE_SLEEP);
    }

    pub fn write_frame(&mut self, data: [[u8; 8]; 8]) {
        for (rows, layer) in data.iter().zip(0u8..) {
            self.write_layer(layer, *rows);
            thread::sleep(LAYER_STROBE_SLEEP);
        }
    }
}
