pub mod descriptor;
pub mod transport;
pub mod adapter;
pub mod coordinator;
pub mod persistence;
pub mod window_close;

pub use descriptor::{
    AppAvailability, IntegrationCapability, MicroAppDescriptor, MicroAppRegistry, RuntimeKind,
};
pub use transport::{CraftHostTransport, RawBridgeMessage, TransportError, validate_origin, validate_structure};
pub use adapter::{DocumentStateEvent, GridCraftAdapter, MicroAppAdapter, PdfCraftAdapter};
pub use coordinator::{
    CoordinatorAction, DocumentState, HostDocumentCoordinator, UserClosingDecision,
};
pub use persistence::{
    atomic_durable_write, atomic_durable_write_with_ticket, compute_fingerprint,
    execute_durable_save, validate_destination_path, DestinationSaveTracker, DestinationState,
    PersistenceError, PersistenceReport, WriteTicket, GLOBAL_SAVE_TRACKER,
};
pub use window_close::{WindowCloseDecision, WindowCloseManager};
#[cfg(target_arch = "wasm32")]
pub use window_close::register_window_close_listener;
