use super::AppState;
use std::sync::Mutex;

// pub mod group_controller;
// pub mod index_controller;
pub mod soal_controller;

// pub use group_controller::init as init_group_controller;
// pub use index_controller::init as init_index_controller;
pub use soal_controller::init as init_soal_controller;

fn log_request(route: &'static str, connections: &Mutex<u32>) {
    let mut con = connections.lock().unwrap();
    *con += 1;
    println!("{}\n\tconnections: {}", route, con);
}
