//! Minimal IOKit bindings: read registry entries straight from the kernel
//! instead of parsing `ioreg` output.

use core_foundation::array::CFArray;
use core_foundation::base::{CFType, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::data::CFData;
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use core_foundation_sys::base::{CFAllocatorRef, CFTypeRef, kCFAllocatorDefault};
use core_foundation_sys::dictionary::CFMutableDictionaryRef;
use core_foundation_sys::number::CFNumberIsFloatType;
use core_foundation_sys::string::CFStringRef;
use serde_json::{Map, Value};
use core_foundation::runloop::{CFRunLoop, CFRunLoopSource, kCFRunLoopDefaultMode};
use core_foundation_sys::runloop::CFRunLoopSourceRef;
use std::ffi::{CString, c_char, c_void};
use std::sync::mpsc::Sender;

type IoObject = u32;
type KernReturn = i32;
const MAIN_PORT_DEFAULT: u32 = 0;

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOServiceMatching(name: *const c_char) -> CFMutableDictionaryRef;
    fn IOServiceGetMatchingService(main_port: u32, matching: CFMutableDictionaryRef) -> IoObject;
    fn IORegistryEntryCreateCFProperties(
        entry: IoObject,
        properties: *mut CFMutableDictionaryRef,
        allocator: CFAllocatorRef,
        options: u32,
    ) -> KernReturn;
    fn IORegistryEntryCreateCFProperty(
        entry: IoObject,
        key: CFStringRef,
        allocator: CFAllocatorRef,
        options: u32,
    ) -> CFTypeRef;
    fn IOObjectRelease(object: IoObject) -> KernReturn;
    fn IOPSNotificationCreateRunLoopSource(
        callback: extern "C" fn(*mut c_void),
        context: *mut c_void,
    ) -> CFRunLoopSourceRef;
}

#[link(name = "SystemConfiguration", kind = "framework")]
unsafe extern "C" {
    fn SCDynamicStoreCopyConsoleUser(store: *const c_void, uid: *mut u32, gid: *mut u32) -> CFStringRef;
}

/// UID of the user logged in at the console, or `None` at the login window.
pub fn console_uid() -> Option<u32> {
    let mut uid = 0;
    let name = unsafe { SCDynamicStoreCopyConsoleUser(std::ptr::null(), &mut uid, std::ptr::null_mut()) };
    if name.is_null() {
        return None;
    }
    let name = unsafe { CFString::wrap_under_create_rule(name) }.to_string();
    (name != "loginwindow" && uid != 0).then_some(uid)
}

extern "C" fn power_source_changed(context: *mut c_void) {
    let tx = unsafe { &*(context as *const Sender<()>) };
    let _ = tx.send(());
}

/// Sends `()` on `tx` each time the power source changes (adapter plugged in
/// or pulled, charge level change). Runs a CFRunLoop on a background thread.
pub fn on_power_source_change(tx: Sender<()>) {
    std::thread::spawn(move || {
        // The run loop never exits, so the sender lives for the whole process.
        let context = Box::into_raw(Box::new(tx)) as *mut c_void;
        let source = unsafe { IOPSNotificationCreateRunLoopSource(power_source_changed, context) };
        if source.is_null() {
            eprintln!("power source notifications unavailable; using the timer only");
            return;
        }
        let source = unsafe { CFRunLoopSource::wrap_under_create_rule(source) };
        CFRunLoop::get_current().add_source(&source, unsafe { kCFRunLoopDefaultMode });
        CFRunLoop::run_current();
    });
}

struct Service(IoObject);

impl Service {
    fn matching(class: &str) -> Option<Self> {
        let name = CString::new(class).ok()?;
        // IOServiceGetMatchingService consumes the matching dictionary.
        let service = unsafe {
            IOServiceGetMatchingService(MAIN_PORT_DEFAULT, IOServiceMatching(name.as_ptr()))
        };
        (service != 0).then_some(Service(service))
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        unsafe { IOObjectRelease(self.0) };
    }
}

/// All properties of the first service of `class`, as JSON.
pub fn properties(class: &str) -> Option<Map<String, Value>> {
    let service = Service::matching(class)?;
    let mut dict: CFMutableDictionaryRef = std::ptr::null_mut();
    let kr = unsafe {
        IORegistryEntryCreateCFProperties(service.0, &mut dict, kCFAllocatorDefault, 0)
    };
    if kr != 0 || dict.is_null() {
        return None;
    }
    let dict = unsafe { CFType::wrap_under_create_rule(dict as CFTypeRef) };
    match to_json(&dict) {
        Value::Object(map) => Some(map),
        _ => None,
    }
}

/// A single string property, e.g. IOPlatformUUID.
pub fn string_property(class: &str, key: &str) -> Option<String> {
    let service = Service::matching(class)?;
    let key = CFString::new(key);
    let value = unsafe {
        IORegistryEntryCreateCFProperty(service.0, key.as_concrete_TypeRef(), kCFAllocatorDefault, 0)
    };
    if value.is_null() {
        return None;
    }
    let value = unsafe { CFType::wrap_under_create_rule(value) };
    value.downcast::<CFString>().map(|s| s.to_string())
}

fn to_json(value: &CFType) -> Value {
    if let Some(b) = value.downcast::<CFBoolean>() {
        return Value::Bool(b.into());
    }
    if let Some(n) = value.downcast::<CFNumber>() {
        if unsafe { CFNumberIsFloatType(n.as_concrete_TypeRef()) } != 0 {
            return n.to_f64().map(Value::from).unwrap_or(Value::Null);
        }
        return n.to_i64().map(Value::from).unwrap_or(Value::Null);
    }
    if let Some(s) = value.downcast::<CFString>() {
        return Value::String(s.to_string());
    }
    let type_id = value.type_of();
    if type_id == CFArray::<CFType>::type_id() {
        let a: CFArray<CFType> = unsafe { CFArray::wrap_under_get_rule(value.as_CFTypeRef() as _) };
        return Value::Array(a.iter().map(|v| to_json(&v)).collect());
    }
    if type_id == CFDictionary::<CFType, CFType>::type_id() {
        let d: CFDictionary<CFType, CFType> =
            unsafe { CFDictionary::wrap_under_get_rule(value.as_CFTypeRef() as _) };
        let (keys, values) = d.get_keys_and_values();
        let mut map = Map::new();
        for (k, v) in keys.into_iter().zip(values) {
            let k = unsafe { CFType::wrap_under_get_rule(k) };
            let v = unsafe { CFType::wrap_under_get_rule(v) };
            if let Some(k) = k.downcast::<CFString>() {
                map.insert(k.to_string(), to_json(&v));
            }
        }
        return Value::Object(map);
    }
    if value.downcast::<CFData>().is_some() {
        // Raw blobs (e.g. firmware data) are not needed for the report.
        return Value::String("<data>".into());
    }
    Value::Null
}
