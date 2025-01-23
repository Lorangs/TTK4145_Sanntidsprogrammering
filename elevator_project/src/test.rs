// Denne siden brukes til test av diverse funksjoner i programmet

use driver_rust::elevio;
use driver_rust::elevio::elev as e;

use std::path::Path;   
use std::thread::*;
use std::time::*;

use std::io::ErrorKind;

use crossbeam_channel as cbc;

mod config;

