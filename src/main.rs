mod cube;

use std::{
    iter::{once, repeat},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{sync_channel, Receiver, SyncSender, TryRecvError},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use clap::{Parser, Subcommand, ValueEnum};

use cube::CubeDriver;

type Frame = [[u8; 8]; 8];

/// Bit-bang the PI GPIO pins to render 3D values on the LED cube
#[derive(Parser)]
struct Cli {
    /// The display program to run
    #[command(subcommand)]
    program: Program,
}

#[derive(Copy, Clone, ValueEnum)]
enum Index {
    Zero,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
}

impl From<Index> for u8 {
    fn from(item: Index) -> Self {
        match item {
            Index::Zero => 0,
            Index::One => 1,
            Index::Two => 2,
            Index::Three => 3,
            Index::Four => 4,
            Index::Five => 5,
            Index::Six => 6,
            Index::Seven => 7,
        }
    }
}

#[derive(Clone, Subcommand)]
enum Program {
    /// Turn on all of the LEDs
    AllOn,
    /// Cycle one layer at a time
    Cycle,
    /// Plane waves moving diagonally
    PlaneWave { reflect: Option<bool> },
    /// Flat wave
    Wave { rotate: Option<bool> },
    /// Turn on alternate LEDs like a chessboard
    Chess { invert: Option<bool> },
    /// Turn on one full layer of LEDs
    OneLayer { which: Index },
    /// Turn on one full row of LEDs
    OneRow { which: Index },
    /// Turn on one full column of LEDs
    OneCol { which: Index },
}

fn spawn_display() -> (SyncSender<Frame>, JoinHandle<rppal::gpio::Result<()>>) {
    let (tx, rx): (SyncSender<Frame>, Receiver<Frame>) = sync_channel(64);

    let handler = thread::spawn(move || {
        let mut driver = CubeDriver::try_new()?;

        let mut curr_frame = [[0; 8]; 8];

        loop {
            let maybe_frame = rx.try_recv();
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

fn test_all_on(stop_token: Arc<AtomicBool>) {
    let mut driver = CubeDriver::try_new().unwrap();

    while !stop_token.load(Ordering::Relaxed) {
        driver.write_frame([[255; 8]; 8]);
    }
}

fn test_one_row(row: Index, stop_token: Arc<AtomicBool>) {
    let mut driver = CubeDriver::try_new().unwrap();

    let layer_pattern: [u8; 8] =
        core::array::from_fn(|i| if i == u8::from(row).into() { 255 } else { 0 });

    let frame = [layer_pattern; 8];

    while !stop_token.load(Ordering::Relaxed) {
        driver.write_frame(frame);
    }
}

fn test_one_col(col: Index, stop_token: Arc<AtomicBool>) {
    let mut driver = CubeDriver::try_new().unwrap();

    let frame = [[1 << u8::from(col); 8]; 8];

    while !stop_token.load(Ordering::Relaxed) {
        driver.write_frame(frame);
    }
}

fn test_chess(invert: bool, stop_token: Arc<AtomicBool>) {
    let mut driver = CubeDriver::try_new().unwrap();

    let evens: u8 = 170;
    let odds: u8 = 85;

    let layer_pattern = core::array::from_fn(|i| if (i % 2 == 0) != invert { evens } else { odds });

    let frame = [layer_pattern; 8];

    while !stop_token.load(Ordering::Relaxed) {
        driver.write_frame(frame);
    }
}

fn test_one_layer(layer: Index, stop_token: Arc<AtomicBool>) {
    let mut driver = CubeDriver::try_new().unwrap();

    // let frame: [[u8; 8]; 8] = core::array::from_fn(|i| {
    //     if i == u8::from(layer).into() {
    //         [255; 8]
    //     } else {
    //         [0; 8]
    //     }
    // });

    while !stop_token.load(Ordering::Relaxed) {
        // driver.write_frame(frame);
        driver.test_layer(layer.into(), [255; 8]);
    }
}

fn test_cycle_layers(stop_token: Arc<AtomicBool>) {
    let (sender, handle) = spawn_display();

    let mut layer_cycle = once([255; 8]).chain(repeat([0; 8]).take(8)).cycle();

    while !stop_token.load(Ordering::Relaxed) {
        // Cycle through a window of 9 layers with one lit
        if sender
            .send([
                layer_cycle.next().unwrap(),
                layer_cycle.next().unwrap(),
                layer_cycle.next().unwrap(),
                layer_cycle.next().unwrap(),
                layer_cycle.next().unwrap(),
                layer_cycle.next().unwrap(),
                layer_cycle.next().unwrap(),
                layer_cycle.next().unwrap(),
            ])
            .is_err()
        {
            eprintln!("Failed to write layer");
            break;
        }

        thread::sleep(Duration::from_millis(100));
    }

    drop(sender);

    let _ = handle.join().expect("Could not join sender thread");
}

fn test_diag_plane_wave(reflect: bool, stop_token: Arc<AtomicBool>) {
    let (sender, handle) = spawn_display();

    let base: [u8; 8] = core::array::from_fn(|i| 1u8.rotate_left(i.try_into().unwrap()));

    let mut frame_cycle = (0u32..8u32)
        .map(|i| base.map(|row| row.rotate_left(i)))
        .chain((0u32..8u32).map(|i| base.map(|row| row.rotate_right(i))))
        .take(if reflect { 15 } else { 8 })
        .cycle();

    while !stop_token.load(Ordering::Relaxed) {
        if sender.send([frame_cycle.next().unwrap(); 8]).is_err() {
            eprintln!("Failed to write layer");
            break;
        }

        thread::sleep(Duration::from_millis(100));
    }

    drop(sender);

    let _ = handle.join().expect("Could not join sender thread");
}

fn test_flat_wave(rotate: bool, stop_token: Arc<AtomicBool>) {
    let (sender, handle) = spawn_display();

    let base: [u8; 8] = core::array::from_fn(|i| 1u8.rotate_left(i.try_into().unwrap()));

    let mut frame_cycle = (0u32..8u32)
        .map(|i| base.map(|row| row.rotate_left(i)))
        .chain((0u32..8u32).map(|i| base.map(|row| row.rotate_right(i))))
        .take(if rotate { 15 } else { 8 })
        .cycle();

    while !stop_token.load(Ordering::Relaxed) {
        if sender.send([frame_cycle.next().unwrap(); 8]).is_err() {
            eprintln!("Failed to write layer");
            break;
        }

        thread::sleep(Duration::from_millis(100));
    }

    drop(sender);

    let _ = handle.join().expect("Could not join sender thread");
}

fn main() {
    let args = Cli::parse();

    let stop_token = Arc::new(AtomicBool::new(false));
    let stop_token_clone = stop_token.clone();

    ctrlc::set_handler(move || {
        println!("Exiting...");
        stop_token_clone.store(true, Ordering::Relaxed);
    })
    .expect("Error setting Ctrl-C handler");

    match args.program {
        Program::AllOn => test_all_on(stop_token),
        Program::Cycle => test_cycle_layers(stop_token),
        Program::PlaneWave { reflect } => {
            test_diag_plane_wave(reflect.unwrap_or_default(), stop_token)
        }
        Program::Wave { rotate } => {
            test_flat_wave(rotate.unwrap_or_default(), stop_token)
        }
        Program::Chess { invert } => test_chess(invert.unwrap_or_default(), stop_token),
        Program::OneLayer { which: layer } => test_one_layer(layer, stop_token),
        Program::OneRow { which: row } => test_one_row(row, stop_token),
        Program::OneCol { which: col } => test_one_col(col, stop_token),
    };
}
