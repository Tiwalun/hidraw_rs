// Unix specific code
use anyhow::Result;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};
use udev::Enumerator;

pub fn list_devices() -> Result<Vec<DeviceInfo>> {
    let mut device_info_list = Vec::new();

    let mut enumerator = Enumerator::new()?;

    enumerator.match_subsystem("hidraw")?;

    let devices = enumerator.scan_devices()?;

    for device in devices {
        //println!("\tSyspath: {}", device.syspath().display());

        let hid_subsystem = Path::new("hid");

        if let Some(parent_device) = device.parent_with_subsystem(hid_subsystem)? {
            //println!("Parent device: {}", parent_device.syspath().display());
            log::debug!(
                "Device {:?}",
                parent_device.property_value("HID_NAME").unwrap()
            );

            if let Some(uevent) = parent_device.attribute_value("uevent") {
                let parsed_event = HidUevent::from_raw(uevent);

                if let Some(devnode) = device.devnode() {
                    log::debug!("\tPID: {:06x}", parsed_event.product);
                    log::debug!("\tVID: {:06x}", parsed_event.vendor);
                    log::debug!("\tDevnode: {}", devnode.display());
                    device_info_list.push(DeviceInfo::new(parsed_event, devnode));
                }
            }
        }

        /*
        println!("\tProperties:");
        for property in device.properties() {
            println!("\t\t{:?} = {:?}", property.name(), property.value());
        }

        println!("\tAttributes:");
        for attribute in device.attributes() {
            println!("\t\t{:?} = {:?}", attribute.name(), attribute.value());
        }

        */
    }

    Ok(device_info_list)
}

pub struct DeviceInfo {
    product_id: u32,
    vendor_id: u32,

    serial_number: Option<String>,

    dev_node: PathBuf,
}

impl DeviceInfo {
    fn new(event: HidUevent, dev_node: &Path) -> Self {
        // Replace empty string with None
        let serial_number = if !event.serial_number.is_empty() {
            Some(event.serial_number.to_owned())
        } else {
            None
        };

        DeviceInfo {
            product_id: event.product,
            vendor_id: event.vendor,
            serial_number,
            dev_node: dev_node.to_owned(),
        }
    }

    // TODO: Make private
    pub fn dev_node(&self) -> &Path {
        &self.dev_node
    }

    pub fn product_id(&self) -> u32 {
        self.product_id
    }

    pub fn vendor_id(&self) -> u32 {
        self.vendor_id
    }

    pub fn serial_number(&self) -> Option<&str> {
        self.serial_number.as_deref()
    }
}

#[derive(Debug)]
struct HidUevent<'r> {
    bus: u16,
    vendor: u32,
    product: u32,

    name: &'r str,
    serial_number: &'r str,
}

impl HidUevent<'_> {
    fn from_raw(raw: &OsStr) -> HidUevent<'_> {
        let parsed = raw.to_str().unwrap();

        let mut bus = None;
        let mut vendor = None;
        let mut product = None;

        let mut name = None;
        let mut serial_number = None;

        for line in parsed.lines() {
            let mut parts = line.splitn(2, '=');

            let key = parts.next().unwrap();
            let value = parts.next().unwrap();

            match key {
                "HID_NAME" => name = Some(value),
                "HID_UNIQ" => serial_number = Some(value),
                "HID_ID" => {
                    let mut parts = value.split(':');

                    bus = Some(u16::from_str_radix(parts.next().unwrap(), 16).unwrap());
                    vendor = Some(u32::from_str_radix(parts.next().unwrap(), 16).unwrap());
                    product = Some(u32::from_str_radix(parts.next().unwrap(), 16).unwrap());
                }
                _ => {} // Ignore other keys
            }
        }

        HidUevent {
            bus: bus.unwrap(),
            vendor: vendor.unwrap(),
            product: product.unwrap(),
            name: name.unwrap(),
            serial_number: serial_number.unwrap(),
        }
    }
}
