//! This module implements the visualisation facilities that can be used to generate an
//! html file depicting the instance (and a possible solution of that instance).
use std::{io::{BufReader, Write}, fs::File};

use clap::Args;
use handlebars::no_escape;
use osrm_client::{Route, RouteRequestBuilder, Geometries, OverviewRequest, Client};
use rand_distr::num_traits::ToPrimitive;
use serde_json::json;

use crate::instance::Instance;

/// This command lets you generate an html file to visualize a given instance
/// and an optional solution.
#[derive(Debug, Args)]
pub struct Visualize {
    /// The path to the instance file
    #[clap(short, long)]
    pub instance: String,
    /// A possible solution (sequence of destination identifiers 0..n)
    #[clap(short, long)]
    pub solution: Option<String>,
    /// If present, the path where to write the output html
    #[clap(short, long)]
    pub output: Option<String>,
    /// URL of the osrm server to use (optional)
    #[clap(short, long)]
    pub url_osrm: Option<String>,
}
impl Visualize {
    /// Executes this command
    pub async fn execute(&self) {
        let instance = serde_json::from_reader(BufReader::new(File::open(&self.instance).unwrap())).unwrap();
        
        let html = if let Some(solution) = self.solution.as_ref() {
            let mut client = osrm_client::Client::default();
            if let Some(url) = self.url_osrm.as_ref() {
                client = client.base_url(url.clone());
            }
            let solution = solution.split_whitespace().into_iter().map(|tok| tok.parse::<usize>().unwrap()).collect::<Vec<_>>();
            let route = self.solution_route(&client, &instance, &solution).await;
            self.visualize_solution(&instance, &route).await
        } else {
            self.visualize(&instance).await
        };
        
        if let Some(output) = self.output.as_ref() {
            File::create(output).unwrap().write_all(html.as_bytes()).unwrap();
        } else {
            println!("{html}");
        }
    }

    /// Bare bones visualisation: only shows the locations on the map
    pub async fn visualize(&self, instance: &Instance) -> String {
        let template = include_str!("./visual_template.hbs");
        let destinations = serde_json::to_string_pretty(&instance.geojson()).unwrap();
        let handlebars = handlebars::Handlebars::new();
        handlebars.render_template(template, &json!({
            "destinations": destinations
        })).unwrap()
    }

    /// More elaborate visualisation: shows locations as well as a route to join all these cities
    pub async fn visualize_solution(&self, instance: &Instance, route: &Route) -> String {
        let template = include_str!("./visual_template.hbs");
        let total_distance = route.distance;
        let total_duration = route.duration;
        let destinations = serde_json::to_string(&instance.geojson()).unwrap();
        let route = serde_json::to_string(&route.geometry).unwrap();

        let hours = total_duration / 3600.0;
        let minutes = (hours - hours.floor()) * 60.0;
        let seconds = (minutes - minutes.floor()) * 60.0;

        let hours = hours.floor().to_u8().unwrap();
        let minutes = minutes.floor().to_u8().unwrap();
        let seconds = seconds.floor().to_u8().unwrap();


        let mut handlebars = handlebars::Handlebars::new();
        handlebars.register_escape_fn(no_escape);
        handlebars.render_template(template, &json!({
            "destinations": destinations,
            "route": route,
            "totalDistance": format!("{:.2}", total_distance / 1000.0),   // in kilometers
            "totalDuration": format!("{hours} hours {minutes} minutes {seconds} seconds"), // in hours
        })).unwrap()
    }

    /// Computes the actual route based on the locations ordering
    async fn solution_route(&self, client: &Client, instance: &Instance, solution: &[usize]) -> Route {
        let path = solution.iter().copied()
                .map(|i| instance.destinations[i])
                .collect();
        let response = RouteRequestBuilder::default()
            .coordinates(osrm_client::Coordinates::Multi(path))
            .geometries(Geometries::GeoJson)
            .overview(OverviewRequest::Full)
            .build()
            .unwrap()
            .send(client).await
            .unwrap();
        response.routes[0].clone()
    }
}