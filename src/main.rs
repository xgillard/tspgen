use std::{time::{UNIX_EPOCH, SystemTime}, fs::File, io::Write};

use osrm_client::{Location, TableRequestBuilder, NearestRequestBuilder, TableAnnotationRequest};
use rand::prelude::*;
use clap::Parser;
use rand_chacha::ChaChaRng;
use rand_distr::{Uniform, Normal};
use serde_json::json;

/// TspGen is a generator for realistic TSP instances where the cities to visit are gouped in clusters.
/// 
/// Generate instance in Belgium:
/// ```
/// ./target/release/tspgen  --min-longitude=2.376776  --max-longitude=5.91469  --min-latitude=50.2840167  --max-latitude=51.034368
/// ```
#[derive(Debug, Parser)]
#[command(author, version, about)]
struct TspGen {
    /// The number of cities that must be visited
    #[clap(short='n', long, default_value="10")]
    nb_cities: usize,
    /// The number of centroids that must be visited
    #[clap(short='c', long, default_value="3")]
    nb_centroids: usize,
    /// The std deviation between a city and its centroid
    #[clap(short='d', long, default_value="0.1")]
    std_dev: f32,
    /// The west most longitude allowed in this generation
    #[clap(long, default_value="-4.4744")]
    min_longitude: f32,
    /// The east most longitude allowed in this generation
    #[clap(long, default_value="8.1350")]
    max_longitude: f32,
    /// The south most longitude allowed in this generation
    #[clap(long, default_value="42.1958")]
    min_latitude: f32,
    /// The north most longitude allowed in this generation
    #[clap(long, default_value="51.0521")]
    max_latitude: f32,
    /// Name of the file where to generate the tsp instance
    #[clap(long)]
    inst: Option<String>,
    /// Name of the file where to generate the html visualisation of the instance
    #[clap(long)]
    html: Option<String>,
    /// Zoom for the html output
    #[clap(short, long, default_value="5")]
    zoom: usize,
    #[clap(short, long)]
    force_routable: bool,

    /// An optional seed to kickstart the instance generation
    #[clap(short='s', long)]
    seed: Option<u128>,
}

struct Generation {
    centroids:     Vec<Location>,
    destinations:  Vec<Location>,
    distances:     Vec<Vec<f32>>,
}

impl Generation {
    fn instance_text(&self) -> String {
        let mut result = String::new();
        result.push_str("c This instance has been generated with tspgen \n");
        result.push_str("c https://github.com/xgillard/tspgen           \n");
        result.push_str("c --- centroids -------------------------------\n");
        for c in self.centroids.iter() {
            result.push_str(&format!("c {:>10.5} {:>10.5}\n", c.longitude, c.latitude));
        }
        result.push_str("c --- cities ----------------------------------\n");
        for c in self.destinations.iter() {
            result.push_str(&format!("c {:>10.5} {:>10.5}\n", c.longitude, c.latitude));
        }
        result.push_str("c --- destinations -------------------------------\n");
        for i in 0..self.destinations.len() {
            for j in 0..self.destinations.len() {
                result.push_str(&format!("{:>15.5} ", self.distances[i][j]));
            }
            result.push('\n');
        }
        result
    }

    fn visualisation_text(&self, zoom: usize) -> String {
        let template = r#"
        <html>
            <head>
                <link rel="stylesheet" href="https://unpkg.com/leaflet@1.9.3/dist/leaflet.css"
                    integrity="sha256-kLaT2GOSpHechhsozzB+flnD+zUyjE2LlfWPgU04xyI="
                    crossorigin=""/>
                <script src="https://unpkg.com/leaflet@1.9.3/dist/leaflet.js"
                    integrity="sha256-WBkoXOwTeyKclOHuWtc+i2uENFpDZ9YPdf5Hf+D7ewM="
                    crossorigin=""></script>
            </head>
            <body>
                <div id="map" style="height: 100%; width: 100%; ">
                </div>
                <script>
                    var map = L.map('map').setView([{{center_lat}}, {{center_lon}}], {{zoom}});

                    L.tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png', {
                        maxZoom: 19,
                        attribution: '&copy; <a href="http://www.openstreetmap.org/copyright">OpenStreetMap</a>'
                    }).addTo(map);
                    
                    {{markers}}
                </script>
            </body>
        </html>
        "#;

        let center_lat = self.centroids.iter().map(|l| l.latitude).sum::<f32>() / self.centroids.len() as f32;
        let center_lon = self.centroids.iter().map(|l| l.longitude).sum::<f32>() / self.centroids.len() as f32;
        
        let mut markers = String::new();
        for Location { longitude, latitude } in self.destinations.iter() {
            markers.push_str(&format!("L.marker([{latitude}, {longitude}]).addTo(map);\n"));
        }

        let handlerbars = handlebars::Handlebars::new();
        handlerbars.render_template(template, &json!({
            "zoom": zoom,
            "center_lon": center_lon,
            "center_lat": center_lat,
            "markers": markers,
        })).unwrap()
    }
}

