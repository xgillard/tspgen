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
}
impl Visualize {
    pub async fn execute(&self) {
        let instance = serde_json::from_reader(BufReader::new(File::open(&self.instance).unwrap())).unwrap();
        
        let html = if let Some(solution) = self.solution.as_ref() {
            let client = osrm_client::Client::default();
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
    
    pub async fn visualize(&self, instance: &Instance) -> String {
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

                    const destinationPin = markerIcon('destination-icon', '#3366ff');

                    var map = L.map('map');
                    L.tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png', {
                        maxZoom: 19,
                        attribution: '&copy; <a href="http://www.openstreetmap.org/copyright">OpenStreetMap</a>'
                    }).addTo(map);

                    var destinations = L.geoJSON({{destinations}}, {
                    pointToLayer: function(feature, latlng) {
                        return L.marker(latlng, {icon: destinationPin});
                    },
                    });
                    destinations.addTo(map);
                    map.fitBounds(destinations.getBounds());
                </script>
            </body>
        </html>
        "#;

        let destinations = serde_json::to_string(&instance.geojson()).unwrap();
        let mut handlebars = handlebars::Handlebars::new();
        handlebars.register_escape_fn(no_escape);
        handlebars.render_template(template, &json!({"destinations": destinations})).unwrap()
    }


   pub async fn visualize_solution(&self, instance: &Instance, route: &Route) -> String {
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
   
                   const destinationPin = markerIcon('destination-icon', '#3366ff');
                   var map = L.map('map');
                   L.tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png', {
                       maxZoom: 19,
                       attribution: '&copy; <a href="http://www.openstreetmap.org/copyright">OpenStreetMap</a>'
                   }).addTo(map);
   
                   var destinations = L.geoJSON({{destinations}}, {
                   pointToLayer: function(feature, latlng) {
                       return L.marker(latlng, {icon: destinationPin});
                   },
                   });
                   destinations.addTo(map);
   
                   var route = L.geoJSON({{route}}, {"color": "red"});
                   route.on("click", function(e) {
                        L.popup()
                            .setLatLng(e.latlng)
                            .setContent('<div style="font-weight: bold; font-size: 15;">{{totalDistance}} km</div>{{totalDuration}}')
                            .openOn(map);
                   })
                   route.addTo(map);
   
                   map.fitBounds(destinations.getBounds());
               </script>
           </body>
       </html>
       "#;
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
}