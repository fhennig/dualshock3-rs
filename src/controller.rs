use log::debug;

#[derive(Copy, Clone)]
pub struct Coordinate(pub f64, pub f64);

// TODO this should be an optional; the angle is undefined for 0, 0 (WARNING!)
impl Coordinate {
    /// Returns radians from [-1, 1)
    /// top is 0, left is -0.5, right is 0.5, bottom is -1
    pub fn angle(&self) -> Option<f64> {
        if self.length() < 0.01 {
            return None;
        }
        let a = Self(0.0, 1.0);
        let b = self;
        let mut angle = (a.0 * b.0 + a.1 * b.1)
            / ((a.0.powi(2) + a.1.powi(2)).powf(0.5) * (b.0.powi(2) + b.1.powi(2)).powf(0.5));
        if b.0 > 0.0 && b.1 > 0.0 {
            angle = (1.0 - angle) * 0.5;
        } else if b.0 > 0.0 && b.1 <= 0.0 {
            angle = 0.5 + (angle * 0.5) * -1.0;
        } else if b.0 <= 0.0 && b.1 <= 0.0 {
            angle = -0.5 + angle * 0.5;
        } else if b.0 <= 0.0 && b.1 > 0.0 {
            angle = -(1.0 - angle) * 0.5;
        }
        Some(angle)
    }

