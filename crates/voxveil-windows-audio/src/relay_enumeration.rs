use wasapi::{DeviceEnumerator, Direction};

use crate::device::EndpointDescriptor;

pub(super) fn enumerate_render_blocking() -> Result<Vec<EndpointDescriptor>, String> {
    std::thread::spawn(move || {
        wasapi::initialize_mta()
            .ok()
            .map_err(|error| error.to_string())?;
        let result = enumerate_render_inner();
        wasapi::deinitialize();
        result
    })
    .join()
    .map_err(|_| "Windows endpoint enumeration panicked".to_string())?
}

fn enumerate_render_inner() -> Result<Vec<EndpointDescriptor>, String> {
    let enumerator = DeviceEnumerator::new().map_err(|error| error.to_string())?;
    let default_id = enumerator
        .get_default_device(&Direction::Render)
        .and_then(|device| device.get_id())
        .unwrap_or_default();
    let collection = enumerator
        .get_device_collection(&Direction::Render)
        .map_err(|error| error.to_string())?;
    let mut endpoints = Vec::new();
    for device in &collection {
        let device = device.map_err(|error| error.to_string())?;
        let id = device.get_id().map_err(|error| error.to_string())?;
        let name = device
            .get_friendlyname()
            .map_err(|error| error.to_string())?;
        endpoints.push(EndpointDescriptor {
            is_default: id == default_id,
            interface_name: device.get_interface_friendlyname().ok(),
            description: device.get_description().ok(),
            id,
            name,
        });
    }
    Ok(endpoints)
}