impl TspGen {
    /// This is the method you want to call in order to generate a clustered TSP instance
    async fn generate(&self) -> Generation {
        let mut rng = self.rng();
        let centroids = self.generate_centroids(&mut rng);
        let centroids = self.routable_cities(&centroids).await;
        let mut destinations = self.generate_cities(&mut rng, &centroids);
        if self.force_routable {
            destinations = self.routable_cities(&destinations).await;
        }

        let distances = self.travel_distance_matrix(&destinations).await;

        Generation{
            centroids,
            destinations,
            distances
        }
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
    fn generate_centroids(&self, rng: &mut impl Rng) -> Vec<Location> {
        let mut centroids = vec![];
        for _ in 0..self.nb_centroids {
            centroids.push(self.random_centroid(rng));
        }
        centroids
    }
    /// This method returns a new random centroid uniformly sampled from 0..max
    fn random_centroid(&self, rng: &mut impl Rng) -> Location {
        //
        let lon_dist = Uniform::new_inclusive(self.min_longitude, self.max_longitude);
        let lat_dist = Uniform::new_inclusive(self.min_latitude, self.max_latitude);
        let longitude = lon_dist.sample(rng);
        let latitude = lat_dist.sample(rng);
        Location { longitude, latitude }
    }
    /// This method returns a vector of random cities close to the centroids
    fn generate_cities(&self, rng: &mut impl Rng, centroids: &[Location]) -> Vec<Location> {
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
    fn random_pos_close_to(&self, rng: &mut impl Rng, Location{longitude, latitude}: Location) -> Location {
        let dist_x = Normal::new(longitude as f32, self.std_dev).expect("cannot create normal dist");
        let dist_y = Normal::new(latitude as f32, self.std_dev).expect("cannot create normal dist");
        let lon = dist_x.sample(rng);
        let lat = dist_y.sample(rng);
        Location { longitude: lon, latitude: lat }
    }
    


    // /// This method returns the euclidian distance between two cities
    // fn euclidian_distance(&self, a: Location, b: Location) -> f32 {
    //     let dx = a.longitude - b.longitude;
    //     let dy = a.latitude - b.latitude;
    //     let dx = dx * dx;
    //     let dy = dy * dy;
    // 
    //     (dx + dy).sqrt()
    // }
    // 
    // fn euclidian_distance_matrix(&self, cities: &[Location]) -> Vec<Vec<f32>> {
    //     let mut result = vec![];
    //     for a in cities.iter().copied() {
    //         let mut line = vec![];
    //         for b in cities.iter().copied() {
    //             line.push(self.euclidian_distance(a, b));
    //         }
    //         result.push(line);
    //     }
    //     result
    // }

    async fn routable_cities(&self, locations: &[Location]) -> Vec<Location> {
        let client = osrm_client::Client::default();
        
        let mut out = vec![];
        for loc in locations {
            let rsp = NearestRequestBuilder::default()
                .coordinates(osrm_client::Coordinates::Single(*loc))
                .build()
                .unwrap()
                .send(&client)
                .await
                .unwrap();
            
            let wp = &rsp.waypoints.unwrap()[0];
            out.push(wp.location);
        }
        out
    }

    async fn travel_distance_matrix(&self, locations: &[Location]) -> Vec<Vec<f32>>{
        let client = osrm_client::Client::default();

        let matrix = TableRequestBuilder::default()
            .coordinates(osrm_client::Coordinates::Multi(Vec::from_iter(locations.iter().copied())))
            .annotations(TableAnnotationRequest::Both)
            .build().unwrap()
            .send(&client)
            .await
            .unwrap();

        let mut result = vec![];
        for line in matrix.durations.unwrap().iter() {
            result.push(line.iter().map(|x| x.unwrap()).collect());
        }
        result
    }

}

#[tokio::main]
async fn main() {
    let cli = TspGen::parse();
    let tsp = cli.generate().await;

    if let Some(fname) = cli.inst {
        let mut out = File::create(fname).unwrap();
        out.write_all(tsp.instance_text().as_bytes()).unwrap();
    } else {
        println!("{}",  tsp.instance_text());
    }

    if let Some(fname) = cli.html {
        let mut out = File::create(fname).unwrap();
        out.write_all(tsp.visualisation_text(cli.zoom).as_bytes()).unwrap();
    }
}