//! Transport-free editor host state machines shared by native and web hosts.

#[cfg(feature = "chat")]
pub mod chat;
#[cfg(feature = "codegen")]
pub mod codegen;
#[cfg(feature = "codegen")]
pub mod codegen_session;
#[cfg(feature = "design")]
pub mod design;
