//! This module comprises all utilities that are required to generate a random TSP instance
//! between places in the real world.

use std::{time::{SystemTime, UNIX_EPOCH}, fs::File, io::Write};

use clap::Args;
use osrm_client::{Location, NearestRequestBuilder, TableRequestBuilder, TableAnnotationRequest, Client};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use rand_distr::{Uniform, Normal, Distribution};

use crate::instance::Instance;


/// TspGen is a generator for realistic TSP instances where the cities to visit are gouped in clusters.
/// 
/// Generate instance in Belgium:
/// ```
/// ./target/release/tspgen  --min-longitude=2.376776  --max-longitude=5.91469  --min-latitude=50.2840167  --max-latitude=51.034368
/// ```
#[derive(Debug, Args)]
pub struct GenerateInstance {
    /// An optional seed to kickstart the instance generation
    #[clap(short='s', long)]
    pub seed: Option<u128>,

    /// The number of cities that must be visited
    #[clap(short='n', long, default_value="10")]
    pub nb_cities: usize,
    /// The number of centroids that must be visited
    #[clap(short='c', long, default_value="3")]
    pub nb_centroids: usize,
    /// The std deviation between a city and its centroid
    #[clap(short='d', long, default_value="0.1")]
    pub std_dev: f32,
    /// The west most longitude allowed in this generation
    #[clap(long, default_value="-4.4744")]
    pub min_longitude: f32,
    /// The east most longitude allowed in this generation
    #[clap(long, default_value="8.1350")]
    pub max_longitude: f32,
    /// The south most longitude allowed in this generation
    #[clap(long, default_value="42.1958")]
    pub min_latitude: f32,
    /// The north most longitude allowed in this generation
    #[clap(long, default_value="51.0521")]
    pub max_latitude: f32,
    /// Force all destinations to be routable (takes longer to generate an instance)
    #[clap(short, long)]
    pub force_routable: bool,
    /// Base the distance matrix on duration rather than distance
    #[clap(short='D', long)]
    pub duration: bool,

    /// Name of the file where to generate the tsp instance
    #[clap(short, long)]
    pub output: Option<String>,

    /// URL of the osrm server to use (optional)
    #[clap(short, long)]
    pub url_osrm: Option<String>,
}

impl GenerateInstance {
    /// Executes this command
    pub async fn execute(&self) {
        let mut client = osrm_client::Client::default();
        if let Some(url) = self.url_osrm.as_ref() {
            client = client.base_url(url.clone());
        }

        let instance  = self.generate(&client).await;
        let instance = serde_json::to_string_pretty(&instance).unwrap();

        if let Some(output) = self.output.as_ref() {
            File::create(output).unwrap().write_all(instance.as_bytes()).unwrap();
        } else {
            println!("{instance}");
        }
    }

    /// This is the method you want to call in order to generate a clustered TSP instance
    pub async fn generate(&self, client: &Client) -> Instance {
        let mut rng = self.rng();
        let centroids = self.generate_centroids(&mut rng);
        let centroids = self.routable_cities(client, &centroids).await;
        let mut destinations = self.generate_cities(&mut rng, &centroids);
        if self.force_routable {
            destinations = self.routable_cities(client, &destinations).await;
        }

        let distances = self.travel_cost_matrix(client, &destinations).await;

        Instance{
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
        let mut cities_per_centroids = vec![self.nb_cities / self.nb_centroids; self.nb_centroids];
        for i in 0..(self.nb_cities % self.nb_centroids) {
            cities_per_centroids[i] += 1;
        }

        let mut cities = vec![];
        for (i, centroid) in centroids.iter().copied().enumerate() {
            let n = cities_per_centroids[i];
            for _ in 0..n {
                cities.push(self.random_pos_close_to(rng, centroid));
            }
        }
        cities
    }

    /// This method returns a new city close to the given centroid
    fn random_pos_close_to(&self, rng: &mut impl Rng, Location{longitude, latitude}: Location) -> Location {
        let dist_x = Normal::new(longitude, self.std_dev).expect("cannot create normal dist");
        let dist_y = Normal::new(latitude,  self.std_dev).expect("cannot create normal dist");
        let lon = dist_x.sample(rng);
        let lat = dist_y.sample(rng);
        Location { longitude: lon, latitude: lat }
    }
    
    /// This method maps a set of location to the nearset routable point on the map.
    async fn routable_cities(&self, client: &Client, locations: &[Location]) -> Vec<Location> {
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

    /// This method computes the travel cost matrix between all the given locations. Depending
    /// on the 'duration' flag, this method will either return a matrix of durations (in seconds) 
    /// to reach each location from each other; or it will return the actual distance that is going 
    /// to be travelled (in metres).
    async fn travel_cost_matrix(&self, client: &Client, locations: &[Location]) -> Vec<Vec<f32>>{
        let matrix = TableRequestBuilder::default()
            .coordinates(osrm_client::Coordinates::Multi(Vec::from_iter(locations.iter().copied())))
            .annotations(TableAnnotationRequest::Both)
            .build().unwrap()
            .send(&client)
            .await
            .unwrap();

        let mut result = vec![];
        if self.duration {
            for line in matrix.durations.unwrap().iter() {
                result.push(line.iter().map(|x| x.unwrap()).collect());
            }
        } else {
            for line in matrix.distances.unwrap().iter() {
                result.push(line.iter().map(|x| x.unwrap()).collect());
            }
        }
        result
    }
}
