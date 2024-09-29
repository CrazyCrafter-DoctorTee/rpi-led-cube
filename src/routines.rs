type Frame = [[u8; 8]; 8];

use std::iter::{once, repeat};

use crate::Index;

use rand::{RngCore, SeedableRng};

pub struct AllOn {}

impl AllOn {
    pub fn new() -> Self {
        AllOn {}
    }
}

impl IntoIterator for AllOn {
    type Item = Frame;
    type IntoIter = std::iter::Repeat<Frame>;

    fn into_iter(self) -> Self::IntoIter {
        repeat([[255; 8]; 8])
    }
}

pub struct OneOn {
    row: u8,
    col: u8,
    layer: u8,
}

impl OneOn {
    pub fn new(row_: Index, col_: Index, layer_: Index) -> Self {
        OneOn {
            row: row_.into(),
            col: col_.into(),
            layer: layer_.into(),
        }
    }
}

impl IntoIterator for OneOn {
    type Item = Frame;
    type IntoIter = std::iter::Repeat<Frame>;

    fn into_iter(self) -> Self::IntoIter {
        use std::ops::Shl;

        let mut frame = [[0u8; 8]; 8];

        frame[self.layer as usize][self.row as usize] = 1u8.shl(self.col);

        repeat(frame)
    }
}

pub struct OneRow {
    row: u8,
}

impl OneRow {
    pub fn new(row_: Index) -> Self {
        OneRow { row: row_.into() }
    }
}

impl IntoIterator for OneRow {
    type Item = Frame;
    type IntoIter = std::iter::Repeat<Frame>;

    fn into_iter(self) -> Self::IntoIter {
        let layer_pattern: [u8; 8] = core::array::from_fn(|i| {
            if i == u8::from(self.row).into() {
                255
            } else {
                0
            }
        });
        let frame = [layer_pattern; 8];
        repeat(frame)
    }
}

pub struct OneCol {
    col: u8,
}

impl OneCol {
    pub fn new(col_: Index) -> Self {
        OneCol { col: col_.into() }
    }
}

impl IntoIterator for OneCol {
    type Item = Frame;
    type IntoIter = std::iter::Repeat<Frame>;

    fn into_iter(self) -> Self::IntoIter {
        let frame = [[1 << u8::from(self.col); 8]; 8];

        repeat(frame)
    }
}

pub struct OneLayer {
    layer: u8,
}

impl OneLayer {
    pub fn new(layer_: Index) -> Self {
        OneLayer {
            layer: layer_.into(),
        }
    }
}

impl IntoIterator for OneLayer {
    type Item = Frame;
    type IntoIter = std::iter::Repeat<Frame>;

    fn into_iter(self) -> Self::IntoIter {
        let frame: Frame = core::array::from_fn(|i| {
            if i == u8::from(self.layer).into() {
                [255; 8]
            } else {
                [0; 8]
            }
        });

        repeat(frame)
    }
}

pub struct Chess {}

impl Chess {
    pub fn new() -> Self {
        Chess {}
    }
}

impl IntoIterator for Chess {
    type Item = Frame;
    type IntoIter = std::iter::Repeat<Frame>;

    fn into_iter(self) -> Self::IntoIter {
        let evens: u8 = 0b10101010;
        let odds: u8 = 0b01010101;

        let layer_pattern = core::array::from_fn(|i| if i % 2 == 0 { evens } else { odds });

        let frame = [layer_pattern; 8];

        repeat(frame)
    }
}

pub struct CycleLayers {
    layer_cycle: std::iter::Cycle<
        std::iter::Chain<std::iter::Once<[u8; 8]>, std::iter::Take<std::iter::Repeat<[u8; 8]>>>,
    >,
}

impl CycleLayers {
    pub fn new() -> Self {
        CycleLayers {
            layer_cycle: once([255; 8]).chain(repeat([0; 8]).take(8)).cycle(),
        }
    }
}

impl Iterator for CycleLayers {
    type Item = Frame;