    pub fn length(&self) -> f64 {
        (self.0.powi(2) + self.1.powi(2)).sqrt()
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Button {
    PS,
    Start,
    Select,
    Up,
    Down,
    Left,
    Right,
    L1,
    L2,
    L3,
    R1,
    R2,
    R3,
    Triangle,
    Circle,
    Cross,
    Square,
}

impl Button {
    fn val(&self) -> (usize, u8) {
        match *self {
            Button::PS => (4, 0),
            Button::Start => (2, 3),
            Button::Select => (2, 0),
            Button::Up => (2, 4),
            Button::Down => (2, 6),
            Button::Left => (2, 7),
            Button::Right => (2, 5),
            Button::L1 => (3, 2),
            Button::L2 => (3, 0),
            Button::L3 => (2, 1),
            Button::R1 => (3, 3),
            Button::R2 => (3, 1),
            Button::R3 => (2, 2),
            Button::Triangle => (3, 4),
            Button::Circle => (3, 5),
            Button::Cross => (3, 6),
            Button::Square => (3, 7),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Axis {
    LX,
    LY,
    L3,
    RX,
    RY,
    R3,
}

impl Axis {
    fn val(&self) -> usize {
        match *self {
            Axis::LX => 6,
            Axis::LY => 7,
            Axis::L3 => 18,
            Axis::RX => 8,
            Axis::RY => 9,
            Axis::R3 => 19,
        }
    }
}

/// gets the bit at position `n`.
/// Bits are numbered from 0 (least significant) to 31 (most significant).
fn get_bit_at(input: u8, n: u8) -> bool {
    if n < 32 {
        input & (1 << n) != 0
    } else {
        false
    }
}

/// This type represents the raw byte representation of the controller
/// state, taken from the HIDAPI.
pub type RawCVals = [u8; 20];

/// A basic abstraction over the byte representation of the controller
/// state.  Allows accessing the state with the Button and Axis enums.
#[derive(Copy, Clone)]
pub struct ControllerValues {
    pub buf: RawCVals,
}

impl ControllerValues {
    pub fn new_empty() -> ControllerValues {
        ControllerValues::new([0; 20])
    }

    pub fn new(buf: RawCVals) -> ControllerValues {
        ControllerValues { buf: buf }
    }

    pub fn is_pressed(&self, btn: Button) -> bool {
        let v = btn.val();
        get_bit_at(self.buf[v.0], v.1)
    }

    pub fn get_axis_val(&self, ax: Axis) -> u8 {
        let v = ax.val();
        self.buf[v]
    }
}

/// The controller abstracts away over controller state at specific
/// points and allows to detect when a button is pressed or released.
/// It also combines the X and Y axis for each stick into a coordinate
/// and turns the triggers into values in [0, 1].
pub struct Controller {
    prev_vals: ControllerValues,
    curr_vals: ControllerValues,
}

impl Controller {
    pub fn new(buf: ControllerValues) -> Controller {
        Controller {
            prev_vals: buf,
            curr_vals: buf,
        }
    }

    pub fn update(&mut self, new_vals: ControllerValues) {
        self.prev_vals = self.curr_vals;
        self.curr_vals = new_vals;
    }

    pub fn left_pos(&self) -> Coordinate {
        let l_x = self.curr_vals.get_axis_val(Axis::LX);
        let l_y = self.curr_vals.get_axis_val(Axis::LY);
        let l_x = ((l_x as f64) / 255.0 - 0.5) * 2.0;
        let l_y = ((l_y as f64 / 255.0 - 0.5) * -1.0) * 2.0;
        Coordinate(l_x, l_y)
    }

    pub fn right_pos(&self) -> Coordinate {
        let r_x = self.curr_vals.get_axis_val(Axis::RX);
        let r_y = self.curr_vals.get_axis_val(Axis::RY);
        let r_x = ((r_x as f64) / 255.0 - 0.5) * 2.0;
        let r_y = ((r_y as f64 / 255.0 - 0.5) * -1.0) * 2.0;
        Coordinate(r_x, r_y)
    }

    /// Returns a value in [0, 1]
    pub fn left_trigger(&self) -> f64 {
        let t = self.curr_vals.get_axis_val(Axis::L3);
        (t as f64) / 255.0
    }

    /// Returns a value in [0, 1]
    pub fn right_trigger(&self) -> f64 {
        let t = self.curr_vals.get_axis_val(Axis::R3);
        (t as f64) / 255.0
    }

    pub fn was_pressed(&self, btn: Button) -> bool {
        let prev = self.prev_vals.is_pressed(btn);
        let curr = self.curr_vals.is_pressed(btn);
        return !prev && curr;
    }

    pub fn is_pressed(&self, btn: Button) -> bool {
        self.curr_vals.is_pressed(btn)
    }

    #[allow(dead_code)]
    pub fn was_released(&self, btn: Button) -> bool {
        let prev = self.prev_vals.is_pressed(btn);
        let curr = self.curr_vals.is_pressed(btn);
        return prev && !curr;
    }

    /// Indicates whether there is currently any kind of user input.
    /// A pressed button, or a moved joystick.
    pub fn has_any_input(&self) -> bool {
        if self.left_pos().length() > 0.1 || self.right_pos().length() > 0.1 {
            return true;
        }
        if self.left_trigger() > 0.1 || self.right_trigger() > 0.1 {
            return true;
        }
        for btn in [
            Button::PS,
            Button::Start,
            Button::Select,
            Button::Up,
            Button::Down,
            Button::Left,
            Button::Right,
            Button::L1,
            Button::L2,
            Button::L3,
            Button::R1,
            Button::R2,
            Button::R3,
            Button::Triangle,
            Button::Circle,
            Button::Cross,
            Button::Square,
        ]
        .iter()
        {
            if self.is_pressed(*btn) {
                return true;
            }
        }
        return false;
    }

    #[allow(dead_code)]
    fn debug_print(&self) {
        for btn in [
            Button::PS,
            Button::Start,
            Button::Select,
            Button::Up,
            Button::Down,
            Button::Left,
            Button::Right,
            Button::L1,
            Button::L2,
            Button::L3,
            Button::R1,
            Button::R2,
            Button::R3,
            Button::Triangle,
            Button::Circle,
            Button::Cross,
            Button::Square,
        ]
        .iter()
        {
            if self.was_pressed(*btn) {
                debug!("{:?} pressed.", btn);
            }
        }
    }
}
