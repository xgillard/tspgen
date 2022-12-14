use std::{time::{UNIX_EPOCH, SystemTime}};

use rand::prelude::*;
use clap::Parser;
use rand_chacha::ChaChaRng;
use rand_distr::{Uniform, Normal};

#[derive(Debug, Clone, Copy)]
struct Position {x: f32, y: f32}

/// TspGen is a generator for realistic TSP instances where the cities to visit are gouped in clusters.
#[derive(Debug, Parser)]
#[command(author, version, about)]
struct TspGen {
    /// The number of cities that must be visited
    #[clap(short='n', long, default_value="10")]
    nb_cities: usize,
    /// The number of centroids that must be visited
    #[clap(short='c', long, default_value="3")]
    nb_centroids: usize,
    /// The maximum width of the generated map
    #[clap(short='m', long, default_value="1000")]
    max: usize,
    /// The std deviation between a city and its centroid
    #[clap(short='d', long, default_value="10")]
    std_dev: usize,
    /// An optional seed to kickstart the instance generation
    #[clap(short='s', long)]
    seed: Option<u128>,
}

impl TspGen {
    /// This is the method you want to call in order to generate a clustered TSP instance
    fn generate(&self) -> String {
        let mut rng = self.rng();
        let centroids = self.generate_centroids(&mut rng);
        let cities = self.generate_cities(&mut rng, &centroids);

        let mut result = String::new();
        result.push_str("c This instance has been generated with tspgen \n");
        result.push_str("c https://github.com/xgillard/tspgen           \n");
        result.push_str("c --- centroids -------------------------------\n");
        for c in centroids.iter() {
            result.push_str(&format!("c {:>7.2} {:>7.2}\n", c.x, c.y));
        }
        result.push_str("c --- cities ----------------------------------\n");
        for c in cities.iter() {
            result.push_str(&format!("c {:>7.2} {:>7.2}\n", c.x, c.y));
        }
        result.push_str("c --- distances -------------------------------\n");
        for a in cities.iter().copied() {
            for b in cities.iter().copied() {
                result.push_str(&format!("{:>7.2} ", self.distance(a, b)));
            }
            result.push('\n');
        }
        result
    }
    
    /// This method returns an initialized random number generator
    fn rng(&self) -> impl Rng {
        let init = self.seed.unwrap_or_else(|| SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis());
        let mut seed = [0_u8; 32];
        seed.iter_mut().zip(init.to_be_bytes().into_iter()).for_each(|(s, i)| *s = i);
        seed.iter_mut().rev().zip(init.to_le_bytes().into_iter()).for_each(|(s, i)| *s = i);
        ChaChaRng::from_seed(seed)
    }

    /// This method returns a vector of random centroids for this instance
    fn generate_centroids(&self, rng: &mut impl Rng) -> Vec<Position> {
        let mut centroids = vec![];
        for _ in 0..self.nb_centroids {
            centroids.push(self.random_centroid(rng));
        }
        centroids
    }
    /// This method returns a new random centroid uniformly sampled from 0..max
    fn random_centroid(&self, rng: &mut impl Rng) -> Position {
        let dist = Uniform::new_inclusive(0 as f32, self.max as f32);
        let x = dist.sample(rng);
        let y = dist.sample(rng);
        Position { x, y }
    }
    /// This method returns a vector of random cities close to the centroids
    fn generate_cities(&self, rng: &mut impl Rng, centroids: &[Position]) -> Vec<Position> {
        let cities_per_centroids = self.nb_cities / self.nb_centroids;
        let cities_in_first_centroid = if cities_per_centroids * self.nb_centroids == self.nb_cities {
            cities_per_centroids
        } else {
            cities_per_centroids + 1
        };

        let mut cities = vec![];
        for (i, centroid) in centroids.iter().copied().enumerate() {
            let n = if i == 0 {cities_in_first_centroid} else {cities_per_centroids};
            for _ in 0..n {
                cities.push(self.random_pos_close_to(rng, centroid));
            }
        }
        cities
    }
    /// This method returns a new city close to the given centroid
    fn random_pos_close_to(&self, rng: &mut impl Rng, Position{x, y}: Position) -> Position {
        let dist_x = Normal::new(x as f32, self.std_dev as f32).expect("cannot create normal dist");
        let dist_y = Normal::new(y as f32, self.std_dev as f32).expect("cannot create normal dist");
        let x = dist_x.sample(rng);
        let y = dist_y.sample(rng);
        Position { x, y }
    }
    

    /// This method returns the euclidian distance between two cities
    fn distance(&self, a: Position, b: Position) -> f32 {
        let dx = a.x - b.x;
        let dy = a.y - b.y;
        let dx = dx * dx;
        let dy = dy * dy;

        (dx + dy).sqrt()
    }
}

fn main() {
    let tsp = TspGen::parse().generate();
    println!("{tsp}");
}