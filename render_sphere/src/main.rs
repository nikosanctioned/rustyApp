use crate::vec3::Vec3;
use rayon::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};

// Define basic 3D vector
mod vec3 {
    #[derive(Clone, Copy)]
    pub struct Vec3 {
        pub x: f64,
        pub y: f64,
        pub z: f64,
    }
    impl Vec3 {
        pub fn new(x: f64, y: f64, z: f64) -> Self {
            Self { x, y, z }
        }
        pub fn dot(&self, other: &Self) -> f64 {
            self.x * other.x + self.y * other.y + self.z * other.z
        }
        pub fn length(&self) -> f64 {
            self.dot(self).sqrt()
        }
        pub fn normalized(&self) -> Self {
            let len = self.length();
            Self::new(self.x / len, self.y / len, self.z / len)
        }
        pub fn sub(&self, other: &Self) -> Self {
            Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
        }
        pub fn add(&self, other: &Self) -> Self {
            Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
        }
        pub fn mul(&self, t: f64) -> Self {
            Self::new(self.x * t, self.y * t, self.z * t)
        }
        pub fn reflect(&self, n: &Self) -> Self {
            self.sub(&n.mul(2.0 * self.dot(n)))
        }
    }
}

// Material enum for different object properties
#[derive(Clone)] // Added Clone
enum Material {
    Diffuse(Vec3),   // Color for diffuse objects
    Reflective(f64), // Reflectivity factor (0.0 to 1.0)
}

// Scene objects
struct Sphere {
    center: Vec3,
    radius: f64,
    material: Material,
}

struct Ray {
    origin: Vec3,
    direction: Vec3,
}

// Hit record
struct Hit {
    t: f64,
    point: Vec3,
    normal: Vec3,
    material: Material,
}

// Scene hit detection for spheres
fn hit_sphere(sphere: &Sphere, ray: &Ray, t_min: f64, t_max: f64) -> Option<Hit> {
    let oc = ray.origin.sub(&sphere.center);
    let a = ray.direction.dot(&ray.direction);
    let b = 2.0 * oc.dot(&ray.direction);
    let c = oc.dot(&oc) - sphere.radius * sphere.radius;
    let discriminant: f64 = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return None;
    }
    let t = (-b - discriminant.sqrt()) / (2.0 * a);
    if t < t_min || t > t_max {
        return None;
    }
    let point = ray.origin.add(&ray.direction.mul(t));
    let normal = point.sub(&sphere.center).normalized();
    Some(Hit {
        t,
        point,
        normal,
        material: sphere.material.clone(), // Clone the material
    })
}

// Ground plane (infinite XZ plane at y = 0)
fn hit_ground(ray: &Ray, t_min: f64, t_max: f64) -> Option<Hit> {
    let t = -ray.origin.y / ray.direction.y;
    if t < t_min || t > t_max {
        return None;
    }
    let point = ray.origin.add(&ray.direction.mul(t));
    let normal = Vec3::new(0.0, 1.0, 0.0); // Upward normal
    Some(Hit {
        t,
        point,
        normal,
        material: Material::Diffuse(Vec3::new(0.8, 0.8, 0.8)), // Gray ground
    })
}

