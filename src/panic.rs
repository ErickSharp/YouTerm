pub fn set_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        println!("{}", panic_info);
        std::process::exit(0); // Exit cleanly for asthetic reasons
    }));
}