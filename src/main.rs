mod cube;
mod decoders;
mod routines;

use std::{
    io::stdin,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{sync_channel, Receiver, RecvError, SyncSender, TryRecvError},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use clap::{Parser, Subcommand, ValueEnum};

use cube::CubeDriver;

use routines::*;

use crate::decoders::read_base16_frame;

/// Outer array is Z/layer, inner array is X/row, each bit is Y/column
type Frame = [[u8; 8]; 8];

/// Bit-bang the PI GPIO pins to render 3D values on the LED cube
#[derive(Parser)]
struct Cli {
    /// The display program to run
    #[command(subcommand)]
    program: Program,
    #[arg(long)]
    invert: bool,
    #[arg(long, default_value_t = Rotation::None)]
    rotate: Rotation,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
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

impl std::fmt::Display for Rotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("all values possible")
            .get_name()
            .fmt(f)
    }
}

impl Default for Rotation {
    fn default() -> Self {
        Rotation::None
    }
}

impl Rotation {
    fn apply(&self, data: &[[u8; 8]; 8]) -> Frame {
        match self {
            Self::None => data.clone(),
            Self::I => core::array::from_fn(|layer| {
                core::array::from_fn(|row| {
                    // Build a row from each of the bits in the corresponding layer
                    (0..8).map(|l| data[l][row]).fold(0u8, |acc, e| {
                        (acc << 1) + if e & (1 << layer) != 0 { 1 } else { 0 }
                    })
                })
            }),
            Self::J => {
                core::array::from_fn(|layer| core::array::from_fn(|row| data[row][7 - layer]))
            }
            Self::K => core::array::from_fn(|layer| {
                core::array::from_fn(|row| {
                    // Build row from each of the bits in the corresponding column
                    data[layer].into_iter().fold(0u8, |acc, e| {
                        (acc << 1) + if e & (1 << row) != 0 { 1 } else { 0 }
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
    /// Turn on a single LED
    OneOn {
        row: Index,
        col: Index,
        layer: Index,
    },
    /// Cycle one layer at a time
    Cycle,
    /// Like rainfall
    Rain,
    /// Plane waves moving diagonally
    PlaneWave { reflect: Option<bool> },
    /// Flat wave
    Wave,
    /// Turn on alternate LEDs like a chessboard
    Chess,
    /// Turn on one full layer of LEDs
    OneLayer { which: Index },
    /// Turn on one full row of LEDs
    OneRow { which: Index },
    /// Turn on one full column of LEDs
    OneCol { which: Index },
    /// Tiny cube in a cube
    MiniCube,
    /// Flip a random bit at a time
    RandomFlip,
    /// A fistful of lights
    LittleBlips,
    /// A moving snake
    Traveller,
    /// Read hexadecimal frame data from stdin
    Listener,
}

fn spawn_display() -> (SyncSender<Frame>, JoinHandle<rppal::gpio::Result<()>>) {
    let (tx, rx): (SyncSender<Frame>, Receiver<Frame>) = sync_channel(0);

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

/// Consumes frames at up to a fixed rate from the sender to display
fn spawn_ratelimited_display(
    frame_sleep: Duration,
    stop_token: Arc<AtomicBool>,
) -> (SyncSender<Frame>, JoinHandle<rppal::gpio::Result<()>>) {
    let (tx, rx): (SyncSender<Frame>, Receiver<Frame>) = sync_channel(64);

    let handler = thread::spawn(move || {
        let (sender, handle) = spawn_display();

        loop {
            if stop_token.load(Ordering::Relaxed) {
                break;
            }

            let maybe_frame = rx.recv();
            if let Ok(frame) = maybe_frame {
                if sender.send(frame).is_err() {
                    eprintln!("Failed to hand off layer");
                    break;
                }
            } else if let Err(RecvError) = maybe_frame {
                break;
            }
            thread::sleep(frame_sleep);
        }

        drop(sender);

        handle.join().expect("Could not join sender thread")
    });

    (tx, handler)
}

fn run_routine<'a, I>(
    stop_token: Arc<AtomicBool>,
    frame_sleep: Duration,
    frames: I,
    invert: bool,
    rotate: Rotation,
) where
    I: IntoIterator<Item = Frame>,
{
    let (sender, handle) = spawn_ratelimited_display(frame_sleep, stop_token);

    for frame in frames {
        let rotated = rotate.apply(&frame);
        let inverted = if invert {
            rotated.map(|layer| layer.map(|row| row ^ 0xff))
        } else {
            rotated
        };

        // Send fails when stop token triggers
        if sender.send(inverted).is_err() {
            break;
        }
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
        Program::AllOn => run_routine(stop_token, ftime, AllOn::new(), args.invert, args.rotate),
        Program::OneOn { row, col, layer } => run_routine(
            stop_token,
            ftime,
            OneOn::new(row, col, layer),
            args.invert,
            args.rotate,
        ),
        Program::Cycle => run_routine(
            stop_token,
            ftime,
            CycleLayers::new(),
            args.invert,
            args.rotate,
        ),
        Program::Rain => run_routine(stop_token, ftime, Rain::new(), args.invert, args.rotate),
        Program::PlaneWave { reflect } => run_routine(
            stop_token,
            ftime,
            DiagonalPlane::new(reflect.unwrap_or_default()),
            args.invert,
            args.rotate,
        ),
        Program::Wave => run_routine(stop_token, ftime, Wave::new(), args.invert, args.rotate),
        Program::Chess => run_routine(stop_token, ftime, Chess::new(), args.invert, args.rotate),
        Program::OneLayer { which: layer } => run_routine(
            stop_token,
            ftime,
            OneLayer::new(layer),
            args.invert,
            args.rotate,
        ),
        Program::OneRow { which: row } => run_routine(
            stop_token,
            ftime,
            OneRow::new(row),
            args.invert,
            args.rotate,
        ),
        Program::OneCol { which: col } => run_routine(
            stop_token,
            ftime,
            OneCol::new(col),
            args.invert,
            args.rotate,
        ),
        Program::MiniCube => {
            run_routine(stop_token, ftime, MiniCube::new(), args.invert, args.rotate)
        }
        Program::RandomFlip => run_routine(
            stop_token,
            ftime,
            RandomFlip::new(),
            args.invert,
            args.rotate,
        ),
        Program::LittleBlips => run_routine(
            stop_token,
            Duration::from_millis(200),
            LittleBlips::new(),
            args.invert,
            args.rotate,
        ),
        Program::Traveller => run_routine(
            stop_token,
            ftime,
            Traveller::new(),
            args.invert,
            args.rotate,
        ),
        // Broken, not respecting stop token...
        Program::Listener => run_routine(
            stop_token,
            ftime,
            stdin().lines().map(|l| l.ok().and_then(|s| read_base16_frame(&s).ok())).flatten(),
            args.invert,
            args.rotate,
        ),
    };
}
