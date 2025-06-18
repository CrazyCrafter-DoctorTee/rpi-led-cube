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

pub struct RandomFlip {
    rng: rand::rngs::SmallRng,
    state: Frame,
}

impl RandomFlip {
    pub fn new() -> Self {
        let evens: u8 = 0b10101010;
        let odds: u8 = 0b01010101;

        let a = [odds, evens, odds, evens, odds, evens, odds, evens];
        let b = [evens, odds, evens, odds, evens, odds, evens, odds];

        RandomFlip {
            rng: rand::rngs::SmallRng::from_entropy(),
            state: [a, b, a, b, a, b, a, b],
        }
    }
}

impl Iterator for RandomFlip {
    type Item = Frame;

    fn next(&mut self) -> Option<Frame> {
        let choice = self.rng.next_u32() as usize;
        let layer = choice % 8;
        let row = (choice >> 3) % 8;
        let mask = 1 << ((choice >> 6) % 8);

        self.state[layer][row] ^= mask;

        Some(self.state)
    }
}

pub struct LittleBlips {
    rng: rand::rngs::SmallRng,
}

impl LittleBlips {
    pub fn new() -> Self {
        LittleBlips {
            rng: rand::rngs::SmallRng::from_entropy(),
        }
    }

    fn gen_layer(&mut self) -> [u8; 8] {
        (self.rng.next_u64() & self.rng.next_u64() & self.rng.next_u64() & self.rng.next_u64())
            .to_be_bytes()
    }
}

impl Iterator for LittleBlips {
    type Item = Frame;

    fn next(&mut self) -> Option<Frame> {
        Some([
            self.gen_layer(),
            self.gen_layer(),
            self.gen_layer(),
            self.gen_layer(),
            self.gen_layer(),
            self.gen_layer(),
            self.gen_layer(),
            self.gen_layer(),
        ])
    }
}

pub struct Traveller {
    rng: rand::rngs::SmallRng,
    last_x: u8,
    last_y: u8,
    last_z: u8,
    current_x: u8,
    current_y: u8,
    current_z: u8,
}

impl Traveller {
    pub fn new() -> Self {
        Traveller {
            rng: rand::rngs::SmallRng::from_entropy(),
            last_x: 4,
            last_y: 4,
            last_z: 4,
            current_x: 4,
            current_y: 4,
            current_z: 3,
        }
    }

    fn pick_x(&self, dir: bool) -> (u8, u8, u8) {
        if self.current_x == 7 {
            (6, self.current_y, self.current_z)
        } else if self.current_x == 0 {
            (1, self.current_y, self.current_z)
        } else if dir {
            (self.current_x - 1, self.current_y, self.current_z)
        } else {
            (self.current_x + 1, self.current_y, self.current_z)
        }
    }

    fn pick_y(&self, dir: bool) -> (u8, u8, u8) {
        if self.current_y == 7 {
            (self.current_x, 6, self.current_z)
        } else if self.current_y == 0 {
            (self.current_x, 1, self.current_z)
        } else if dir {
            (self.current_x, self.current_y - 1, self.current_z)
        } else {
            (self.current_x, self.current_y + 1, self.current_z)
        }
    }

    fn pick_z(&self, dir: bool) -> (u8, u8, u8) {
        if self.current_z == 7 {
            (self.current_x, self.current_y, 6)
        } else if self.current_z == 0 {
            (self.current_x, self.current_y, 1)
        } else if dir {
            (self.current_x, self.current_y, self.current_z - 1)
        } else {
            (self.current_x, self.current_y, self.current_z + 1)
        }
    }

    /// Pick a new pixel to light up while "moving" orthogonally
    /// This needs to avoid backtracking and overflows
    fn pick(&mut self) -> (u8, u8, u8) {
        let choice = self.rng.next_u32();
        let dir = choice & 1 == 0;

        if self.last_x > self.current_x {
            // Hit the wall
            if self.current_x == 0 {
                if choice & 2 == 0 {
                    self.pick_y(dir)
                } else {
                    self.pick_z(dir)
                }
            } else {
                if choice & 2 == 0 {
                    if choice & 4 == 0 {
                        self.pick_y(dir)
                    } else {
                        self.pick_z(dir)
                    }
                } else {
                    (self.current_x - 1, self.current_y, self.current_z)
                }
            }
        } else if self.last_x < self.current_x {
            // Hit the wall
            if self.current_x == 7 {
                if choice & 2 == 0 {
                    self.pick_y(dir)
                } else {
                    self.pick_z(dir)
                }
            } else {
                if choice & 2 == 0 {
                    if choice & 4 == 0 {
                        self.pick_y(dir)
                    } else {
                        self.pick_z(dir)
                    }
                } else {
                    (self.current_x + 1, self.current_y, self.current_z)
                }
            }
        } else if self.last_y > self.current_y {
            // Hit the wall
            if self.current_y == 0 {
                if choice & 2 == 0 {
                    self.pick_x(dir)
                } else {
                    self.pick_z(dir)
                }
            } else {
                if choice & 2 == 0 {
                    if choice & 4 == 0 {
                        self.pick_x(dir)
                    } else {
                        self.pick_z(dir)
                    }
                } else {
                    (self.current_x, self.current_y - 1, self.current_z)
                }
            }
        } else if self.last_y < self.current_y {
            // Hit the wall
            if self.current_y == 7 {
                if choice & 2 == 0 {
                    self.pick_x(dir)
                } else {
                    self.pick_z(dir)
                }
            } else {
                if choice & 2 == 0 {
                    if choice & 4 == 0 {
                        self.pick_x(dir)
                    } else {
                        self.pick_z(dir)
                    }
                } else {
                    (self.current_x, self.current_y + 1, self.current_z)
                }
            }
        } else if self.last_z > self.current_z {
            // Hit the wall
            if self.current_z == 0 {
                if choice & 2 == 0 {
                    self.pick_x(dir)
                } else {
                    self.pick_y(dir)
                }
            } else {
                if choice & 2 == 0 {
                    if choice & 4 == 0 {
                        self.pick_x(dir)
                    } else {
                        self.pick_y(dir)
                    }
                } else {
                    (self.current_x, self.current_y, self.current_z - 1)
                }
            }
        } else {
            // Hit the wall
            if self.current_z == 7 {
                if choice & 2 == 0 {
                    self.pick_x(dir)
                } else {
                    self.pick_y(dir)
                }
            } else {
                if choice & 2 == 0 {
                    if choice & 4 == 0 {
                        self.pick_x(dir)
                    } else {
                        self.pick_y(dir)
                    }
                } else {
                    (self.current_x, self.current_y, self.current_z + 1)
                }
            }
        }
    }
}

impl Iterator for Traveller {
    type Item = Frame;

    fn next(&mut self) -> Option<Frame> {
        let (next_x, next_y, next_z) = self.pick();

        let mut frame: Frame = [[0; 8]; 8];

        frame[self.last_z as usize][self.last_x as usize] |= 1 << self.last_y;
        frame[self.current_z as usize][self.current_x as usize] |= 1 << self.current_y;
        frame[next_z as usize][next_x as usize] |= 1 << next_y;

        self.last_x = self.current_x;
        self.last_y = self.current_y;
        self.last_z = self.current_z;

        self.current_x = next_x;
        self.current_y = next_y;
        self.current_z = next_z;

        Some(frame)
    }
}
