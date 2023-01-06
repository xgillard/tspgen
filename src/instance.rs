use osrm_client::{Location, GeoJsonGeometry, GeoJsonPoint};
use serde::{Serialize, Deserialize};

/// A TSP instance that knows the gps coordinates of the destinations that must
/// be visited along with the distances to travel from one city to the other.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    /// The gps coordinates of the places that must be visited.
    pub destinations: Vec<Location>,
    /// The distance (in metres) between all pairs of destinations
    pub distances: Vec<Vec<f32>>,
}

impl Instance {
    /// Generates a string corresponding a description of the instance in the form 
    /// which is usually used to encode TSP instances
    #[allow(dead_code)]
    pub fn instance_text(&self) -> String {
        let mut result = String::new();
        result.push_str("c This instance has been generated with tspgen \n");
        result.push_str("c https://github.com/xgillard/tspgen           \n");
        result.push_str("c --- destinations ----------------------------\n");
        for c in self.destinations.iter() {
            result.push_str(&format!("c {:>10.5} {:>10.5}\n", c.longitude, c.latitude));
        }
        result.push_str("c --- distances -------------------------------\n");
        for i in 0..self.destinations.len() {
            for j in 0..self.destinations.len() {
                result.push_str(&format!("{:>15.5} ", self.distances[i][j]));
            }
            result.push('\n');
        }
        result
    }

    /// Returns a geojson multipoint geometry where each point is one of the destinations
    /// to be visited
    pub fn geojson(&self) -> GeoJsonGeometry {
        GeoJsonGeometry::MultiPoint { 
            coordinates: self.destinations.iter().copied().map(GeoJsonPoint::from).collect::<Vec<_>>()
        }
    }
}
