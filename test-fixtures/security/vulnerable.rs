// INTENTIONAL VULNERABILITIES - DO NOT USE IN PRODUCTION
// Test fixture for security scanner validation

// RUST-001: Unsafe blocks without proper documentation
fn process_data(ptr: *const u8, len: usize) {
    unsafe {  // BAD: No SAFETY comment
        let slice = std::slice::from_raw_parts(ptr, len);
        println!("{:?}", slice);
    }
}

// RUST-002: Unchecked unwrap on fallible operations
fn parse_config(data: &str) -> i32 {
    let value = data.parse::<i32>().unwrap();  // BAD: May panic
    let other = Some(42).expect("hardcoded");   // BAD: May panic
    value + other
}

// RUST-003: Raw pointer operations
fn raw_pointer_usage() {
    let x = 42;
    let ptr = &x as *const i32;           // BAD: Raw pointer cast
    let raw = x.to_string().as_ptr();     // BAD: as_ptr() call
    unsafe {
        std::ptr::read(ptr);               // BAD: ptr::read
        std::ptr::write(ptr as *mut i32, 0); // BAD: ptr::write
    }
}

// SAFE PATTERNS (should not trigger or have reduced severity)
fn safe_code_example() {
    // SAFETY: ptr is guaranteed valid by caller contract
    unsafe {
        // documented unsafe block
    }

    // Using proper error handling
    let value = "42".parse::<i32>().unwrap_or(0);
    let other = Some(42).unwrap_or_default();
    let result = "123".parse::<i32>()?;  // Would need Result return type

    // Using NonNull for safer pointer handling
    let x = 42;
    if let Some(ptr) = std::ptr::NonNull::new(&x as *const _ as *mut i32) {
        // Safe: using NonNull abstraction
    }
}
