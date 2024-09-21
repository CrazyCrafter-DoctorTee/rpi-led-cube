mod cube;
mod routines;

use std::{
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

use routines::*;

type Frame = [[u8; 8]; 8];

/// Bit-bang the PI GPIO pins to render 3D values on the LED cube
#[derive(Parser)]
struct Cli {
    /// The display program to run
    #[command(subcommand)]
    program: Program,
}

#[derive(Copy, Clone, ValueEnum)]
/// Assume +X is "forward", +Y is "left", and +Z is "up", then
enum Rotation {
    /// No-op
    None,
    /// Rotate about X
    I,
    /// Rotate about Y
    J,
    /// Rotate about Z
    K,
}

impl Rotation {
    fn apply(&self, data: &[[u8; 8]; 8]) -> Frame {
        match self {
            Self::None => core::array::from_fn(|i| core::array::from_fn(|j| data[i][j])),
            Self::I => core::array::from_fn(|i| core::array::from_fn(|j| data[j][i])),
            Self::J => core::array::from_fn(|i| {
                core::array::from_fn(|j| {
                    data[j].into_iter().fold(0u8, |acc, e| {
                        (acc << 1) + if e & (1 << i) != 0 { 1 } else { 0 }
                    })
                })
            }),
            Self::K => core::array::from_fn(|i| {
                core::array::from_fn(|j| {
                    data[i].into_iter().fold(0u8, |acc, e| {
                        (acc << 1) + if e & (1 << j) != 0 { 1 } else { 0 }
                    })
                })
            }),
        }
    }
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
    /// Like rainfall
    Rain,
    /// Plane waves moving diagonally
    PlaneWave { reflect: Option<bool> },
    /// Flat wave
    Wave { rotate: Option<Rotation> },
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

fn run_routine<'a, I>(stop_token: Arc<AtomicBool>, frame_sleep: Duration, frames: I)
where
    I: IntoIterator<Item = Frame>,
{
    let (sender, handle) = spawn_display();

    for frame in frames {
        if stop_token.load(Ordering::Relaxed) {
            break;
        }

        if sender.send(frame).is_err() {
            eprintln!("Failed to write layer");
            break;
        }

        thread::sleep(frame_sleep);
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

    let ftime = Duration::from_millis(100);

    match args.program {
        Program::AllOn => run_routine(stop_token, ftime, AllOn::new()),
        Program::Cycle => run_routine(stop_token, ftime, CycleLayers::new()),
        Program::Rain => run_routine(stop_token, ftime, Rain::new()),
        Program::PlaneWave { reflect } => run_routine(
            stop_token,
            ftime,
            DiagonalPlane::new(reflect.unwrap_or_default()),
        ),
        Program::Wave { rotate } => run_routine(stop_token, ftime, Wave::new()),
        Program::Chess { invert } => {
            run_routine(stop_token, ftime, Chess::new(invert.unwrap_or_default()))
        }
        Program::OneLayer { which: layer } => run_routine(stop_token, ftime, OneLayer::new(layer)),
        Program::OneRow { which: row } => run_routine(stop_token, ftime, OneRow::new(row)),
        Program::OneCol { which: col } => run_routine(stop_token, ftime, OneCol::new(col)),
    };
}
