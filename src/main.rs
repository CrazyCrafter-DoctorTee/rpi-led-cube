mod cube;

use std::{iter::{once, repeat}, sync::mpsc::{self, sync_channel, Receiver, SyncSender, TryRecvError}, thread::{self, JoinHandle}, time::Duration};

use cube::CubeDriver;

type Frame = [[u8; 8]; 8];

const INTER_FRAME_SLEEP: Duration = Duration::from_millis(19); // 50ish FPS 

fn spawn_display() -> (SyncSender<Frame>, JoinHandle<()>) {
    let (tx, rx): (SyncSender<Frame>, Receiver<Frame>) = sync_channel(64);

    let handler = thread::spawn( move || {
        let mut driver = CubeDriver::try_new()?;

        let mut curr_frame = rx.recv()?;

        while let maybe_frame = rx.try_recv() {
            if let Ok(frame) = maybe_frame {
                curr_frame = frame;
            } else if let Err(TryRecvError::Disconnected) = maybe_frame {
                break;
            }

            driver.write_frame(curr_frame);
        }
    });

    (tx, handler)
}

fn test_all_on() {
    let (sender, handle) = spawn_display();

    loop {
        sender.send([[255; 8]; 8]);
    }
}

fn test_cycle_layers() {
    let (sender, handle) = spawn_display();

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
    println!("Hello, world!");
}
