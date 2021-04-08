mod controller;
pub use controller::{Controller, ControllerValues, Coordinate, Button, Axis};
use hidapi::HidApi;
use log::{debug, info, trace};
use core::f64;
use std::{thread, time};
use stoppable_thread::{spawn, StoppableHandle};

pub enum LR {
    Left,
    Right,
}

pub enum ButtonEvt {
    Down,
    Up,
}


// #[derive(Debug, Copy, Clone)]
pub enum HidEvent {
    Button(Button, ButtonEvt),
    Stick(LR, Coordinate),
    Trigger(LR, f64),
}

/// A trait that takes controller values and updates a state, sends
/// them over a network or does whatever with them.
pub trait ControllerHandler {
    /// Defines the minimal length for stick events to be sent.
    fn min_len_stick(&self) -> f64 { 0. }

    /// Defines the minimal length for trigger events to be sent.
    fn min_len_trigger(&self) -> f64 { 0. }

    //TODO: should this take a Vec<HidEvent> instead, to allow for checking for multiple simultaneous button presses?
    /// Takes a HidEvent to process. This can be a void implementation if you override
    /// controller_update instead (e.g. you want direct control over the raw controller state).
    fn on_event(&mut self, e: HidEvent);

    /// Default implementation that calls on_event to process changed/active controller states.
    fn controller_update(&mut self, controller: &Controller) {
        let l_stk = self.min_len_stick();
        let l_tgr = self.min_len_trigger();

        // Active sticks get processed first
        if controller.left_pos_changed() {
            if l_stk == 0. || controller.left_pos().length() > l_stk {
                self.on_event(
                    HidEvent::Stick(LR::Left, controller.left_pos())
                );
            } else if controller.prev_left_pos().length() > l_stk {
                self.on_event(
                    HidEvent::Stick(LR::Left, Coordinate::zero())
                );
            }
        }
        if controller.right_pos_changed() {
            if l_stk == 0. || controller.right_pos().length() > l_stk {
                self.on_event(
                    HidEvent::Stick(LR::Right, controller.right_pos())
                );
            } else if controller.prev_right_pos().length() > l_stk {
                self.on_event(
                    HidEvent::Stick(LR::Right, Coordinate::zero())
            );
            }
        }

        // Next come the simple buttons
        let (pressed, released) = controller.changed_buttons();
        for btn in pressed.iter() {  //TODO bad naming
            self.on_event(
                HidEvent::Button(*btn, ButtonEvt::Down)
            );
        }
        for btn in released.iter() {
            self.on_event(
                HidEvent::Button(*btn, ButtonEvt::Up)
            );
        }

        // Finally, we process the active trigger axes
        // NOTE: there is already a simple button for each trigger as well...
        // ... and I'm not quite sure if our threshold agrees with the controller-internal one.
        if controller.left_trigger_changed() {
            if l_tgr == 0. || controller.left_trigger() > l_tgr {
                self.on_event(
                    HidEvent::Trigger(LR::Left, controller.left_trigger())
                );
            } else if controller.prev_left_trigger() > l_tgr {
                self.on_event(
                    HidEvent::Trigger(LR::Left, 0.)
                );
            }
        }
        if controller.right_trigger_changed() {
            if l_tgr == 0. || controller.right_trigger() > l_tgr {
                self.on_event(
                    HidEvent::Trigger(LR::Right, controller.right_trigger())
                );
            } else if controller.prev_right_trigger() > l_tgr {
                self.on_event(
                    HidEvent::Trigger(LR::Right, 0.)
                );
            }
        }
    }
}

/// Takes a sink where controller updates will be put.  Controller
/// values are read from the HIDAPI.  This function takes care of
/// waiting for a controller to connect and automatically reconnects
/// if the controller is disconnected.
#[allow(unused_must_use)]
pub fn read_controller(
    mut controller_handler: Box<dyn ControllerHandler + Send + Sync>
) -> StoppableHandle<()> {
    spawn(move |stopped| {
        // TODO make a big retry loop, where we retry to open the device.
        let dur = time::Duration::from_millis(1000);
        while !stopped.get() {
            info!("Trying to connect to a controller ...");

            let (vid, pid) = (1356, 616);
            let mut api = HidApi::new().unwrap("Failed to get HID API.");
            trace!("Unwrapped HID API.");
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
