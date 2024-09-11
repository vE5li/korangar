use korangar_debug::logging::{print_debug, Colorize};
use wgpu::Error;

pub fn error_handler(error: Error) {
    let (message_type, message) = match error {
        Error::OutOfMemory { source } => ("OutOfMemory", source.to_string()),
        Error::Validation { source, description } => ("Validation", format!("{source}: {description}")),
        Error::Internal { source, description } => ("Internal", format!("{source}: {description}")),
    };

    print_debug!("wgpu [{}] [{}]: {}", message_type.yellow(), "error".red(), message);
}
