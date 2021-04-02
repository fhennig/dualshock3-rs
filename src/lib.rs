mod controller;
pub use controller::{Controller, ControllerValues};
use hidapi::HidApi;
use log::{debug, info};
use std::{thread, time};
use stoppable_thread::{spawn, StoppableHandle};

/// A trait that takes controller values and updates a state, sends
/// them over a network or does whatever with them.
pub trait ControllerHandler {
    fn controller_update(&mut self, controller: &Controller);
}

/// Takes a sink where controller updates will be put.  Controller
/// values are read from the HIDAPI.  This function takes care of
/// waiting for a controller to connect and automatically reconnects
/// if the controller is disconnected.
#[allow(unused_must_use)]
pub fn read_controller(
    mut controller_handler: Box<dyn ControllerHandler + Send + Sync>,
) -> StoppableHandle<()> {
    spawn(move |stopped| {
        // TODO make a big retry loop, where we retry to open the device.
        let dur = time::Duration::from_millis(1000);
        while !stopped.get() {
            info!("Trying to connect to a controller ...");

            let (vid, pid) = (1356, 616);
            let mut api = HidApi::new().unwrap();
            let mut found = false;
            while !found {
                api.refresh_devices();
                debug!("Devices refreshed!");
                for device in api.device_list() {
                    debug!("{:?}", device.path());
                    if device.vendor_id() == vid && device.product_id() == pid {
                        info!("Found the device!");
                        found = true;
                    }
                }
                if !found {
                    debug!("Device not found, retrying ...");
                    thread::sleep(dur);
                }
            }
            // at this point the device was found, open it:
            info!("Opening...");
            let device = api.open(vid, pid).unwrap();

            let mut buf = [0u8; 20];
            let mut controller = Controller::new(ControllerValues::new(buf));
            // The loop
            while !stopped.get() {
                // Read data from device
                match device.read_timeout(&mut buf[..], -1) {
                    Ok(_) => {
                        debug!("Read: {:?}", buf);
                        let vals = ControllerValues::new(buf);
                        controller.update(vals);
                        controller_handler.controller_update(&controller);
                    }
                    Err(_e) => {
                        info!("Error reading controller values.");
                        break;
                    }
                }
            }
        }
    })
}