    fn next(&mut self) -> Option<Self::Item> {
        // Cycle through a window of 9 layers with one lit
        Some([
            self.layer_cycle.next().unwrap(),
            self.layer_cycle.next().unwrap(),
            self.layer_cycle.next().unwrap(),
            self.layer_cycle.next().unwrap(),
            self.layer_cycle.next().unwrap(),
            self.layer_cycle.next().unwrap(),
            self.layer_cycle.next().unwrap(),
            self.layer_cycle.next().unwrap(),
        ])
    }
}

pub struct DiagonalPlane {
    reflect: bool,
    frames: [Frame; 15],
}

impl DiagonalPlane {
    pub fn new(reflect: bool) -> Self {
        let base: [u8; 8] = core::array::from_fn(|i| 1u8.rotate_left(i.try_into().unwrap()));

        let frames: [Frame; 15] = core::array::from_fn(|i| {
            [base.map(|row| row.rotate_left(if i < 8 { i as u32 } else { 15 - i as u32 })); 8]
        });

        DiagonalPlane { reflect, frames }
    }
}

impl IntoIterator for DiagonalPlane {
    type Item = Frame;
    type IntoIter = std::iter::Cycle<std::iter::Take<std::array::IntoIter<[[u8; 8]; 8], 15>>>;

    fn into_iter(self) -> Self::IntoIter {
        self.frames
            .into_iter()
            .take(if self.reflect { 15 } else { 8 })
            .cycle()
    }
}

pub struct Rain {
    rng: rand::rngs::SmallRng,
    memory: Frame,
    head: usize,
}

impl Rain {
    pub fn new() -> Self {
        let rng = rand::rngs::SmallRng::from_entropy();

        let memory = [[0u8; 8]; 8];
        let head = 0usize;

        Rain { rng, memory, head }
    }
}

impl Iterator for Rain {
    type Item = Frame;

    fn next(&mut self) -> Option<Frame> {
        self.memory[self.head] =
            (self.rng.next_u64() & self.rng.next_u64() & self.rng.next_u64() & self.rng.next_u64())
                .to_be_bytes();
        self.head = (self.head + 1) % 8;

        Some(core::array::from_fn(|i| {
            self.memory[(self.head + i) % self.memory.len()]
        }))
    }
}

pub struct Wave {
    i: usize,
}

impl Wave {
    pub fn new() -> Self {
        Wave { i: 0 }
    }
}

impl Iterator for Wave {
    type Item = Frame;

    fn next(&mut self) -> Option<Frame> {
        let template: [[u8; 12]; 8] = [
            [0, 0, 0, 0, 0, 255, 255, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 255, 0, 0, 255, 0, 0, 0, 0],
            [0, 0, 0, 255, 0, 0, 0, 0, 255, 0, 0, 0],
            [0, 0, 0, 255, 0, 0, 0, 0, 255, 0, 0, 0],
            [0, 0, 255, 0, 0, 0, 0, 0, 0, 255, 0, 0],
            [0, 0, 255, 0, 0, 0, 0, 0, 0, 255, 0, 0],
            [0, 255, 0, 0, 0, 0, 0, 0, 0, 0, 255, 0],
            [255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255],
        ];

        // LCM 8, 12 = 24
        let old_i = self.i;
        self.i = (self.i + 1) % 96;

        Some(core::array::from_fn(|layer| {
            core::array::from_fn(|j| template[layer][(old_i + j) % template[layer].len()])
        }))
    }
}

pub struct MiniCube {}

impl MiniCube {
    pub fn new() -> Self {
        MiniCube {}
    }
}

impl IntoIterator for MiniCube {
    type Item = Frame;
    type IntoIter = std::iter::Repeat<Frame>;

    fn into_iter(self) -> Self::IntoIter {
        repeat([
            [255, 129, 129, 129, 129, 129, 129, 255],
            [129, 66, 0, 0, 0, 0, 66, 129],
            [129, 0, 60, 36, 36, 60, 0, 129],
            [129, 0, 36, 0, 0, 36, 0, 129],
            [129, 0, 36, 0, 0, 36, 0, 129],
            [129, 0, 60, 36, 36, 60, 0, 129],
            [129, 66, 0, 0, 0, 0, 66, 129],
            [255, 129, 129, 129, 129, 129, 129, 255],
        ])
    }
}
