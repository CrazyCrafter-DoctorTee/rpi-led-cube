mod cube;

use std::{
    iter::{once, repeat},
    sync::mpsc::{sync_channel, Receiver, SyncSender, TryRecvError},
    thread::{self, JoinHandle},
    time::Duration,
};

use clap::{Parser, Subcommand, ValueEnum};

use cube::CubeDriver;

type Frame = [[u8; 8]; 8];

const INTER_FRAME_SLEEP: Duration = Duration::from_millis(19); // 50ish FPS

/// Bit-bang the PI GPIO pins to render 3D values on the LED cube
#[derive(Parser)]
struct Cli {
    /// The display program to run
    #[command(subcommand)]
    program: Program,
}

#[derive(Copy, Clone, ValueEnum)]
enum Layer {
    Zero,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
}

impl From<Layer> for u8 {
    fn from(item: Layer) -> Self {
        match item {
            Layer::Zero => 0,
            Layer::One => 1,
            Layer::Two => 2,
            Layer::Three => 3,
            Layer::Four => 4,
            Layer::Five => 5,
            Layer::Six => 6,
            Layer::Seven => 7,
        }
    }
}

#[derive(Clone, Subcommand)]
enum Program {
    /// Turn on all of the LEDs
    AllOn,
    /// Cycle one layer at a time
    Cycle,
    /// Turn on one full layer of LEDs
    OneLayer { which: Layer },
}

fn spawn_display() -> (SyncSender<Frame>, JoinHandle<rppal::gpio::Result<()>>) {
    let (tx, rx): (SyncSender<Frame>, Receiver<Frame>) = sync_channel(64);

    let handler = thread::spawn(move || {
        let mut driver = CubeDriver::try_new()?;

        let mut curr_frame = [[0; 8]; 8];

        while let maybe_frame = rx.try_recv() {
            if let Ok(frame) = maybe_frame {
                curr_frame = frame;
            } else if let Err(TryRecvError::Disconnected) = maybe_frame {
                break;
            }

            driver.write_frame(curr_frame);
        }
        Ok(())
    });

    (tx, handler)
}

fn test_dummy_on() {
    let mut driver = CubeDriver::try_new().unwrap();

    loop {
        driver.write_frame([[255; 8]; 8]);
    }
}

fn test_one_layer(layer: Layer) {
    let mut driver = CubeDriver::try_new().unwrap();

    let frame: [[u8; 8]; 8] = core::array::from_fn(|i| {
        if i == u8::from(layer).into() {
            [255; 8]
        } else {
            [0; 8]
        }
    });

    loop {
        driver.write_frame(frame);
    }
}

fn test_all_on() {
    let (sender, _handle) = spawn_display();

    loop {
        sender.send([[255; 8]; 8]);
    }
}

fn test_cycle_layers() {
    let (sender, _handle) = spawn_display();

    let mut layer_cycle = once([255; 8]).chain(repeat([0; 8]).take(8)).cycle();

    loop {
        // Cycle through a window of 9 layers with one lit
        sender.send([
            layer_cycle.next().unwrap(),
            layer_cycle.next().unwrap(),
            layer_cycle.next().unwrap(),
            layer_cycle.next().unwrap(),
            layer_cycle.next().unwrap(),
            layer_cycle.next().unwrap(),
            layer_cycle.next().unwrap(),
            layer_cycle.next().unwrap(),
        ]);
    }
}

fn main() {
    let args = Cli::parse();

    match args.program {
        Program::AllOn => test_dummy_on(),
        Program::Cycle => test_cycle_layers(),
        Program::OneLayer { which: layer } => test_one_layer(layer),
    };
}
