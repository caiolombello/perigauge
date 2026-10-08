//! D-Bus connections with an explicit method timeout, so an unresponsive
//! service cannot stall a collection or the daemon loop.

use std::time::Duration;

use zbus::blocking::Connection;
use zbus::blocking::connection::Builder;

pub const METHOD_TIMEOUT: Duration = Duration::from_secs(5);

pub fn system() -> zbus::Result<Connection> {
    Builder::system()?.method_timeout(METHOD_TIMEOUT).build()
}

pub fn session() -> zbus::Result<Connection> {
    Builder::session()?.method_timeout(METHOD_TIMEOUT).build()
}
