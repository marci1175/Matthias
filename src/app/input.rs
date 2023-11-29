use device_query::{DeviceState, Keycode};

#[derive(Clone)]
pub struct Input {
    pub device_input: DeviceState,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            device_input: DeviceState::new(),
        }
    }
}

pub fn keymap(input: Input) -> Vec<Keycode> {
    let keys: Vec<Keycode> = input.device_input.query_keymap();

    keys
}
