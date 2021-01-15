use std::path::Path;

pub fn setup() {
    let active_state = "./active-state";
    let saved_states = "./saved-states";
    if Path::new(active_state).exists() {
        std::fs::remove_dir_all(active_state).expect("[!] Can't delete ./active-state");
    }

    if Path::new(saved_states).exists() {
        std::fs::remove_dir_all(saved_states).expect("[!] Can't delete ./saved-states");
    }

    std::fs::create_dir(active_state).expect("[!] Can't create ./active-state");

    std::fs::create_dir(saved_states).expect("[!] Can't create ./saved-states");
}

pub fn teardown() {
    let active_state = "./active-state";
    let saved_states = "./saved-states";
    let cmin_tmp = "./cmin-tmp";
    let generation_inputs = "./generation-inputs";

    match std::fs::remove_dir_all(active_state) {
        Ok(()) => (),
        Err(_) => println!("[!] No ./active-state to delete"),
    };

    match std::fs::remove_dir_all(saved_states) {
        Ok(()) => (),
        Err(_) => println!("[!] No ./saved-states to delete"),
    };

    match std::fs::remove_dir_all(cmin_tmp) {
        Ok(()) => (),
        Err(_) => println!("[!] No ./saved-states to delete"),
    };

    match std::fs::remove_dir_all(generation_inputs) {
        Ok(()) => (),
        Err(_) => println!("[!] No ./saved-states to delete"),
    };
}
