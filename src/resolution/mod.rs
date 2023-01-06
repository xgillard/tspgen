//! This module provides the facilities to solve a tsp instance using branch and bound with mdd

use std::{fs::File, io::BufReader, time::Duration};

use clap::Args;
use ddo::{ParallelSolver, FixedWidth, TimeBudget, SimpleFrontier, MaxUB, Solver, Completion};

use self::model::{TspModel, TspRelax, TspRanking};

mod model;

/// This command lets you generate an html file to visualize a given instance
/// and an optional solution.
#[derive(Debug, Args)]
pub struct Solve {
    /// The path to the instance file
    #[clap(short, long)]
    pub instance: String,
    /// max number of nodes in a layeer
    #[clap(short, long, default_value="100")]
    pub width: usize,
    /// timeout
    #[clap(short, long, default_value="60")]
    pub timeout: u64,

    /// If present, the path where to write the output html
    #[clap(short, long)]
    pub output: Option<String>,
}

impl Solve {
    pub async fn execute(&self) {
        let instance = serde_json::from_reader(BufReader::new(File::open(&self.instance).unwrap())).unwrap();
        
        let problem = TspModel{instance};
        let relaxation = TspRelax;

        let width = FixedWidth(self.width);
        let cutoff = TimeBudget::new(Duration::from_secs(self.timeout));
        let ranking = TspRanking;
        let mut fringe = SimpleFrontier::new(MaxUB::new(&ranking));

        let mut solver = ParallelSolver::new(&problem, &relaxation, &ranking, &width, &cutoff, &mut fringe);

        let Completion{best_value, is_exact} = solver.maximize();

        let best_value = best_value.map(|v| v as f32 / -100_000_000.0).unwrap_or(0.0); // en kilometres
        println!("is exact {is_exact}");
        println!("best value {best_value}");

        let mut sol = String::new();
        solver.best_solution().unwrap()
            .iter().map(|d| d.value)
            .for_each(|v| sol.push_str(&format!("{v} ")));

        println!("solution: {sol}");
    }
}