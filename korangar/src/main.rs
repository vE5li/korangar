#![allow(incomplete_features)]
#![allow(clippy::too_many_arguments)]
#![feature(adt_const_params)]
#![feature(allocator_api)]
#![feature(generic_const_exprs)]
#![feature(iter_next_chunk)]
#![feature(negative_impls)]
#![feature(proc_macro_hygiene)]
#![feature(random)]
#![feature(type_changing_struct_update)]
#![feature(unsized_const_params)]
#![feature(variant_count)]
#![feature(anonymous_lifetime_in_impl_trait)]
#![feature(associated_type_defaults)]
#![feature(macro_metavar_expr)]
#![feature(unsafe_cell_access)]
#![feature(impl_trait_in_assoc_type)]
#![feature(thread_local)]

use std::sync::atomic::Ordering;

#[cfg(feature = "debug")]
use korangar::threads;
use korangar::{Client, SHUTDOWN_SIGNAL, init_tls_rand, time_phase};
#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};

fn initialize_shutdown_signal() {
    ctrlc::set_handler(|| {
        println!("CTRL-C received. Shutting down");
        SHUTDOWN_SIGNAL.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");
}

fn main() {
    // We start a frame so that functions trying to start a measurement don't panic.
    #[cfg(feature = "debug")]
    let _measurement = threads::Main::start_frame();

    initialize_shutdown_signal();

    time_phase!("create global thread pool", {
        rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .start_handler(|_| init_tls_rand())
            .build_global()
            .unwrap();
    });

    time_phase!("seed main random instance", {
        init_tls_rand();
    });

    // Check if korangar is in the correct working directory and if not, try to
    // correct it.
    // NOTE: This check might be temporary or feature gated in the future.
    time_phase!("adjust working directory", {
        if !std::fs::metadata("archive").is_ok_and(|metadata| metadata.is_dir()) {
            #[cfg(feature = "debug")]
            print_debug!(
                "[{}] failed to find archive directory, attempting to change working directory {}",
                "warning".yellow(),
                "korangar".magenta()
            );

            if let Err(_error) = std::env::set_current_dir("korangar") {
                #[cfg(feature = "debug")]
                print_debug!("[{}] failed to change working directory: {:?}", "error".red(), _error);
            }
        }
    });

    let args: Vec<String> = std::env::args().collect();
    let sync_cache = args.len() > 1 && &args[1] == "sync-cache";

    let Some(mut client) = Client::init(sync_cache) else {
        return;
    };

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let _ = event_loop.run_app(&mut client);
}
