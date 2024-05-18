pub fn set_panic_hook(process_name: String) {
    std::panic::set_hook(Box::new(move |panic_info| {
        eprintln!("\nPanic in process: {process_name}");
        if let Some(location) = panic_info.location() {
            eprintln!("panic occurred in file '{}' at line {}", location.file(), location.line());
        } else {
            eprintln!("panic occurred but can't get location information...");
        }
        if let Some(payload) = panic_info.payload().downcast_ref::<&str>() {
            eprintln!("with message: {payload}");
        }

        println!("full is: {:?}", panic_info.to_string());
        std::process::exit(1);
    }));
}