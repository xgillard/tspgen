use std::{time::{UNIX_EPOCH, SystemTime}, fs::File, io::Write, collections::HashMap};

use handlebars::no_escape;
use osrm_client::{Location, TableRequestBuilder, NearestRequestBuilder, TableAnnotationRequest, TripRequestBuilder, Geometries, GeoJsonGeometry, GeoJsonPoint};
use rand::prelude::*;
use clap::Parser;
use rand_chacha::ChaChaRng;
use rand_distr::{Uniform, Normal};

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
    /// Force all destinations to be routable (takes longer to generate an instance)
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

    async fn visualisation_text(&self) -> String {
        let template = r#"
        <html>
            <head>
                <link rel="stylesheet" href="https://unpkg.com/leaflet@1.9.3/dist/leaflet.css"
                    integrity="sha256-kLaT2GOSpHechhsozzB+flnD+zUyjE2LlfWPgU04xyI="
                    crossorigin=""/>
                <script src="https://unpkg.com/leaflet@1.9.3/dist/leaflet.js"
                    integrity="sha256-WBkoXOwTeyKclOHuWtc+i2uENFpDZ9YPdf5Hf+D7ewM="
                    crossorigin=""></script>
                <script src="./Polyline.encoded.js"></script>
            </head>
            <body>
                <div id="map" style="height: 100%; width: 100%; ">
                </div>
                <script>
                    function markerIcon(name, color) {
                        const myCustomColour   = '#583470';
                        const markerHtmlStyles = `
                            position:         relative;
                            display:          block;
                            width:            2rem;
                            height:           2rem;
                            border:           3px solid white;
                            left:             -0.8rem;
                            top:              -2.6rem;
                            border-radius:    3rem 3rem 0;
                            transform:        rotate(45deg);
                            background-color: ${color};
                            text-align:       center;
                            `;
                        const middleDot = `
                            display: block; 
                            position: relative;
                            width: 5px; 
                            height: 5px; 
                            top: 0.75rem;
                            left: 0.75rem;
                            border-radius: 10px 10px 10px; 
                            background-color: white; 
                            border: 2px solid white;
                        `;
                        const icon = L.divIcon({
                        className: name,
                        html: `<div style="${markerHtmlStyles}">
                                <div style="${middleDot}">
                                </div>
                                </div>`
                        });
                        return icon;
                    }

                    const centroidPin    = markerIcon('centroid-icon',    '#ff0000');
                    const destinationPin = markerIcon('destination-icon', '#3366ff');

                    var map = L.map('map');
                    L.tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png', {
                        maxZoom: 19,
                        attribution: '&copy; <a href="http://www.openstreetmap.org/copyright">OpenStreetMap</a>'
                    }).addTo(map);

                    // -- var centroids = L.geoJSON({{centroids}}, {
                    // -- pointToLayer: function(feature, latlng) {
                    // --     return L.marker(latlng, {icon: centroidPin});
                    // -- },
                    // -- });
                    // -- centroids.addTo(map);

                    var cities = L.geoJSON({{cities}}, {
                    pointToLayer: function(feature, latlng) {
                        return L.marker(latlng, {icon: destinationPin});
                    },
                    });
                    cities.addTo(map);

                    var trip = L.geoJSON({{trip}}, {"color": "red"});
                    trip.addTo(map);

                    map.fitBounds(cities.getBounds());
                </script>
            </body>
        </html>
        "#;

        // FIXME
        let client = osrm_client::Client::default();
        let tour = TripRequestBuilder::default()
            .coordinates(osrm_client::Coordinates::Multi(self.destinations.clone()))
            .roundtrip(true)
            .geometries(Geometries::GeoJson)
            .source(Some(osrm_client::Source::Any))
            .destination(Some(osrm_client::Destination::Any))
            .build()
            .unwrap()
            .send(&client)
            .await
            .unwrap();
        
        let trip = serde_json::to_string(&tour.trips.unwrap()[0].geometry).unwrap();
        // FIXME

        let centroids = serde_json::to_string(&GeoJsonGeometry::MultiPoint { 
            coordinates: self.centroids.iter().copied().map(GeoJsonPoint::from).collect::<Vec<_>>()
        }).unwrap();

        let cities = serde_json::to_string(&GeoJsonGeometry::MultiPoint { 
            coordinates: self.destinations.iter().copied().map(GeoJsonPoint::from).collect::<Vec<_>>()
        }).unwrap();

        let mut handlebars = handlebars::Handlebars::new();
        handlebars.register_escape_fn(no_escape);
        let mut template_params = HashMap::<&'static str, String>::default();

        template_params.insert("trip",       trip);
        template_params.insert("cities",     cities);
        template_params.insert("centroids",  centroids);
        template_params.insert("showCentroids", "false".to_string());

        handlebars.render_template(template, &template_params).unwrap()
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
        let dist_x = Normal::new(longitude, self.std_dev).expect("cannot create normal dist");
        let dist_y = Normal::new(latitude,  self.std_dev).expect("cannot create normal dist");
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
        out.write_all(tsp.visualisation_text().await.as_bytes()).unwrap();
    }
}