// Trace ray through the scene
fn trace(ray: &Ray, spheres: &[Sphere], depth: u32) -> Vec3 {
    if depth > 3 {
        // Max reflection depth
        return Vec3::new(0.0, 0.0, 0.0); // Black after too many bounces
    }

    let t_max = 1000.0;
    let mut closest_hit: Option<Hit> = None;
    let mut closest_t = t_max;

    // Check spheres
    for sphere in spheres {
        if let Some(hit) = hit_sphere(sphere, ray, 0.001, closest_t) {
            closest_t = hit.t;
            closest_hit = Some(hit);
        }
    }

    // Check ground
    if let Some(hit) = hit_ground(ray, 0.001, closest_t) {
        closest_hit = Some(hit);
    }

    match closest_hit {
        Some(hit) => match hit.material {
            Material::Diffuse(color) => {
                // Simple diffuse: just return the color
                color
            }
            Material::Reflective(reflectivity) => {
                // Reflect the ray
                let reflected_dir = ray.direction.reflect(&hit.normal);
                let reflected_ray = Ray {
                    origin: hit.point,
                    direction: reflected_dir.normalized(),
                };
                let reflected_color = trace(&reflected_ray, spheres, depth + 1);
                Vec3::new(
                    reflected_color.x * reflectivity,
                    reflected_color.y * reflectivity,
                    reflected_color.z * reflectivity,
                )
            }
        },
        None => {
            // Sky gradient
            let t = 0.5 * (ray.direction.y + 1.0);
            Vec3::new(1.0 - t + 0.5 * t, 1.0 - t + 0.7 * t, 1.0)
        }
    }
}
/// Adds one to the number given.
///
/// # Examples
///
/// ```
/// let arg = 5;
/// let answer = my_crate::add_one(arg);
///
/// assert_eq!(6, answer);
/// ```
fn main() {
    // Image setup
    let width = 400;
    let height = 200;
    let samples = 8; // More samples for smoother reflections
    let mut pixels = vec![Vec3::new(0.0, 0.0, 0.0); width * height];

    // Scene setup: multiple spheres
    let spheres = vec![
        Sphere {
            center: Vec3::new(0.0, 0.5, -1.0),
            radius: 0.5,
            material: Material::Reflective(0.9), // Shiny sphere
        },
        Sphere {
            center: Vec3::new(-1.0, 0.3, -0.8),
            radius: 0.3,
            material: Material::Diffuse(Vec3::new(0.9, 0.1, 0.1)), // Red sphere
        },
        Sphere {
            center: Vec3::new(1.0, 0.2, -0.7),
            radius: 0.2,
            material: Material::Diffuse(Vec3::new(0.1, 0.9, 0.1)), // Green sphere
        },
    ];

    let progress = AtomicUsize::new(0);

    // Camera setup
    let origin = Vec3::new(0.0, 0.5, 1.0); // Slightly elevated camera
    let lower_left = Vec3::new(-2.0, -1.0, -1.0);
    let horizontal = Vec3::new(4.0, 0.0, 0.0);
    let vertical = Vec3::new(0.0, 2.0, 0.0); // Fixed: completed the Vec3::new call

    // Parallel rendering
    pixels
        .par_chunks_mut(width)
        .enumerate()
        .for_each(|(j, row)| {
            for i in 0..width {
                let mut pixel_color = Vec3::new(0.0, 0.0, 0.0);
                for _s in 0..samples {
                    let u = (i as f64 + rand::random::<f64>()) / width as f64;
                    let v = (j as f64 + rand::random::<f64>()) / height as f64;
                    let direction = lower_left
                        .add(&horizontal.mul(u))
                        .add(&vertical.mul(v))
                        .sub(&origin)
                        .normalized();
                    let ray = Ray { origin, direction };
                    pixel_color = pixel_color.add(&trace(&ray, &spheres, 0));
                }
                row[i] = Vec3::new(
                    pixel_color.x / samples as f64,
                    pixel_color.y / samples as f64,
                    pixel_color.z / samples as f64,
                );
            }
            let completed = progress.fetch_add(1, Ordering::Relaxed) + 1;
            eprintln!(
                "Progress: {:.2}%",
                (completed as f64 / height as f64) * 100.0
            );
        });

    println!("P3\n{} {}\n255", width, height);
    for j in (0..height).rev() {
        for i in 0..width {
            let pixel = pixels[j * width + i];
            let r = (255.99 * pixel.x.clamp(0.0, 1.0)) as u8;
            let g = (255.99 * pixel.y.clamp(0.0, 1.0)) as u8;
            let b = (255.99 * pixel.z.clamp(0.0, 1.0)) as u8;
            println!("{} {} {}", r, g, b);
        }
    }
}
