pub mod cursor_position;
pub mod event_tap;
pub mod input_source;
pub mod os_version;
pub mod permissions;
pub mod text_replacer;

/// GCD를 사용하여 클로저를 메인 스레드에서 실행합니다.
pub fn dispatch_to_main<F: FnOnce() + Send + 'static>(f: F) {
    // dispatch_get_main_queue()는 C 매크로이므로, 실제 심볼인 _dispatch_main_q를 사용
    extern "C" {
        static _dispatch_main_q: std::ffi::c_void;
        fn dispatch_async_f(
            queue: *const std::ffi::c_void,
            context: *mut std::ffi::c_void,
            work: extern "C" fn(*mut std::ffi::c_void),
        );
    }

    extern "C" fn trampoline<F: FnOnce()>(context: *mut std::ffi::c_void) {
        unsafe {
            let f = Box::from_raw(context as *mut F);
            f();
        }
    }

    let boxed = Box::new(f);
    let raw = Box::into_raw(boxed) as *mut std::ffi::c_void;

    unsafe {
        let main_queue = &_dispatch_main_q as *const std::ffi::c_void;
        dispatch_async_f(main_queue, raw, trampoline::<F>);
    }
}
