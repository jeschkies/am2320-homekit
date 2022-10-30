use am2320::Am2320;
use hap::{
    accessory::{
        temperature_sensor::TemperatureSensorAccessory, AccessoryCategory, AccessoryInformation,
    },
    characteristic::{
        current_temperature::CurrentTemperatureCharacteristic, CharacteristicCallbacks,
    },
    server::{IpServer, Server},
    service::temperature_sensor::TemperatureSensorService,
    storage::{FileStorage, Storage},
    Config, MacAddress, Pin, Result,
};
use mac_address;
use rppal::{hal::Delay, i2c::I2c};
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    // HomeKit
    let mut temperature_sensor = TemperatureSensorAccessory::new(
        1,
        AccessoryInformation {
            name: "Pi Temperature Sensor".into(),
            ..Default::default()
        },
    )?;
    let mut temperature_service = TemperatureSensorService::new(1, 1);
    let mut current_temperature = CurrentTemperatureCharacteristic::new(1, 1);
    current_temperature.on_read(Some(|| {
        // Sensor
        let device = I2c::new().expect("could not initialize I2c on your RPi");
        let delay = Delay::new();

        let mut am2320 = Am2320::new(device, delay);
        let measurement = am2320.read().unwrap(); // TODO: use ? and thiserror.
        Ok(Some(measurement.temperature))
    }));

    temperature_service.current_temperature = current_temperature;
    temperature_sensor.temperature_sensor = temperature_service;

    let mut storage = FileStorage::current_dir().await?;

    let config = match storage.load_config().await {
        Ok(mut config) => {
            config.redetermine_local_ip();
            storage.save_config(&config).await?;
            config
        }
        Err(_) => {
            let address = mac_address::get_mac_address()
                .unwrap()
                .map(|a| MacAddress::new(a.bytes()))
                .unwrap_or(MacAddress::new([10, 20, 30, 40, 50, 60]));
            let config = Config {
                pin: Pin::new([1, 1, 1, 2, 2, 3, 3, 3])?,
                name: "Pi Temperature Sensor".into(),
                device_id: address,
                category: AccessoryCategory::Sensor,
                ..Default::default()
            };
            storage.save_config(&config).await?;
            config
        }
    };

    let server = IpServer::new(config, storage).await?;
    server.add_accessory(temperature_sensor).await?;

    let handle = server.run_handle();

    std::env::set_var("RUST_LOG", "hap=debug");
    env_logger::init();

    handle.await
}
