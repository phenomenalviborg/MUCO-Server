use std::{ffi::CString, ptr::null};
use mdns_sd::{Receiver, ServiceDaemon, ServiceEvent};

#[no_mangle]
pub extern "C" fn hello() -> *mut i8 {
    let my_string = CString::new("hello from discoverer dll").unwrap();
    my_string.into_raw() as *mut i8 
}

#[no_mangle]
pub extern "C" fn new_discoverer() -> *mut Discoverer {
    Box::into_raw(Box::new(Discoverer::new()))
}

#[no_mangle]
pub extern "C" fn destroy_discoverer(ptr: *mut Discoverer) {
    unsafe {
        let _my_box = Box::from_raw(ptr);
    }
}

#[no_mangle]
pub extern "C" fn try_discover(ptr: *mut Discoverer) -> *mut i8 {
    let discoverer = unsafe { &mut *ptr };
    if let Some(ip) = discoverer.try_recv() {
        let my_string = CString::new(ip).unwrap();
        my_string.into_raw() as *mut i8
    }
    else {
        null::<i8>() as *mut i8
    }
}

pub struct Discoverer {
    pub mdns: ServiceDaemon,
    pub receiver: Receiver<ServiceEvent>,
}

impl Discoverer {
    pub fn new() -> Discoverer {
        let mdns = ServiceDaemon::new().expect("Failed to create daemon");

        let service_type = "_muco-server._tcp.local.";
        let receiver = mdns.browse(service_type).expect("Failed to browse");

        Discoverer {
            mdns,
            receiver,
        }
    }

    pub fn try_recv(&self) -> Option<String> {
        let result = self.receiver.try_recv();
        match result {
            Ok(event) => {
                match event {
                    ServiceEvent::ServiceResolved(info) => {
                        let addresses = info.get_addresses();
                        let addr = addresses.iter().next().unwrap();
                        let port = info.get_port();
                        let s = format!("{addr}:{port}");
                        return Some(s);
                    }
                    _ => None
                }
            }
            Err(_) => None
        }
    }
}

impl Drop for Discoverer {
    fn drop(&mut self) {
        self.mdns.shutdown().unwrap();
    }
}
