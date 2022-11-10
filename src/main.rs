use bresenham::Bresenham;
use image::{ImageBuffer, Pixel, Rgb, RgbImage};
use num::complex::Complex;
use plotters::prelude::*;
use std::sync::{Arc, Mutex};

const IMG_WIDTH: u32 = 3960;
const IMG_HEIGHT: u32 = 2160;

const BAILOUT_RADIUS: f32 = 2.0;
const BAILOUT_ITERATIONS: u32 = 1000;

fn complex_to_coordinate(c: Complex<f32>) -> (u32, u32) {
    (
        ((c.re / 2.0 + 1.0) / 2.0 * IMG_WIDTH as f32) as u32,
        ((1.0 - (c.im / (2.0 * IMG_HEIGHT as f32 / IMG_WIDTH as f32) + 1.0) / 2.0) * IMG_HEIGHT as f32) as u32
    )
}

fn coordinate_to_complex((x, y): (u32, u32)) -> Complex<f32> {
    Complex::new(
        2.0 * (x as f32 / IMG_WIDTH as f32 * 2.0 - 1.0),
        2.0 * IMG_HEIGHT as f32 / IMG_WIDTH as f32 * ((1.0 - y as f32 / IMG_HEIGHT as f32) * 2.0 - 1.0)
    )
}

fn compute_radius(img_buf: &RgbImage, theta: f32) -> f32 {
    let origin = complex_to_coordinate(Complex::new(0.0, 0.0));

    let end =
        complex_to_coordinate(Complex::new(2.0 * theta.cos(), 2.0 * theta.sin()));

    let mut last_member = origin;

    for (x, y) in Bresenham::new(
        (origin.0 as isize, origin.1 as isize),
        (end.0 as isize, end.1 as isize)
    ) {
        if img_buf.get_pixel(x as u32, y as u32).channels() != [255, 255, 255] {
            break;
        }

        last_member = (x as u32, y as u32);
    }

    coordinate_to_complex(last_member).norm()
}

fn compute_row(y: u32) -> Vec<Rgb<u8>> {
    let mut result = Vec::with_capacity(IMG_WIDTH as usize);

    for x in 0..IMG_WIDTH {
        let c = coordinate_to_complex((x, y));

        let mut z = Complex::new(0.0, 0.0);

        let mut i = 0;

        while z.norm() < BAILOUT_RADIUS && i < BAILOUT_ITERATIONS {
            z = z * z + c;

            i += 1;
        }

        result.push(
            Rgb::<u8>(
                if i < BAILOUT_ITERATIONS {
                    [0, 0, 0]
                } else {
                    [255, 255, 255]
                }
            )
        );
    }

    result
}

async fn compute_and_set_row(img_buf_mutex: Arc<Mutex<RgbImage>>, y: u32) {
    let row = compute_row(y);

    let mut img_buf = img_buf_mutex.lock().unwrap();

    for x in 0..IMG_WIDTH {
        img_buf.put_pixel(x, y, row[x as usize]);
    }
}

fn plot_polar(img_buf: &RgbImage) -> Result<(), Box<dyn std::error::Error>> {
    let domain = 0.0..std::f32::consts::PI * 2.0;
    let domain_size = 1000;

    let root = BitMapBackend::new("output_plot.png", (1280, 960)).into_drawing_area();

    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(domain.start..domain.end, 0.0f32..2.0f32)?;

    chart.configure_mesh().draw()?;

    chart.draw_series(
        LineSeries::new(
            (0..domain_size)
                .map(|i| domain.start + (domain.end - domain.start) / domain_size as f32 * i as f32)
                .map(|theta| (theta, compute_radius(&img_buf, theta))),

            &RED
        )
    )?;

    root.present()?;

    Ok(())
}

#[tokio::main]
async fn main() {
    let img_buf_mutex = Arc::new(Mutex::new(ImageBuffer::new(IMG_WIDTH, IMG_HEIGHT)));

    let mut futures = Vec::new();

    for y in 0..IMG_HEIGHT {
        futures.push(tokio::spawn(compute_and_set_row(img_buf_mutex.clone(), y)));
    }

    futures::future::join_all(futures).await;

    let img_buf = img_buf_mutex.lock().unwrap();

    img_buf.save("output_set.png").unwrap();

    plot_polar(&img_buf).unwrap();
}
