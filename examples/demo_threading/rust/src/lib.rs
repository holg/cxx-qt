// SPDX-FileCopyrightText: 2021, 2022 Klarälvdalens Datakonsult AB, a KDAB Group company <info@kdab.com>
// SPDX-FileContributor: Andrew Hayzen <andrew.hayzen@kdab.com>
// SPDX-FileContributor: Leon Matthes <leon.matthes@kdab.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

mod constants;
mod network;
mod workers;

// This mod defines our QObject called EnergyUsage
#[cxx_qt::bridge(cxx_file_stem = "energy_usage", namespace = "cxx_qt::energy_usage")]
mod ffi {
    use crate::{constants::SENSOR_MAXIMUM_COUNT, workers::SensorHashMap};
    use std::{
        sync::{Arc, Mutex},
        thread::JoinHandle,
    };

    #[namespace = ""]
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    #[cxx_qt::qobject(qml_uri = "com.kdab.energy", qml_version = "1.0")]
    pub struct EnergyUsage {
        /// The average power usage of the connected sensors
        #[qproperty]
        average_use: f64,
        /// The count of connected sensors
        #[qproperty]
        sensors: u32,
        /// The total power usage of the connected sensors
        #[qproperty]
        total_use: f64,

        /// The join handles of the running threads
        pub(crate) join_handles: Option<[JoinHandle<()>; 4]>,
        /// A HashMap of the currently connected sensors
        ///
        /// This uses an Arc inside the Mutex as well as outside so that the HashMap is only
        /// cloned when required. By using Arc::make_mut on the inner HashMap data is only cloned
        /// when mutating if another thread is still holding onto reference to the data.
        /// <https://doc.rust-lang.org/std/sync/struct.Arc.html#method.make_mut>
        pub(crate) sensors_map: Arc<Mutex<Arc<SensorHashMap>>>,
    }

    impl Default for EnergyUsage {
        fn default() -> Self {
            Self {
                average_use: 0.0,
                sensors: 0,
                total_use: 0.0,

                join_handles: None,
                sensors_map: Arc::new(Mutex::new(Arc::new(SensorHashMap::with_capacity(
                    SENSOR_MAXIMUM_COUNT,
                )))),
            }
        }
    }

    // Enabling threading on the qobject
    impl cxx_qt::Threading for qobject::EnergyUsage {}

    unsafe extern "RustQt" {
        /// A new sensor has been detected
        #[qsignal]
        fn sensor_added(self: Pin<&mut qobject::EnergyUsage>, uuid: QString);
        /// A value on an existing sensor has changed
        #[qsignal]
        fn sensor_changed(self: Pin<&mut qobject::EnergyUsage>, uuid: QString);
        /// An existing sensor has been removed
        #[qsignal]
        fn sensor_removed(self: Pin<&mut qobject::EnergyUsage>, uuid: QString);
    }

    unsafe extern "RustQt" {
        /// A Q_INVOKABLE that returns the current power usage for a given uuid
        #[qinvokable]
        fn sensor_power(self: Pin<&mut qobject::EnergyUsage>, uuid: &QString) -> f64;

        /// A Q_INVOKABLE which starts the TCP server
        #[qinvokable]
        fn start_server(self: Pin<&mut qobject::EnergyUsage>);
    }
}

use crate::{
    constants::CHANNEL_NETWORK_COUNT,
    network::NetworkServer,
    workers::{AccumulatorWorker, SensorsWorker, TimeoutWorker},
};

use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use futures::executor::block_on;
use std::sync::{atomic::AtomicBool, mpsc::sync_channel, Arc};
use uuid::Uuid;

// TODO: this will change to qobject::EnergyUsage once
// https://github.com/KDAB/cxx-qt/issues/559 is done
impl ffi::EnergyUsageQt {
    /// A Q_INVOKABLE that returns the current power usage for a given uuid
    fn sensor_power(self: Pin<&mut Self>, uuid: &QString) -> f64 {
        let sensors = SensorsWorker::read_sensors(&self.rust_mut().sensors_map);

        if let Ok(uuid) = Uuid::parse_str(&uuid.to_string()) {
            sensors.get(&uuid).map(|v| v.power).unwrap_or_default()
        } else {
            0.0
        }
    }

    /// A Q_INVOKABLE which starts the TCP server
    fn start_server(mut self: Pin<&mut Self>) {
        if self.rust().join_handles.is_some() {
            println!("Already running a server!");
            return;
        }

        // Create a channel which is used for passing valid network requests
        // from the NetworkServer to the SensorsWorker
        let (network_tx, network_rx) = sync_channel(CHANNEL_NETWORK_COUNT);
        // Create an AtomicBool which the SensorsWorker uses to tell
        // the AccumulatorWorker that the sensors have changed
        let sensors_changed = Arc::new(AtomicBool::new(false));

        // Make relevent clones so that we can pass them to the threads
        let accumulator_sensors = Arc::clone(&self.as_mut().rust_mut().sensors_map);
        let accumulator_sensors_changed = Arc::clone(&sensors_changed);
        let accumulator_qt_thread = self.qt_thread();
        let sensors = Arc::clone(&self.as_mut().rust_mut().sensors_map);
        let sensors_qt_thread = self.qt_thread();
        let timeout_sensors = Arc::clone(&self.as_mut().rust_mut().sensors_map);
        let timeout_network_tx = network_tx.clone();

        // Start our threads
        self.rust_mut().join_handles = Some([
            // Create a TimeoutWorker
            // If a sensor is not seen for N seconds then a disconnect is requested
            std::thread::spawn(move || {
                block_on(TimeoutWorker::run(timeout_network_tx, timeout_sensors))
            }),
            // Create a AccumulatorWorker
            // When sensor values change this creates accumulations of the data
            // (such as total, average etc) and then requests an update to Qt
            std::thread::spawn(move || {
                block_on(AccumulatorWorker::run(
                    accumulator_sensors,
                    accumulator_sensors_changed,
                    accumulator_qt_thread,
                ))
            }),
            // Create a SensorsWorker
            // Reads network requests from the NetworkServer, collates the commands
            // by mutating the sensors hashmap, and requests signal changes to Qt
            std::thread::spawn(move || {
                block_on(SensorsWorker::run(
                    network_rx,
                    sensors,
                    sensors_changed,
                    sensors_qt_thread,
                ))
            }),
            // Create a NetworkServer
            // Starts a TCP server which listens for requests and sends valid
            // network requests to the SensorsWorker
            std::thread::spawn(move || {
                block_on(NetworkServer::listen("127.0.0.1:8080", network_tx))
            }),
        ]);
    }
}
