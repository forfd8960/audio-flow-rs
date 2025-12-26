// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use audio_flow_core::run;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run()?;
    Ok(())
}
