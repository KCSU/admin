pub mod lookup {
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/kcsu.lookup.v1.rs"));
    }
}

pub mod alerts {
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/kcsu.alerts.v1.rs"));
    }
}
