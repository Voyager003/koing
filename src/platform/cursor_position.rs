//! Accessibility API를 사용한 텍스트 커서(caret) 위치 감지

use std::ffi::c_void;
use std::ptr;

// AXUIElement 타입
type AXUIElementRef = *mut c_void;
type AXError = i32;
type AXValueRef = *mut c_void;
type CFTypeRef = *mut c_void;
type CFStringRef = *const c_void;

const K_AX_ERROR_SUCCESS: AXError = 0;
const K_AX_VALUE_TYPE_CG_POINT: u32 = 1;
const K_AX_VALUE_TYPE_CG_SIZE: u32 = 2;
const K_AX_VALUE_TYPE_CG_RECT: u32 = 4;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct CGSize {
    width: f64,
    height: f64,
}

extern "C" {
    fn AXUIElementCreateSystemWide() -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> AXError;
    fn AXUIElementCopyParameterizedAttributeValue(
        element: AXUIElementRef,
        attr: CFStringRef,
        param: CFTypeRef,
        result: *mut CFTypeRef,
    ) -> AXError;
    fn AXValueGetValue(value: AXValueRef, value_type: u32, value_ptr: *mut c_void) -> bool;
    fn CFRelease(cf: CFTypeRef);
}

// CoreFoundation string 생성
extern "C" {
    fn CFStringCreateWithCString(
        allocator: *const c_void,
        c_str: *const u8,
        encoding: u32,
    ) -> CFStringRef;
}

const K_CF_STRING_ENCODING_UTF8: u32 = 0x08000100;

/// CFString을 생성하는 헬퍼 (호출자가 CFRelease 해야 함)
unsafe fn cf_str(s: &str) -> CFStringRef {
    let mut bytes = s.as_bytes().to_vec();
    bytes.push(0);
    CFStringCreateWithCString(ptr::null(), bytes.as_ptr(), K_CF_STRING_ENCODING_UTF8)
}

/// AX 속성 값을 가져오는 헬퍼 (실패 시 None)
unsafe fn ax_get_attr(element: AXUIElementRef, attr_name: &str) -> Option<CFTypeRef> {
    let attr = cf_str(attr_name);
    let mut value: CFTypeRef = ptr::null_mut();
    let err = AXUIElementCopyAttributeValue(element, attr, &mut value);
    CFRelease(attr as CFTypeRef);
    if err == K_AX_ERROR_SUCCESS && !value.is_null() {
        Some(value)
    } else {
        None
    }
}

/// AX 파라미터화 속성 값을 가져오는 헬퍼
unsafe fn ax_get_param_attr(
    element: AXUIElementRef,
    attr_name: &str,
    param: CFTypeRef,
) -> Option<CFTypeRef> {
    let attr = cf_str(attr_name);
    let mut value: CFTypeRef = ptr::null_mut();
    let err = AXUIElementCopyParameterizedAttributeValue(element, attr, param, &mut value);
    CFRelease(attr as CFTypeRef);
    if err == K_AX_ERROR_SUCCESS && !value.is_null() {
        Some(value)
    } else {
        None
    }
}

/// AXValue에서 CGPoint를 추출
unsafe fn ax_value_to_point(value: CFTypeRef) -> Option<CGPoint> {
    let mut point = CGPoint::default();
    if AXValueGetValue(value, K_AX_VALUE_TYPE_CG_POINT, &mut point as *mut _ as *mut c_void) {
        Some(point)
    } else {
        None
    }
}

/// AXValue에서 CGSize를 추출
unsafe fn ax_value_to_size(value: CFTypeRef) -> Option<CGSize> {
    let mut size = CGSize::default();
    if AXValueGetValue(value, K_AX_VALUE_TYPE_CG_SIZE, &mut size as *mut _ as *mut c_void) {
        Some(size)
    } else {
        None
    }
}

/// AXValue에서 CGRect를 추출
unsafe fn ax_value_to_rect(value: CFTypeRef) -> Option<CGRect> {
    let mut rect = CGRect::default();
    if AXValueGetValue(value, K_AX_VALUE_TYPE_CG_RECT, &mut rect as *mut _ as *mut c_void) {
        Some(rect)
    } else {
        None
    }
}

/// AX API를 사용하여 현재 포커스된 텍스트 필드의 커서(caret) 위치를 가져옵니다.
/// 반환값: (x, y) 스크린 좌표 (좌상단 기준)
pub fn get_caret_position() -> Option<(f64, f64)> {
    unsafe {
        let system_wide = AXUIElementCreateSystemWide();
        if system_wide.is_null() {
            return None;
        }

        // AXFocusedUIElement를 시스템와이드에서 직접 가져오기
        let focused_element = ax_get_attr(system_wide, "AXFocusedUIElement");
        CFRelease(system_wide as CFTypeRef);

        let focused_element = focused_element?;

        // 방법 1: AXSelectedTextRange → AXBoundsForRange (정확한 커서 위치)
        let result = get_bounds_via_selected_range(focused_element as AXUIElementRef)
            // 방법 2: AXPosition + AXSize (텍스트 필드 위치 기반 폴백)
            .or_else(|| get_element_bottom_position(focused_element as AXUIElementRef));

        CFRelease(focused_element);
        result
    }
}

/// AXSelectedTextRange → AXBoundsForRange로 정확한 커서 위치를 가져옵니다.
unsafe fn get_bounds_via_selected_range(element: AXUIElementRef) -> Option<(f64, f64)> {
    let range_value = ax_get_attr(element, "AXSelectedTextRange")?;
    let bounds_value = ax_get_param_attr(element, "AXBoundsForRange", range_value);
    CFRelease(range_value);

    let bounds_value = bounds_value?;
    let rect = ax_value_to_rect(bounds_value);
    CFRelease(bounds_value);

    let rect = rect?;

    // 커서 위치: rect의 x, 하단 y
    Some((rect.origin.x + rect.size.width, rect.origin.y + rect.size.height))
}

/// 포커스된 요소의 AXPosition + AXSize로 하단 위치를 폴백으로 사용
unsafe fn get_element_bottom_position(element: AXUIElementRef) -> Option<(f64, f64)> {
    let pos_value = ax_get_attr(element, "AXPosition")?;
    let point = ax_value_to_point(pos_value);
    CFRelease(pos_value);
    let point = point?;

    let size_value = ax_get_attr(element, "AXSize");
    let height = if let Some(sv) = size_value {
        let size = ax_value_to_size(sv);
        CFRelease(sv);
        size.map(|s| s.height).unwrap_or(20.0)
    } else {
        20.0
    };

    // 요소의 왼쪽 하단 근처
    Some((point.x, point.y + height))
}

/// 마우스 커서 위치를 폴백으로 사용
pub fn get_mouse_position() -> (f64, f64) {
    extern "C" {
        fn CGEventCreate(source: *const c_void) -> *mut c_void;
        fn CGEventGetLocation(event: *const c_void) -> CGPoint;
    }

    unsafe {
        let event = CGEventCreate(ptr::null());
        if event.is_null() {
            return (0.0, 0.0);
        }

        let point = CGEventGetLocation(event);
        CFRelease(event);
        (point.x, point.y)
    }
}
