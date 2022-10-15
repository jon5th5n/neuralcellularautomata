use std::{time::{SystemTime, UNIX_EPOCH}, collections::VecDeque};
use minifb::{Key, Window, WindowOptions};
use rand::Rng;

const WIDTH: usize = 600;
const HEIGHT: usize = 600;

const COLOR: [u8; 3] = [104, 212, 134];

fn main() {
    let mut frame_time: u128 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    let mut deltas: VecDeque<u32> = VecDeque::from(vec![0; 10]);

    let mut frame: u128 = 0;

    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    let mut window = Window::new(
        "Test - ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    // Limit to max ~60 fps update rate
    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    //===================================================================================
    let mut rng = rand::thread_rng();

    let activation = |x: f32| {
        // -1.0 / f32::powf(2.0, 0.6*f32::powf(x, 2.0)) + 1.0
        (1.2 * x).abs()
    };

    let mut nca = NCA::new(WIDTH, HEIGHT, activation);
    for i in 0..WIDTH {
        for j in 0..HEIGHT {
            nca.canvas.set(i, j, rng.gen());
        }
    }

    nca.load_filter(1, &[
        0.565, -0.716, 0.565,
        -0.716, 0.627, -0.716,
        0.565, -0.716, 0.565
        ]);


    while window.is_open() && !window.is_key_down(Key::Escape) {
        frame += 1;

        nca.step();
        
        if frame % 2 == 0 {
            let nca_buffer = nca.canvas.get_buffer();
            for i in 0..WIDTH {
                for j in 0..HEIGHT {
                    buffer[j * WIDTH + i] = rgb_as_u32((COLOR[0] as f32 * nca_buffer[i][j]) as u8, (COLOR[1] as f32 * nca_buffer[i][j]) as u8, (COLOR[2] as f32 * nca_buffer[i][j]) as u8)
                }
            }

            // We unwrap here as we want this code to exit if it fails. Real applications may want to handle this in a different way
            window
                .update_with_buffer(&buffer, WIDTH, HEIGHT)
                .unwrap();
        }

        // -- calculate execution time ---
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
        let delta = now - frame_time;
        frame_time = now;
        deltas.pop_front();
        deltas.push_back(delta as u32);
        let mut _delta_avg: u32 = 0;
        for i in &deltas {
            _delta_avg += *i;
        }
        _delta_avg /= deltas.len() as u32;
        // -----
    }
}

fn rgb_as_u32(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) |
    ((g as u32) << 8) |
    ((b as u32) << 0)
}

struct Canvas {
    width: usize,
    height: usize,
    buffer: Vec<Vec<f32>>
}
impl Canvas {
    fn new(width: usize, height: usize) -> Self {
        Self { width, height, buffer: vec![vec![0.0; height]; width] }
    }

    fn get(&self, x: usize, y: usize) -> f32 {
        self.buffer[x][y]
    }
    fn get_buffer(&self) -> Vec<Vec<f32>> {
        self.buffer.clone()
    }

    fn set(&mut self, x: usize, y: usize, value: f32) {
        self.buffer[x][y] = value;
    }
}

struct Filter {
    size: usize,
    filter: Vec<Vec<f32>>
}
impl Filter {
    fn new() -> Self {
        Self { size: 1, filter: vec![vec![0.0; 3]; 3] }
    }
    fn new_from(size: usize, f: &[f32]) -> Self {
        let s = size * 2 + 1;

        if s.pow(2) != f.len() {
            panic!("size and length of filter do not match")
        }

        let mut filter: Vec<Vec<f32>> = vec![vec![0.0; s]; s];
        for i in 0..f.len() {
            filter[i % s][i / s] = f[i].clamp(-1.0, 1.0);
        }

        Self { size, filter }
    }

    fn load(&mut self, size: usize, f: &[f32]) {
        let s = size * 2 + 1;

        if s.pow(2) != f.len() {
            panic!("size and length of filter do not match")
        }

        let mut filter: Vec<Vec<f32>> = vec![vec![0.0; s]; s];
        for i in 0..f.len() {
            filter[i % s][i / s] = f[i].clamp(-1.0, 1.0);
        }

        self.size = size;
        self.filter = filter;
    }

    fn get_span(&self) -> usize {
        self.size * 2 + 1
    }
}

struct NCA<F> where F: Fn(f32) -> f32 {
    canvas: Canvas,
    filter: Filter,
    activation: F,
}
impl<F> NCA<F> where F: Fn(f32) -> f32 + Send + 'static + Copy + std::marker::Sync {
    fn new(width: usize, height: usize, activation: F) -> Self {
        Self { canvas: Canvas::new(width, height), filter: Filter::new(), activation }
    }

    fn load_filter(&mut self, size: usize, f: &[f32]) {
        self.filter.load(size, f);
    }

    fn set_activation(&mut self, activation: F) {
        self.activation = activation;
    }

    fn step(&mut self) {
        let canvas_buffer_cpy = self.canvas.get_buffer();

        // iterate over canvas
        for x in 0..self.canvas.width {
            for y in 0..self.canvas.height {
                let mut new_val: f32 = 0.0;

                // iterate over filter
                for i in 0..self.filter.get_span() {
                    for j in 0..self.filter.get_span() {
                        let mut k = x as isize + (i as isize - self.filter.size as isize);
                        if k < 0 {
                            k = self.canvas.width as isize - 1;
                        }
                        if k >= self.canvas.width as isize {
                            k = 0;
                        }
                        let mut l = y as isize + (j as isize - self.filter.size as isize);
                        if l < 0 {
                            l = self.canvas.height as isize - 1;
                        }
                        if l >= self.canvas.height as isize {
                            l = 0;
                        }

                        new_val += canvas_buffer_cpy[k as usize][l as usize] * self.filter.filter[i][j];
                    }
                }

                new_val = (self.activation)(new_val);

                self.canvas.set(x, y, new_val.clamp(0.0, 1.0));
            }
        }
    }
}