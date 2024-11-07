mod client;
pub mod context;
pub mod network;

#[cfg(any(feature="cpp", feature="wasm"))]
use context::{Config, Context};
#[cfg(any(feature="cpp", feature="wasm"))]
use std::{ffi::{c_char, c_uchar, CStr}, os::raw::c_void, slice};

#[cfg(target_family="wasm")]
use wasm_bindgen::prelude::{JsValue, wasm_bindgen};
#[cfg(target_family="wasm")]
use wasm_bindgen_futures::{future_to_promise, js_sys::{Error, Promise}};

// ******************************************
// ** posemesh_networking_context_create() **
// ******************************************

#[cfg(any(feature="cpp", feature="wasm"))]
fn posemesh_networking_context_create(config: &Config) -> *mut Context {
    match Context::new(config) {
        Ok(context) => Box::into_raw(context),
        Err(error) => {
            eprintln!("posemesh_networking_context_create(): {:?}", error);
            std::ptr::null_mut()
        }
    }
}

#[cfg(feature="cpp")]
#[no_mangle]
pub extern "C" fn psm_posemesh_networking_context_create(config: *const Config) -> *mut Context {
    assert!(!config.is_null(), "psm_posemesh_networking_context_create(): config is null");
    posemesh_networking_context_create(unsafe { &*config })
}

#[cfg(feature="wasm")]
#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn posemeshNetworkingContextCreate(config: &Config) -> *mut Context {
    posemesh_networking_context_create(config)
}

// *******************************************
// ** posemesh_networking_context_destroy() **
// *******************************************

#[cfg(any(feature="cpp", feature="wasm"))]
fn posemesh_networking_context_destroy(context: *mut Context) {
    assert!(!context.is_null(), "posemesh_networking_context_destroy(): context is null");
    unsafe {
        let _ = Box::from_raw(context);
    }
}

#[cfg(feature="cpp")]
#[no_mangle]
pub extern "C" fn psm_posemesh_networking_context_destroy(context: *mut Context) {
    posemesh_networking_context_destroy(context);
}

#[cfg(feature="wasm")]
#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn posemeshNetworkingContextDestroy(context: *mut Context) {
    posemesh_networking_context_destroy(context);
}

// ************************************************
// ** posemesh_networking_context_send_message() **
// ************************************************

#[cfg(feature="cpp")]
type SendMessageReturnType = u8;
#[cfg(feature="cpp")]
type SendMessageString = *const c_char;

#[cfg(feature="wasm")]
type SendMessageReturnType = Promise;
#[cfg(feature="wasm")]
type SendMessageString = String;

#[cfg(any(feature="cpp", feature="wasm"))]
fn posemesh_networking_context_send_message(
    context: *mut Context,
    #[cfg(feature="cpp")]
    message: *const c_void,
    #[cfg(feature="wasm")]
    message: Vec<u8>,
    #[cfg(feature="cpp")]
    message_size: u32,
    peer_id: SendMessageString,
    protocol: SendMessageString,
    #[cfg(feature="cpp")]
    callback: extern "C" fn(status: u8)
) -> SendMessageReturnType {
    let context = unsafe {
        assert!(!context.is_null(), "posemesh_networking_context_send_message(): context is null");
        &mut *context
    };

    #[cfg(feature="cpp")]
    let message = unsafe {
        assert!(!message.is_null(), "posemesh_networking_context_send_message(): message is null");
        assert!(message_size != 0, "posemesh_networking_context_send_message(): message_size is zero");
        slice::from_raw_parts(message as *const c_uchar, message_size as usize)
    }.to_vec();

    #[cfg(feature="cpp")]
    let peer_id = match unsafe {
        assert!(!peer_id.is_null(), "posemesh_networking_context_send_message(): peer_id is null");
        CStr::from_ptr(peer_id)
    }.to_str() {
        Ok(peer_id) => peer_id,
        Err(error) => {
            eprintln!("posemesh_networking_context_send_message(): {:?}", error);
            return 0;
        }
    }.to_string();

    #[cfg(feature="cpp")]
    let protocol = match unsafe {
        assert!(!protocol.is_null(), "posemesh_networking_context_send_message(): protocol is null");
        CStr::from_ptr(protocol)
    }.to_str() {
        Ok(protocol) => protocol,
        Err(error) => {
            eprintln!("posemesh_networking_context_send_message(): {:?}", error);
            return 0;
        }
    }.to_string();

    #[cfg(feature="wasm")]
    return future_to_promise(async move {
        match context.send(message, peer_id, protocol).await {
            Ok(_) => { Ok(JsValue::from(true)) },
            Err(error) => {
                eprintln!("posemesh_networking_context_send_message(): {:?}", error);
                Err(JsValue::from(Error::new(error.to_string().as_str())))
            }
        }
    });

    #[cfg(feature="cpp")]
    {
        context.send_with_callback(message, peer_id, protocol, callback);
        return 1;
    }
}

#[cfg(feature="cpp")]
#[no_mangle]
pub extern "C" fn psm_posemesh_networking_context_send_message(
    context: *mut Context,
    message: *const c_void,
    message_size: u32,
    peer_id: *const c_char,
    protocol: *const c_char,
    callback: extern "C" fn(status: u8)
) -> u8 {
    posemesh_networking_context_send_message(context, message, message_size, peer_id, protocol, callback)
}

#[cfg(feature="wasm")]
#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn posemeshNetworkingContextSendMessage(context: *mut Context, message: Vec<u8>, peer_id: String, protocol: String) -> Promise {
    posemesh_networking_context_send_message(context, message, peer_id, protocol)
}
