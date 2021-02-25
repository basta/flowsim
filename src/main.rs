#![forbid(unsafe_code)]

use core::f32;
use std::{cmp::{max, min}, u64};

use log::error;
use num_traits::Pow;
use pixels::{Error, Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

use rand::thread_rng;
use rand_distr::Normal;
use rand_distr::*;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 800;

const MICRO: i32 = 1000000;
const PI: f32 = 3.141592653589793238;
const DYN_VISCOSITY: f32 = 18.1/MICRO as f32;

const GRADIENT_HIGH: (u8, u8, u8) = (168, 50, 121);
const GRADIENT_LOW: (u8, u8, u8) = (211, 131, 18);

const TIME_STEP: f32 = 0.0166 * 0.01;
const AMOUNT_OF_DROPLETS: i32 = 10000;
/// Representation of the application state. In this example, a box will bounce around the screen.
struct World {
    bound_x: (i16, i16),
    bound_y: (i16, i16),
    start: (i32, i32),
    real_size: (i32, i32),
    droplets: Vec<Droplet>,
}
impl World {
    /// Create a new `World` instance that can draw a moving box.
    fn new() -> Self {
        Self {
            bound_x: (50, 800 - 50),
            bound_y: (50, 800 - 50),
            real_size: (40000000, 40000000),
            droplets: Vec::new(),
            start: (20000000, 20000000),
        }
    }

    fn average_droplet_speed(&self) -> f32{
        let mut sum = 0.0;
        for droplet in self.droplets.iter(){
            let xspeed = droplet.speed.0;
            let yspeed = droplet.speed.1;
            sum += (xspeed.pow(2.0) + yspeed.pow(2.0)).sqrt();
        }
        return sum/self.droplets.len() as f32;
    }

    fn create_droplets(&mut self, n: i32) {
        //n is multiple of 110
        let distributors = vec![
            Normal::new(3.0, 1.62),
            Normal::new(6.0, 8.94),
            Normal::new(12.0, 4.67),
            Normal::new(20.0, 4.07),
            Normal::new(28.0, 2.36),
            Normal::new(36.0, 1.03),
            Normal::new(45.0, 0.9),
            Normal::new(62.5, 0.98),
            Normal::new(87.5, 0.65),
            Normal::new(112.5, 1.01),
            Normal::new(137.5, 1.03),
            Normal::new(175.0, 1.01),
            Normal::new(225.0, 1.82),
            Normal::new(375.0, 0.5),
            Normal::new(750.0, 0.82),
            Normal::new(1500.0, 0.0),
        ];
        let amounts = [2, 27, 9, 5, 3, 2, 2, 2, 1, 2, 2, 2, 2, 1, 1, 0];

        let speed_normal = Normal::new(11.7 * MICRO as f32, 2.0 * MICRO as f32).unwrap(); // Find better deviations
        for i in 0..(distributors.len()) {
            let normal: Normal<f32> = distributors[i].unwrap();
            let amount = amounts[i];

            let rng = &mut thread_rng();
            let rng2 = &mut thread_rng();
            let rng3 = &mut thread_rng();

            let iter = normal.sample_iter(rng);

            let mut counter = 0;
            for sample in iter {
                self.droplets.push(Droplet {
                    position: (self.start.0, self.start.1, 0),
                    size: f32::max(sample, 0.2),
                    speed: (speed_normal.sample(rng2), speed_normal.sample(rng3)),
                });
                counter += 1;
                // println!("Distributor: {}, sample: {}", i, counter);
                if counter > amount * n {
                    break;
                }
            }
        }
        println!("Created {} droplets.", n*110);
    }

    fn draw_droplets(&mut self, frame: &mut [u8]){
        for droplet in self.droplets.iter(){
            let pos = droplet.get_pixel(self);
            println!("x:{} y:{}", pos.0, pos.1);
            let x = std::cmp::min((pos.0 * 4 -1) as u32, WIDTH*4); // x location in frame (*4 because each pixel is 4*u8)
            let y = std::cmp::min((pos.1 -1) as u32, HEIGHT);
            let color = droplet.get_color();
            let index = (4*y as u32*(WIDTH as u32) + x as u32) as usize;
            let index = std::cmp::min(index, frame.len()-4);
            frame[index] = color.0;
            frame[index +1] = color.1;
            frame[index +2] = color.2;
            frame[index +3] = color.3;
        }
    }
    /// Move the droplets
    fn update(&mut self) {
        for droplet in self.droplets.iter_mut(){
            droplet.step(TIME_STEP);
        }
        println!("{}", self.average_droplet_speed());
    }

    /// Draw the `World` state to the frame buffer.
    ///
    /// Assumes the default texture format: `wgpu::TextureFormat::Rgba8UnormSrgb`
    fn draw(&mut self, frame: &mut [u8]) {
        // for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
        //     let x = (i % WIDTH as usize) as i16;
        //     let y = (i / WIDTH as usize) as i16;

        //     let inside_the_box = x >= self.bound_x.0
        //         && x < self.bound_x.1
        //         && y >= self.bound_y.0
        //         && y < self.bound_y.1;

        //     let rgba = if inside_the_box {
        //         [255, 255, 255, 0xff]
        //     } else {
        //         [0, 0, 0, 1]
        //     };

        //     pixel.copy_from_slice(&rgba);
        // }
        for element in frame.iter_mut(){*element = 0_u8;}

        self.draw_droplets(frame);
    }
}

struct Droplet {
    position: (i32, i32, i32), // x,y,z
    speed: (f32, f32),         // m/s
    size: f32,                 // Radius in mikrom
}

impl Droplet {
    fn mass(&self) -> f32{
        // return mass in kg
        let V = (self.size as f32 / MICRO as f32).pow(3) as f32*PI*4.0/3.0;
        let density = 997.0;
        V*density //in kg
    }

    fn step(&mut self, delta: f32){
        // Apply friction
        let mass = self.mass();
        let f_x = stokes(self.speed.0/MICRO as f32, self.size/MICRO as f32);
        let f_y = stokes(self.speed.1/MICRO as f32, self.size/MICRO as f32);
        let a_x = (f_x/mass) * MICRO as f32;
        let a_y = (f_y/mass) * MICRO as f32;    
    

        self.speed.0 = self.speed.0 - a_x * TIME_STEP;//TODO, i calculate mass as if it was only water
        self.speed.1 = self.speed.1 - a_y * TIME_STEP;

        let xdiff = (self.speed.0 * delta) as i32;
        let ydiff = (self.speed.1 * delta) as i32;
        
        println!("xdiff:{} ydiff:{} size:{}", xdiff, ydiff, self.size);
        self.position.0 = self.position.0 + xdiff; 
        self.position.1 = self.position.1 + ydiff;
    }

    fn get_pixel(&self, world: &World) -> (i32, i32){
        let x: i32 = ((self.position.0 as f32 / world.real_size.0 as f32) * WIDTH as f32) as i32;
        let y: i32 = ((self.position.1 as f32 / world.real_size.1 as f32) * HEIGHT as f32) as i32;
        let x = min(WIDTH as i32, x);
        let y = min(WIDTH as i32, y);
         (x,y)
    }

    fn get_color(&self) -> (u8, u8, u8, u8){
        // percentage how far from gradient low
        let ratio = self.size.sqrt()/100.0;
        let ratio = f32::min(ratio, 1.0);
        (
            GRADIENT_LOW.0 + ((GRADIENT_HIGH.0 as isize - GRADIENT_LOW.0 as isize) as f32 * ratio) as u8,
            GRADIENT_LOW.1 + ((GRADIENT_HIGH.1 as isize - GRADIENT_LOW.1 as isize) as f32 * ratio) as u8,
            GRADIENT_LOW.2 + ((GRADIENT_HIGH.2 as isize - GRADIENT_LOW.2 as isize) as f32 * ratio) as u8,
            1 as u8,
        )
    }
}

fn stokes(speed: f32, size: f32) -> f32 {
    6.0 * PI * DYN_VISCOSITY * speed * size
}

fn main() -> Result<(), Error> {
    println!("Creating window");

    env_logger::init();
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    print!(".");
    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        let mut window = WindowBuilder::new();
        print!(".");
        window = window.with_title("Flowsim");
        window = window.with_inner_size(size);
        window = window.with_min_inner_size(size);
        window.build(&event_loop).unwrap()
    };

    println!("DONE");
    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(WIDTH, HEIGHT, surface_texture)?
    };
    let mut world = World::new();
    println!("Creating droplets...");
    world.create_droplets(AMOUNT_OF_DROPLETS / 110);
    println!("DONE");
    event_loop.run(move |event, _, control_flow| {
        // Draw the current frame
        if let Event::RedrawRequested(_) = event {
            world.draw(pixels.get_frame());
            if pixels
                .render()
                .map_err(|e| error!("pixels.render() failed: {}", e))
                .is_err()
            {
                *control_flow = ControlFlow::Exit;
                return;
            }
        }

        // Handle input events
        if input.update(&event) {
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            }

            // Resize the window
            if let Some(size) = input.window_resized() {
                pixels.resize(size.width, size.height);
            }

            // Update internal state and request a redraw
            world.update();
            window.request_redraw();
        }
    });
}
