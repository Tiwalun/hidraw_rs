use anyhow::{anyhow, Context, Result};

use hidraw_rs::{Device, ReportItem};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use structopt::StructOpt;

use parse_int::parse;
use std::num::ParseIntError;

fn parse_arg(arg: &str) -> std::result::Result<u32, ParseIntError> {
    parse::<u32>(arg)
}

#[derive(Debug, StructOpt)]
struct App {
    /// USB Product ID
    #[structopt(parse(try_from_str=parse_arg))]
    product_id: u32,
    /// USB Vendor ID
    #[structopt(parse(try_from_str=parse_arg))]
    vendor_id: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    let app = App::from_args();

    let hid_devices = hidraw_rs::list_devices()?;

    let target_device = hid_devices
        .iter()
        .find(|device| {
            (device.product_id() == app.product_id) && (device.vendor_id() == app.vendor_id)
        })
        .ok_or_else(|| {
            anyhow!(format!(
                "Unable to find HID device with PID={:010x}, VID={:010x}",
                app.product_id, app.vendor_id
            ))
        })?;

    let target_dev_node = target_device.dev_node().to_str().ok_or_else(|| {
        anyhow!(format!(
            "Failed to convert path '{}' to String",
            target_device.dev_node().display()
        ))
    })?;

    let mut device = Device::from_path(target_dev_node)
        .with_context(|| format!("Failed to open device '{}'", target_dev_node))?;

    let report_descriptor = device.feature_report()?;

    println!("Report Descriptor:");

    let mut index = 0usize;

    let raw_report = report_descriptor.data();

    while index < raw_report.len() {
        let (descriptor, bytes_read) = ReportItem::parse(&raw_report[index..])?;
        println!("\t{:?}", descriptor);
        index += bytes_read;
    }
    println!();

    let mut command = [0u8; 64];

    command[2] = 0xFE;

    device.write(&command).await?;

    let mut response = [0u8; 64];

    let bytes_read = device.read(&mut response).await?;

    println!("Read {} bytes: {:?}", bytes_read, &response[..bytes_read]);

    Ok(())
}
